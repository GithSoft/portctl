use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

pub const DEFAULT_PORT_RANGE: (u16, u16) = (4000, 4999);

pub fn get_sensitive_ports() -> HashSet<u16> {
    let mut s = HashSet::new();
    s.insert(22); // SSH
    s.insert(80); // HTTP standard
    s.insert(443); // HTTPS standard
    s.insert(2375); // Docker daemon
    s.insert(3306); // MySQL
    s.insert(5432); // PostgreSQL
    s.insert(6379); // Redis
    s.insert(8384); // Syncthing admin UI
    s.insert(9222); // Chrome remote debugging
    s.insert(11434); // Ollama API
    s.insert(27017); // MongoDB
    s
}

pub fn get_portctl_config_dir() -> PathBuf {
    if let Ok(val) = env::var("PORTCTL_CONFIG_DIR") {
        return PathBuf::from(val);
    }
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("portctl");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("portctl")
}

pub fn get_registry_dir() -> PathBuf {
    if let Ok(val) = env::var("PORTCTL_REGISTRY_DIR") {
        return PathBuf::from(val);
    }
    get_portctl_config_dir().join("registry")
}

pub fn get_ports_json_path() -> PathBuf {
    get_portctl_config_dir().join("ports.json")
}

pub fn get_tunnel_config_path() -> PathBuf {
    if let Ok(val) = env::var("PORTCTL_TUNNEL_CONFIG") {
        return PathBuf::from(val);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cloudflare-githsoft/tunnel/config.yml")
}

pub fn get_system_tunnel_config_path() -> &'static str {
    "/etc/cloudflared/config.yml"
}
