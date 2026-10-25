use serde_yaml::{Mapping, Value};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use crate::constants::{
    get_sensitive_ports, get_system_tunnel_config_path, get_tunnel_config_path,
};
use crate::registry::PortRegistry;

pub struct ExposeResult {
    pub url: String,
    pub port: u16,
}

pub struct TunnelManager<'a> {
    registry: &'a PortRegistry,
}

pub fn is_valid_rfc1123_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 || !domain.contains('.') {
        return false;
    }
    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
    }
    true
}

impl<'a> TunnelManager<'a> {
    pub fn new(registry: &'a PortRegistry) -> Self {
        Self { registry }
    }

    pub fn expose(
        &self,
        app_name: &str,
        domain: &str,
        force: bool,
    ) -> Result<ExposeResult, String> {
        // 1. Validate RFC 1123 domain
        if !is_valid_rfc1123_domain(domain) {
            return Err(format!(
                "Domain '{}' is invalid according to RFC 1123.",
                domain
            ));
        }

        // 2. Fetch port entry from registry
        let entry = self.registry.find_by_name(app_name).ok_or_else(|| {
            format!(
                "Application '{}' is not registered. Run 'portctl assign' first.",
                app_name
            )
        })?;

        let port = entry.port;

        // 3. Security guardrail: blacklist check
        let sensitive = get_sensitive_ports();
        if sensitive.contains(&port) && !force {
            return Err(format!(
                "[SECURITY GUARDRAIL] Port {} is in the sensitive/database/debug blacklist and cannot be exposed to the Internet! Use --force if you intentionally wish to proceed.",
                port
            ));
        }

        let tunnel_config_path = get_tunnel_config_path();
        if !tunnel_config_path.exists() {
            return Err(format!(
                "Cloudflare Tunnel config file not found at: {:?}",
                tunnel_config_path
            ));
        }

        // 4. Parse YAML
        let raw_yaml = fs::read_to_string(&tunnel_config_path)
            .map_err(|e| format!("Failed to read tunnel config: {}", e))?;

        let mut doc: Value =
            serde_yaml::from_str(&raw_yaml).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let root_map = doc
            .as_mapping_mut()
            .ok_or_else(|| "Tunnel config root must be a YAML mapping".to_string())?;

        let tunnel_id = root_map
            .get(Value::String("tunnel".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let ingress_key = Value::String("ingress".to_string());
        let ingress_val = root_map
            .entry(ingress_key)
            .or_insert_with(|| Value::Sequence(Vec::new()));

        let ingress_seq = ingress_val
            .as_sequence_mut()
            .ok_or_else(|| "'ingress' in tunnel config must be a list".to_string())?;

        // Filter out existing rule for this domain
        ingress_seq.retain(|item| {
            if let Some(map) = item.as_mapping() {
                if let Some(host) = map
                    .get(Value::String("hostname".to_string()))
                    .and_then(|h| h.as_str())
                {
                    return host != domain;
                }
            }
            true
        });

        // Create new ingress rule
        let mut new_rule_map = Mapping::new();
        new_rule_map.insert(
            Value::String("hostname".to_string()),
            Value::String(domain.to_string()),
        );
        new_rule_map.insert(
            Value::String("service".to_string()),
            Value::String(format!("http://localhost:{}", port)),
        );
        let new_rule = Value::Mapping(new_rule_map);

        // Find catch-all index
        let catch_all_idx = ingress_seq.iter().position(|item| {
            if let Some(map) = item.as_mapping() {
                let has_host = map.contains_key(Value::String("hostname".to_string()));
                let has_service = map.contains_key(Value::String("service".to_string()));
                return !has_host && has_service;
            }
            false
        });

        if let Some(idx) = catch_all_idx {
            ingress_seq.insert(idx, new_rule);
        } else {
            ingress_seq.push(new_rule);
            let mut catch_all_map = Mapping::new();
            catch_all_map.insert(
                Value::String("service".to_string()),
                Value::String("http_status:404".to_string()),
            );
            ingress_seq.push(Value::Mapping(catch_all_map));
        }

        let updated_yaml =
            serde_yaml::to_string(&doc).map_err(|e| format!("Failed to serialize YAML: {}", e))?;

        // 5. Temporary file validation (CWE-377 mitigation)
        let parent = tunnel_config_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let mut temp_file = tempfile::Builder::new()
            .prefix(".tunnel-validate-")
            .tempfile_in(parent)
            .map_err(|e| format!("Failed to create temporary validation file: {}", e))?;

        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(temp_file.path(), permissions);
        }

        temp_file
            .write_all(updated_yaml.as_bytes())
            .map_err(|e| format!("Failed to write validation YAML: {}", e))?;
        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush validation YAML: {}", e))?;

        // Validate with cloudflared
        let val_status = Command::new("cloudflared")
            .args([
                "tunnel",
                "--config",
                temp_file.path().to_str().unwrap(),
                "ingress",
                "validate",
            ])
            .output();

        match val_status {
            Ok(output) if !output.status.success() => {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "Cloudflare ingress validation failed:\n{}",
                    err_msg
                ));
            }
            Err(e) => {
                eprintln!(
                    "Warning: could not execute 'cloudflared' for validation: {}",
                    e
                );
            }
            _ => {}
        }

        // 6. Write to local tunnel config
        fs::write(&tunnel_config_path, &updated_yaml)
            .map_err(|e| format!("Failed to write updated tunnel config: {}", e))?;

        // Copy to /etc/cloudflared/config.yml if appropriate
        let system_config = get_system_tunnel_config_path();
        if Path::new(system_config).exists() {
            let _ = Command::new("sudo")
                .args(["cp", tunnel_config_path.to_str().unwrap(), system_config])
                .output();
        }

        // 7. Route DNS
        let _ = Command::new("cloudflared")
            .args(["tunnel", "route", "dns", &tunnel_id, domain])
            .output();

        // 8. Kickstart daemon
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("sudo")
                .args([
                    "launchctl",
                    "kickstart",
                    "-k",
                    "system/com.cloudflare.cloudflared",
                ])
                .output();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("sudo")
                .args(["systemctl", "restart", "cloudflared"])
                .output();
        }

        // 9. Update registry canonical app tunnel metadata
        let mut canonical = self.registry.load_canonical_apps();
        if let Some(app) = canonical.apps.get_mut(app_name) {
            app.tunnel = Some(domain.to_string());
            let _ = self.registry.save_canonical_apps(&canonical);
        }

        Ok(ExposeResult {
            url: format!("https://{}", domain),
            port,
        })
    }
}
