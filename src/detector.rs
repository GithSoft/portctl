use crate::constants::get_portctl_config_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_machine_node_id() -> String {
    // 1. Try ~/.config/portctl/config.json
    let config_file = get_portctl_config_dir().join("config.json");
    if config_file.exists() {
        if let Ok(content) = fs::read_to_string(&config_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(node_id) = val.get("node_id").and_then(|v| v.as_str()) {
                    if !node_id.trim().is_empty() {
                        return node_id.trim().to_string();
                    }
                }
            }
        }
    }

    // 2. Try ~/.portctl-node-id
    if let Some(home) = dirs::home_dir() {
        let custom_id_file = home.join(".portctl-node-id");
        if custom_id_file.exists() {
            if let Ok(id) = fs::read_to_string(&custom_id_file) {
                let trimmed = id.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    // 3. Fallback to hostname heuristics
    if let Ok(output) = Command::new("hostname").output() {
        let hostname = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();
        if hostname.contains("mini") {
            return "node-macmini".to_string();
        }
        if hostname.contains("book") || hostname.contains("laptop") {
            return "node-macbook".to_string();
        }
        let clean: String = hostname
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        return format!("node-{}", clean);
    }

    "node-default".to_string()
}

pub fn find_project_root(start_dir: &Path) -> PathBuf {
    let mut current = start_dir.to_path_buf();
    let markers = [
        "package.json",
        "pyproject.toml",
        "setup.py",
        "Cargo.toml",
        "go.mod",
        "composer.json",
        "pom.xml",
        "build.gradle",
        "mix.exs",
        ".git",
    ];

    loop {
        for marker in &markers {
            if current.join(marker).exists() {
                return current;
            }
        }
        if let Some(parent) = current.parent() {
            if parent == current {
                break;
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    start_dir.to_path_buf()
}

pub fn detect_project_name(start_dir: &Path) -> String {
    let root = find_project_root(start_dir);

    // 1. Node.js (package.json)
    let pkg_path = root.join("package.json");
    if pkg_path.exists() {
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    let clean = if name.starts_with('@') {
                        name.split('/').nth(1).unwrap_or(name)
                    } else {
                        name
                    };
                    if !clean.trim().is_empty() {
                        return clean.trim().to_string();
                    }
                }
            }
        }
    }

    // 2. Python (pyproject.toml)
    let pyproject_path = root.join("pyproject.toml");
    if pyproject_path.exists() {
        if let Ok(content) = fs::read_to_string(&pyproject_path) {
            let re = regex::Regex::new(r#"name\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
    }

    // 3. Rust (Cargo.toml)
    let cargo_path = root.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            let re = regex::Regex::new(r#"name\s*=\s*["']([^"']+)["']"#).unwrap();
            if let Some(caps) = re.captures(&content) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
    }

    // 4. Go (go.mod)
    let go_path = root.join("go.mod");
    if go_path.exists() {
        if let Ok(content) = fs::read_to_string(&go_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(stripped) = trimmed.strip_prefix("module ") {
                    let parts: Vec<&str> = stripped.trim().split('/').collect();
                    if let Some(last) = parts.last() {
                        if !last.trim().is_empty() {
                            return last.trim().to_string();
                        }
                    }
                }
            }
        }
    }

    // 5. Git Remote Origin URL
    if let Ok(output) = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&root)
        .output()
    {
        if output.status.success() {
            let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let re = regex::Regex::new(r#"/([^/]+?)(?:\.git)?$"#).unwrap();
            if let Some(caps) = re.captures(&remote) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
    }

    // 6. Fallback to folder name
    root.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("app")
        .to_string()
}

pub fn get_git_remote_url(dir: &Path) -> String {
    if let Ok(output) = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(dir)
        .output()
    {
        if output.status.success() {
            let mut s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if s.ends_with(".git") {
                s.truncate(s.len() - 4);
            }
            return s;
        }
    }
    "Local Only".to_string()
}
