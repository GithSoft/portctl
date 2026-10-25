use crate::constants::{get_sensitive_ports, DEFAULT_PORT_RANGE};
use crate::detector::{get_git_remote_url, get_machine_node_id};
use crate::probe::probe_port;
use crate::registry::{CanonicalAppItem, FixedServiceItem, MergedPortItem, PortRegistry};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AllocationResult {
    pub port: u16,
    pub is_new: bool,
    pub entry: MergedPortItem,
}

pub struct PortAllocator<'a> {
    registry: &'a PortRegistry,
}

impl<'a> PortAllocator<'a> {
    pub fn new(registry: &'a PortRegistry) -> Self {
        Self { registry }
    }

    pub fn allocate(
        &self,
        app_name: &str,
        fixed_port: Option<u16>,
        scope: Option<&str>,
        domain: Option<&str>,
        desc: Option<&str>,
        repo: Option<&str>,
    ) -> Result<AllocationResult, String> {
        // 1. If already exists
        if let Some(existing) = self.registry.find_by_name(app_name) {
            return Ok(AllocationResult {
                port: existing.port,
                is_new: false,
                entry: existing,
            });
        }

        let fallback_repo = get_git_remote_url(&std::env::current_dir().unwrap_or_default());
        let effective_repo = repo.map(|r| r.to_string()).unwrap_or(fallback_repo);

        // 2. Explicit fixed port requested
        if let Some(port) = fixed_port {
            if let Some(conflict) = self.registry.find_by_port(port) {
                if conflict.name != app_name {
                    return Err(format!(
                        "Port {} already claimed by service '{}'!",
                        port, conflict.name
                    ));
                }
            }

            if scope == Some("local") {
                let mut fixed_data = self.registry.load_fixed_services();
                fixed_data.services.insert(
                    app_name.to_string(),
                    FixedServiceItem {
                        port,
                        scope: Some("machine-local".to_string()),
                        category: Some("general".to_string()),
                        description: desc.map(|s| s.to_string()),
                        tunnel: domain.map(|s| s.to_string()),
                    },
                );
                self.registry.save_fixed_services(&fixed_data)?;
            } else {
                let mut canonical = self.registry.load_canonical_apps();
                canonical.apps.insert(
                    app_name.to_string(),
                    CanonicalAppItem {
                        port,
                        tunnel: domain.map(|s| s.to_string()),
                        repo: Some(effective_repo),
                        description: desc.map(|s| s.to_string()),
                    },
                );
                self.registry.save_canonical_apps(&canonical)?;
            }

            let entry = self
                .registry
                .find_by_name(app_name)
                .ok_or_else(|| "Failed to retrieve saved port entry".to_string())?;

            return Ok(AllocationResult {
                port,
                is_new: true,
                entry,
            });
        }

        // 3. Dynamic auto-allocation
        let next_port = self.find_next_available(DEFAULT_PORT_RANGE.0, DEFAULT_PORT_RANGE.1)?;
        let node_id = get_machine_node_id();

        let mut canonical = self.registry.load_canonical_apps();
        canonical.apps.insert(
            app_name.to_string(),
            CanonicalAppItem {
                port: next_port,
                tunnel: domain.map(|s| s.to_string()),
                repo: Some(effective_repo),
                description: desc
                    .map(|s| s.to_string())
                    .or_else(|| Some(format!("Allocated on {}", node_id))),
            },
        );
        self.registry.save_canonical_apps(&canonical)?;

        let entry = self
            .registry
            .find_by_name(app_name)
            .ok_or_else(|| "Failed to retrieve saved port entry".to_string())?;

        Ok(AllocationResult {
            port: next_port,
            is_new: true,
            entry,
        })
    }

    pub fn find_next_available(&self, start: u16, end: u16) -> Result<u16, String> {
        let all = self.registry.get_all_merged();
        let used_ports: HashSet<u16> = all.iter().map(|item| item.port).collect();
        let sensitive = get_sensitive_ports();

        for p in start..=end {
            // Must not be registered in registry
            if used_ports.contains(&p) {
                continue;
            }

            // Must not be sensitive blacklist port
            if sensitive.contains(&p) {
                continue;
            }

            // Must not be actively bound by another process
            let probe = probe_port(p);
            if probe.is_listening {
                continue;
            }

            return Ok(p);
        }

        Err(format!("No available port in range {}-{}", start, end))
    }
}
