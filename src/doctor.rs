use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::constants::get_tunnel_config_path;
#[cfg(unix)]
use crate::constants::{get_portctl_config_dir, get_registry_dir};
use crate::probe::probe_port;
use crate::registry::PortRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticIssue {
    pub category: String,
    pub severity: String, // "ERROR", "WARNING", "INFO"
    pub description: String,
    pub fixable: bool,
    pub fixed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub issues: Vec<DiagnosticIssue>,
    pub total_issues: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub fixed_count: usize,
    pub is_healthy: bool,
}

pub struct DoctorEngine<'a> {
    registry: &'a PortRegistry,
}

impl<'a> DoctorEngine<'a> {
    pub fn new(registry: &'a PortRegistry) -> Self {
        Self { registry }
    }

    pub fn diagnose(&self, auto_fix: bool) -> DoctorReport {
        let mut issues = Vec::new();

        // 1. Audit Permissions
        self.check_permissions(&mut issues, auto_fix);

        // 2. Audit Rogue ports in managed range (4000-4050)
        self.check_rogue_ports(&mut issues);

        // 3. Audit Ghost entries
        self.check_ghost_entries(&mut issues);

        // 4. Audit Dangling Cloudflare Ingress rules
        self.check_dangling_tunnels(&mut issues);

        // 5. Audit Port Hijacks
        self.check_port_hijacks(&mut issues);

        let error_count = issues.iter().filter(|i| i.severity == "ERROR").count();
        let warning_count = issues.iter().filter(|i| i.severity == "WARNING").count();
        let fixed_count = issues.iter().filter(|i| i.fixed).count();
        let total_issues = issues.len();
        let is_healthy = error_count == 0;

        // If auto_fix, resync ports.json
        if auto_fix {
            let _ = self.registry.sync_ports_json();
        }

        DoctorReport {
            issues,
            total_issues,
            error_count,
            warning_count,
            fixed_count,
            is_healthy,
        }
    }

    #[cfg(unix)]
    fn check_permissions(&self, issues: &mut Vec<DiagnosticIssue>, auto_fix: bool) {
        let config_dir = get_portctl_config_dir();
        if config_dir.exists() {
            if let Ok(meta) = fs::metadata(&config_dir) {
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o700 {
                    let mut fixed = false;
                    if auto_fix {
                        let _ = fs::set_permissions(&config_dir, fs::Permissions::from_mode(0o700));
                        fixed = true;
                    }
                    issues.push(DiagnosticIssue {
                        category: "Permissions".to_string(),
                        severity: "WARNING".to_string(),
                        description: format!(
                            "Directory {:?} permissions are 0{:o} (recommended: 0700).",
                            config_dir, mode
                        ),
                        fixable: true,
                        fixed,
                    });
                }
            }
        }

        let registry_dir = get_registry_dir();
        if registry_dir.exists() {
            if let Ok(entries) = fs::read_dir(&registry_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(meta) = fs::metadata(&path) {
                            let mode = meta.permissions().mode() & 0o777;
                            if mode != 0o600 {
                                let mut fixed = false;
                                if auto_fix {
                                    let _ = fs::set_permissions(
                                        &path,
                                        fs::Permissions::from_mode(0o600),
                                    );
                                    fixed = true;
                                }
                                issues.push(DiagnosticIssue {
                                    category: "Permissions".to_string(),
                                    severity: "WARNING".to_string(),
                                    description: format!(
                                        "File {:?} permissions are 0{:o} (recommended: 0600).",
                                        path.file_name().unwrap_or_default(),
                                        mode
                                    ),
                                    fixable: true,
                                    fixed,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn check_permissions(&self, _issues: &mut Vec<DiagnosticIssue>, _auto_fix: bool) {
        // POSIX permission modes are not applicable on Windows
    }

    fn check_rogue_ports(&self, issues: &mut Vec<DiagnosticIssue>) {
        // Inspect range 4000..=4050
        for p in 4000..=4050 {
            if self.registry.find_by_port(p).is_none() {
                let probe = probe_port(p);
                if probe.is_listening {
                    issues.push(DiagnosticIssue {
                        category: "Rogue Port".to_string(),
                        severity: "WARNING".to_string(),
                        description: format!(
                            "Port {} is listening (PID {:?}, cmd {:?}) but is not registered in portctl.",
                            p,
                            probe.pid.unwrap_or_else(|| "?".to_string()),
                            probe.command.unwrap_or_else(|| "?".to_string())
                        ),
                        fixable: false,
                        fixed: false,
                    });
                }
            }
        }
    }

    fn check_ghost_entries(&self, issues: &mut Vec<DiagnosticIssue>) {
        let canonical = self.registry.load_canonical_apps();
        for (name, item) in canonical.apps {
            if let Some(ref repo) = item.repo {
                if repo.starts_with('/') && !Path::new(repo).exists() {
                    issues.push(DiagnosticIssue {
                        category: "Ghost Entry".to_string(),
                        severity: "INFO".to_string(),
                        description: format!(
                            "App '{}' (port {}) references a local directory that no longer exists: '{}'.",
                            name, item.port, repo
                        ),
                        fixable: false,
                        fixed: false,
                    });
                }
            }
        }
    }

    fn check_dangling_tunnels(&self, issues: &mut Vec<DiagnosticIssue>) {
        let tunnel_config_path = get_tunnel_config_path();
        if !tunnel_config_path.exists() {
            return;
        }

        if let Ok(raw_yaml) = fs::read_to_string(&tunnel_config_path) {
            if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&raw_yaml) {
                if let Some(ingress_seq) = doc.get("ingress").and_then(|v| v.as_sequence()) {
                    for rule in ingress_seq {
                        if let Some(svc) = rule.get("service").and_then(|v| v.as_str()) {
                            if let Some(port_str) = svc.strip_prefix("http://localhost:") {
                                if let Ok(port) = port_str.parse::<u16>() {
                                    if self.registry.find_by_port(port).is_none() {
                                        let hostname = rule
                                            .get("hostname")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        issues.push(DiagnosticIssue {
                                            category: "Dangling Ingress".to_string(),
                                            severity: "WARNING".to_string(),
                                            description: format!(
                                                "Cloudflare Tunnel ingress for '{}' points to port {}, which is not registered in portctl.",
                                                hostname, port
                                            ),
                                            fixable: false,
                                            fixed: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_port_hijacks(&self, issues: &mut Vec<DiagnosticIssue>) {
        let all = self.registry.get_all_merged();
        for item in all {
            let probe = probe_port(item.port);
            if probe.is_listening {
                if let Some(ref cmd) = probe.command {
                    // Check if process command is an obvious completely mismatched program
                    let cmd_lower = cmd.to_lowercase();
                    if cmd_lower.contains("docker-proxy")
                        && !item.name.contains("docker")
                        && !item.name.contains("postgres")
                        && !item.name.contains("redis")
                    {
                        issues.push(DiagnosticIssue {
                            category: "Potential Hijack".to_string(),
                            severity: "WARNING".to_string(),
                            description: format!(
                                "Port {} (assigned to '{}') is listening under docker-proxy (PID {:?}).",
                                item.port, item.name, probe.pid.unwrap_or_else(|| "?".to_string())
                            ),
                            fixable: false,
                            fixed: false,
                        });
                    }
                }
            }
        }
    }
}
