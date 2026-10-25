use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::constants::{get_portctl_config_dir, get_registry_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedServiceItem {
    pub port: u16,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tunnel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FixedServicesFile {
    #[serde(default)]
    pub services: HashMap<String, FixedServiceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalAppItem {
    pub port: u16,
    #[serde(default)]
    pub tunnel: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalAppsFile {
    #[serde(default)]
    pub apps: HashMap<String, CanonicalAppItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineAllocationItem {
    pub port: u16,
    #[serde(default)]
    pub tunnel: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MachineAllocationFile {
    pub node_id: String,
    #[serde(default)]
    pub allocated: HashMap<String, MachineAllocationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedPortItem {
    pub name: String,
    pub port: u16,
    pub scope: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub tunnel: Option<String>,
    pub repo: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPortItem {
    pub name: String,
    pub port: u16,
    pub github: String,
    pub tunnel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPortsFile {
    pub version: String,
    #[serde(default)]
    pub last_updated: Option<String>,
    pub ports: Vec<LegacyPortItem>,
}

pub fn atomic_write_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
    }

    let mut temp_file = tempfile::Builder::new()
        .prefix(".portctl-tmp-")
        .tempfile_in(parent)
        .map_err(|e| format!("Failed to create temporary file in {:?}: {}", parent, e))?;

    // Set 0600 permissions
    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(temp_file.path(), permissions);
    }

    let json_bytes =
        serde_json::to_vec_pretty(data).map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    temp_file
        .write_all(&json_bytes)
        .map_err(|e| format!("Failed to write to temporary file: {}", e))?;
    temp_file
        .flush()
        .map_err(|e| format!("Failed to flush temporary file: {}", e))?;

    temp_file
        .persist(path)
        .map_err(|e| format!("Failed to persist file to {:?}: {}", path, e))?;

    Ok(())
}

pub struct PortRegistry {
    pub registry_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl Default for PortRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PortRegistry {
    pub fn new() -> Self {
        let config_dir = get_portctl_config_dir();
        let registry_dir = get_registry_dir();

        // One-time legacy migration to XDG if ~/.config/portctl/ports.json does not exist
        let config_ports = config_dir.join("ports.json");
        if !config_ports.exists() {
            if let Some(home) = dirs::home_dir() {
                let legacy_path = home.join("port-manager/ports.json");
                if legacy_path.exists() {
                    let _ = fs::create_dir_all(&config_dir);
                    let _ = fs::copy(&legacy_path, &config_ports);
                }
            }
        }

        Self::with_dirs(registry_dir, config_dir)
    }

    pub fn with_dirs(registry_dir: PathBuf, config_dir: PathBuf) -> Self {
        let registry = PortRegistry {
            registry_dir,
            config_dir,
        };
        registry.ensure_dirs();
        registry.bootstrap_if_empty();
        registry
    }

    pub fn ensure_dirs(&self) {
        if !self.registry_dir.exists() {
            let _ = fs::create_dir_all(&self.registry_dir);
        }
        let machines_dir = self.registry_dir.join("machines");
        if !machines_dir.exists() {
            let _ = fs::create_dir_all(&machines_dir);
        }
    }

    /// Dedicated advisory lockfile for cross-process mutations.
    /// Never unlinked or renamed to preserve POSIX inode stability.
    pub fn get_lock_path(&self) -> PathBuf {
        self.config_dir.join(".lock")
    }

    /// Execute a closure under exclusive advisory write lock with a 2000ms deadline.
    pub fn with_write_lock<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce() -> Result<R, String>,
    {
        let lock_path = self.get_lock_path();
        let parent = lock_path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| format!("Failed to open lockfile {:?}: {}", lock_path, e))?;

        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&lock_path, permissions);
        }

        let mut lock = fd_lock::RwLock::new(file);
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(2000);

        let _guard = loop {
            match lock.try_write() {
                Ok(guard) => break guard,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "Timeout acquiring exclusive registry write lock after 2000ms on {:?}",
                            lock_path
                        ));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => return Err(format!("Error acquiring lock on {:?}: {}", lock_path, e)),
            }
        };

        f()
    }

    fn bootstrap_if_empty(&self) {
        let canonical_path = self.registry_dir.join("canonical-apps.json");
        if canonical_path.exists() {
            return;
        }

        // Check if legacy ports.json exists in config_dir to bootstrap from
        let legacy_file = self.config_dir.join("ports.json");
        if legacy_file.exists() {
            if let Ok(content) = fs::read_to_string(&legacy_file) {
                if let Ok(legacy) = serde_json::from_str::<LegacyPortsFile>(&content) {
                    let mut canonical = CanonicalAppsFile::default();
                    for p in legacy.ports {
                        canonical.apps.insert(
                            p.name.clone(),
                            CanonicalAppItem {
                                port: p.port,
                                tunnel: p.tunnel,
                                repo: Some(p.github),
                                description: None,
                            },
                        );
                    }
                    let _ = self.save_canonical_apps(&canonical);
                }
            }
        }
    }

    pub fn load_fixed_services(&self) -> FixedServicesFile {
        let path = self.registry_dir.join("fixed-services.json");
        if !path.exists() {
            return FixedServicesFile::default();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    pub fn load_canonical_apps(&self) -> CanonicalAppsFile {
        let path = self.registry_dir.join("canonical-apps.json");
        if !path.exists() {
            return CanonicalAppsFile::default();
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    pub fn load_machine_allocations(&self, node_id: &str) -> MachineAllocationFile {
        let path = self
            .registry_dir
            .join("machines")
            .join(format!("{}.json", node_id));
        if !path.exists() {
            return MachineAllocationFile {
                node_id: node_id.to_string(),
                allocated: HashMap::new(),
            };
        }
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| MachineAllocationFile {
                node_id: node_id.to_string(),
                allocated: HashMap::new(),
            })
    }

    pub fn load_all_machines(&self) -> HashMap<String, MachineAllocationFile> {
        let mut result = HashMap::new();
        let machines_dir = self.registry_dir.join("machines");
        if let Ok(entries) = fs::read_dir(machines_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(m) = serde_json::from_str::<MachineAllocationFile>(&content) {
                                result.insert(file_stem.to_string(), m);
                            }
                        }
                    }
                }
            }
        }
        result
    }

    pub fn save_fixed_services(&self, data: &FixedServicesFile) -> Result<(), String> {
        self.with_write_lock(|| {
            let path = self.registry_dir.join("fixed-services.json");
            atomic_write_json(&path, data)?;
            self.sync_ports_json()
        })
    }

    pub fn save_canonical_apps(&self, data: &CanonicalAppsFile) -> Result<(), String> {
        self.with_write_lock(|| {
            let path = self.registry_dir.join("canonical-apps.json");
            atomic_write_json(&path, data)?;
            self.sync_ports_json()
        })
    }

    pub fn save_machine_allocation(
        &self,
        node_id: &str,
        app_name: &str,
        item: MachineAllocationItem,
    ) -> Result<(), String> {
        self.with_write_lock(|| {
            let path = self
                .registry_dir
                .join("machines")
                .join(format!("{}.json", node_id));
            let mut machine_data = self.load_machine_allocations(node_id);
            machine_data.node_id = node_id.to_string();
            machine_data.allocated.insert(app_name.to_string(), item);
            atomic_write_json(&path, &machine_data)?;
            self.sync_ports_json()
        })
    }

    pub fn get_all_merged(&self) -> Vec<MergedPortItem> {
        let fixed = self.load_fixed_services().services;
        let canonical = self.load_canonical_apps().apps;
        let machines = self.load_all_machines();

        let mut merged: Vec<MergedPortItem> = Vec::new();

        // 1. Canonical apps
        for (name, app) in canonical {
            merged.push(MergedPortItem {
                name,
                port: app.port,
                scope: "canonical-global".to_string(),
                entry_type: "custom".to_string(),
                tunnel: app.tunnel,
                repo: app.repo.unwrap_or_else(|| "Local Only".to_string()),
                description: app.description.unwrap_or_default(),
            });
        }

        // 2. Fixed services
        for (name, svc) in fixed {
            if !merged.iter().any(|m| m.name == name) {
                merged.push(MergedPortItem {
                    name,
                    port: svc.port,
                    scope: svc.scope.unwrap_or_else(|| "machine-local".to_string()),
                    entry_type: "fixed_third_party".to_string(),
                    tunnel: svc.tunnel,
                    repo: "Local Only".to_string(),
                    description: svc.description.unwrap_or_default(),
                });
            }
        }

        // 3. Machine-specific allocations
        for (m_id, m_data) in machines {
            for (name, alloc) in m_data.allocated {
                if !merged.iter().any(|m| m.name == name) {
                    merged.push(MergedPortItem {
                        name,
                        port: alloc.port,
                        scope: format!("machine:{}", m_id),
                        entry_type: "machine_allocated".to_string(),
                        tunnel: alloc.tunnel,
                        repo: alloc.repo.unwrap_or_else(|| "Local Only".to_string()),
                        description: alloc.description.unwrap_or_default(),
                    });
                }
            }
        }

        merged.sort_by_key(|m| m.port);
        merged
    }

    pub fn find_by_name(&self, name: &str) -> Option<MergedPortItem> {
        let all = self.get_all_merged();
        all.into_iter().find(|m| m.name == name)
    }

    pub fn find_by_port(&self, port: u16) -> Option<MergedPortItem> {
        let all = self.get_all_merged();
        all.into_iter().find(|m| m.port == port)
    }

    pub fn delete_app(&self, app_name: &str) -> Result<Option<MergedPortItem>, String> {
        self.with_write_lock(|| {
            let mut removed_item = None;

            // 1. Remove from canonical
            let mut canonical = self.load_canonical_apps();
            if let Some(app) = canonical.apps.remove(app_name) {
                removed_item = Some(MergedPortItem {
                    name: app_name.to_string(),
                    port: app.port,
                    scope: "canonical-global".to_string(),
                    entry_type: "canonical".to_string(),
                    tunnel: app.tunnel,
                    repo: app.repo.unwrap_or_default(),
                    description: app.description.unwrap_or_default(),
                });
                let path = self.registry_dir.join("canonical-apps.json");
                atomic_write_json(&path, &canonical)?;
            }

            // 2. Remove from machine allocations
            let machines_dir = self.registry_dir.join("machines");
            if let Ok(entries) = fs::read_dir(&machines_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(mut m) =
                                serde_json::from_str::<MachineAllocationFile>(&content)
                            {
                                if let Some(item) = m.allocated.remove(app_name) {
                                    if removed_item.is_none() {
                                        removed_item = Some(MergedPortItem {
                                            name: app_name.to_string(),
                                            port: item.port,
                                            scope: "machine-local".to_string(),
                                            entry_type: "machine".to_string(),
                                            tunnel: None,
                                            repo: String::new(),
                                            description: String::new(),
                                        });
                                    }
                                    atomic_write_json(&path, &m)?;
                                }
                            }
                        }
                    }
                }
            }

            self.sync_ports_json()?;
            Ok(removed_item)
        })
    }

    pub fn rename_app(&self, old_name: &str, new_name: &str) -> Result<MergedPortItem, String> {
        let clean_new = new_name.trim();
        if clean_new.is_empty() {
            return Err("New application name cannot be empty".to_string());
        }
        if old_name == clean_new {
            return Err("Old name and new name are identical".to_string());
        }

        self.with_write_lock(|| {
            // Check if new_name already exists
            if let Some(existing) = self.find_by_name(clean_new) {
                return Err(format!(
                    "Cannot rename: '{}' already exists in registry with port {}",
                    clean_new, existing.port
                ));
            }

            // 1. Try canonical apps
            let mut canonical = self.load_canonical_apps();
            if let Some(app) = canonical.apps.remove(old_name) {
                canonical.apps.insert(clean_new.to_string(), app.clone());
                let path = self.registry_dir.join("canonical-apps.json");
                atomic_write_json(&path, &canonical)?;
                self.sync_ports_json()?;
                return Ok(MergedPortItem {
                    name: clean_new.to_string(),
                    port: app.port,
                    scope: "canonical-global".to_string(),
                    entry_type: "canonical".to_string(),
                    tunnel: app.tunnel,
                    repo: app.repo.unwrap_or_default(),
                    description: app.description.unwrap_or_default(),
                });
            }

            // 2. Try machine allocations
            let machines_dir = self.registry_dir.join("machines");
            if let Ok(entries) = fs::read_dir(&machines_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(mut m) =
                                serde_json::from_str::<MachineAllocationFile>(&content)
                            {
                                if let Some(item) = m.allocated.remove(old_name) {
                                    m.allocated.insert(clean_new.to_string(), item.clone());
                                    atomic_write_json(&path, &m)?;
                                    self.sync_ports_json()?;
                                    return Ok(MergedPortItem {
                                        name: clean_new.to_string(),
                                        port: item.port,
                                        scope: "machine-local".to_string(),
                                        entry_type: "machine".to_string(),
                                        tunnel: None,
                                        repo: String::new(),
                                        description: String::new(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Err(format!("Application '{}' not found in registry", old_name))
        })
    }

    pub fn sync_ports_json(&self) -> Result<(), String> {
        let merged = self.get_all_merged();
        let legacy = LegacyPortsFile {
            version: "2.0.0".to_string(),
            last_updated: Some(chrono_iso_now()),
            ports: merged
                .into_iter()
                .map(|m| LegacyPortItem {
                    name: m.name,
                    port: m.port,
                    github: m.repo,
                    tunnel: m.tunnel,
                })
                .collect(),
        };

        // 1. Sync to ~/.config/portctl/ports.json
        let config_ports_path = self.config_dir.join("ports.json");
        let _ = atomic_write_json(&config_ports_path, &legacy);

        Ok(())
    }
}

fn chrono_iso_now() -> String {
    // Generate ISO 8601 string without pulling heavy chrono crate
    if let Ok(output) = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    "2026-10-25T00:00:00Z".to_string()
}
