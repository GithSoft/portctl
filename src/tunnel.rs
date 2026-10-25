use serde_yaml::{Mapping, Value};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use crate::constants::{
    get_sensitive_ports, get_system_tunnel_config_path, get_tunnel_config_path,
};
use crate::registry::PortRegistry;

#[derive(Debug, Clone)]
pub struct ExposeResult {
    pub url: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct UnexposeResult {
    pub app_name: String,
    pub domain: Option<String>,
    pub port: u16,
    pub rules_removed: usize,
}

pub trait TunnelProvider {
    fn id(&self) -> &'static str;
    fn expose(&self, app_name: &str, domain: &str, force: bool) -> Result<ExposeResult, String>;
    fn unexpose(&self, app_name: &str, domain_opt: Option<&str>) -> Result<UnexposeResult, String>;
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

pub struct TunnelManager<'a> {
    registry: &'a PortRegistry,
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
        if !root_map.contains_key(&ingress_key) {
            root_map.insert(ingress_key.clone(), Value::Sequence(Vec::new()));
        }

        let ingress_seq = root_map
            .get_mut(&ingress_key)
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| "'ingress' in tunnel config must be a YAML sequence".to_string())?;

        // Format target service URL
        let service_url = format!("http://localhost:{}", port);

        // Check if rule already exists
        let mut found = false;
        for rule in ingress_seq.iter_mut() {
            if let Some(map) = rule.as_mapping_mut() {
                if let Some(h) = map
                    .get(Value::String("hostname".to_string()))
                    .and_then(|v| v.as_str())
                {
                    if h == domain {
                        map.insert(
                            Value::String("service".to_string()),
                            Value::String(service_url.clone()),
                        );
                        found = true;
                        break;
                    }
                }
            }
        }

        if !found {
            let mut new_rule = Mapping::new();
            new_rule.insert(
                Value::String("hostname".to_string()),
                Value::String(domain.to_string()),
            );
            new_rule.insert(
                Value::String("service".to_string()),
                Value::String(service_url),
            );

            // Insert before catch-all 404 rule if present
            if let Some(last_pos) = ingress_seq.iter().rposition(|r| {
                if let Some(map) = r.as_mapping() {
                    if let Some(s) = map
                        .get(Value::String("service".to_string()))
                        .and_then(|v| v.as_str())
                    {
                        return s.contains("404");
                    }
                }
                false
            }) {
                ingress_seq.insert(last_pos, Value::Mapping(new_rule));
            } else {
                ingress_seq.push(Value::Mapping(new_rule));
            }
        }

        // Ensure catch-all 404 rule exists at the end
        let has_404 = ingress_seq.iter().any(|r| {
            if let Some(map) = r.as_mapping() {
                if let Some(s) = map
                    .get(Value::String("service".to_string()))
                    .and_then(|v| v.as_str())
                {
                    return s.contains("404");
                }
            }
            false
        });

        if !has_404 {
            let mut catch_all = Mapping::new();
            catch_all.insert(
                Value::String("service".to_string()),
                Value::String("http_status:404".to_string()),
            );
            ingress_seq.push(Value::Mapping(catch_all));
        }

        let updated_yaml = serde_yaml::to_string(&doc)
            .map_err(|e| format!("Failed to serialize updated tunnel YAML: {}", e))?;

        // 5. Pre-flight Sandboxed Ingress Validation
        let mut temp_file = tempfile::Builder::new()
            .prefix(".cloudflared-validate-")
            .suffix(".yml")
            .tempfile()
            .map_err(|e| format!("Failed to create temporary validation file: {}", e))?;

        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(temp_file.path(), permissions);
        }

        temp_file
            .write_all(updated_yaml.as_bytes())
            .map_err(|e| format!("Failed to write to validation file: {}", e))?;
        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush validation file: {}", e))?;

        let validate_status = Command::new("cloudflared")
            .args([
                "tunnel",
                "--config",
                temp_file.path().to_str().unwrap_or_default(),
                "ingress",
                "validate",
            ])
            .output();

        match validate_status {
            Ok(output) if !output.status.success() => {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "Pre-flight ingress validation failed with error:\n{}",
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

        // Copy to system config if appropriate
        let system_config = get_system_tunnel_config_path();
        if Path::new(system_config).exists() {
            let _ = Command::new("sudo")
                .args(["cp", &tunnel_config_path.to_string_lossy(), system_config])
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

    pub fn unexpose(
        &self,
        app_name: &str,
        domain_opt: Option<&str>,
    ) -> Result<UnexposeResult, String> {
        let entry = self.registry.find_by_name(app_name).ok_or_else(|| {
            format!(
                "Application '{}' is not registered in portctl registry.",
                app_name
            )
        })?;

        let port = entry.port;
        let service_url = format!("http://localhost:{}", port);
        let target_domain = domain_opt.or(entry.tunnel.as_deref());

        let tunnel_config_path = get_tunnel_config_path();
        if !tunnel_config_path.exists() {
            return Err(format!(
                "Cloudflare Tunnel config file not found at: {:?}",
                tunnel_config_path
            ));
        }

        let raw_yaml = fs::read_to_string(&tunnel_config_path)
            .map_err(|e| format!("Failed to read tunnel config: {}", e))?;

        let mut doc: Value =
            serde_yaml::from_str(&raw_yaml).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let root_map = doc
            .as_mapping_mut()
            .ok_or_else(|| "Tunnel config root must be a YAML mapping".to_string())?;

        let ingress_key = Value::String("ingress".to_string());
        let ingress_seq = root_map
            .get_mut(&ingress_key)
            .and_then(|v| v.as_sequence_mut())
            .ok_or_else(|| "No ingress configuration found in tunnel config".to_string())?;

        let initial_len = ingress_seq.len();

        // Retain rules that do NOT match target service or target domain
        ingress_seq.retain(|rule| {
            if let Some(map) = rule.as_mapping() {
                // Never remove 404 rule
                if let Some(s) = map
                    .get(Value::String("service".to_string()))
                    .and_then(|v| v.as_str())
                {
                    if s.contains("404") {
                        return true;
                    }
                    if s == service_url {
                        return false;
                    }
                }
                if let Some(d) = target_domain {
                    if let Some(h) = map
                        .get(Value::String("hostname".to_string()))
                        .and_then(|v| v.as_str())
                    {
                        if h == d {
                            return false;
                        }
                    }
                }
            }
            true
        });

        let removed = initial_len - ingress_seq.len();

        let updated_yaml = serde_yaml::to_string(&doc)
            .map_err(|e| format!("Failed to serialize updated tunnel YAML: {}", e))?;

        // Pre-flight validation
        let mut temp_file = tempfile::Builder::new()
            .prefix(".cloudflared-unexpose-")
            .suffix(".yml")
            .tempfile()
            .map_err(|e| format!("Failed to create temporary validation file: {}", e))?;

        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(temp_file.path(), permissions);
        }

        temp_file
            .write_all(updated_yaml.as_bytes())
            .map_err(|e| format!("Failed to write to validation file: {}", e))?;
        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush validation file: {}", e))?;

        let validate_status = Command::new("cloudflared")
            .args([
                "tunnel",
                "--config",
                temp_file.path().to_str().unwrap_or_default(),
                "ingress",
                "validate",
            ])
            .output();

        match validate_status {
            Ok(output) if !output.status.success() => {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "Pre-flight ingress validation failed with error:\n{}",
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

        // Write updated config
        fs::write(&tunnel_config_path, &updated_yaml)
            .map_err(|e| format!("Failed to write updated tunnel config: {}", e))?;

        let system_config = get_system_tunnel_config_path();
        if Path::new(system_config).exists() {
            let _ = Command::new("sudo")
                .args(["cp", &tunnel_config_path.to_string_lossy(), system_config])
                .output();
        }

        // Kickstart daemon
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

        // Clear tunnel domain in registry
        let mut canonical = self.registry.load_canonical_apps();
        if let Some(app) = canonical.apps.get_mut(app_name) {
            app.tunnel = None;
            let _ = self.registry.save_canonical_apps(&canonical);
        }

        Ok(UnexposeResult {
            app_name: app_name.to_string(),
            domain: target_domain.map(|s| s.to_string()),
            port,
            rules_removed: removed,
        })
    }
}

impl<'a> TunnelProvider for TunnelManager<'a> {
    fn id(&self) -> &'static str {
        "cloudflare"
    }

    fn expose(&self, app_name: &str, domain: &str, force: bool) -> Result<ExposeResult, String> {
        self.expose(app_name, domain, force)
    }

    fn unexpose(&self, app_name: &str, domain_opt: Option<&str>) -> Result<UnexposeResult, String> {
        self.unexpose(app_name, domain_opt)
    }
}
