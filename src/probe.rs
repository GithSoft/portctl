use std::net::TcpListener;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SocketProbe {
    pub is_listening: bool,
    pub pid: Option<String>,
    pub command: Option<String>,
    pub user: Option<String>,
}

pub fn probe_port(port: u16) -> SocketProbe {
    // 1. Fast socket check via TcpListener bind
    let is_bound = TcpListener::bind(("127.0.0.1", port)).is_err();

    if !is_bound {
        return SocketProbe {
            is_listening: false,
            pid: None,
            command: None,
            user: None,
        };
    }

    // 2. Fetch process details via lsof
    let mut probe = SocketProbe {
        is_listening: true,
        pid: None,
        command: None,
        user: None,
    };

    if let Ok(output) = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{}", port), "-sTCP:LISTEN"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.trim().lines().collect();
            if lines.len() > 1 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() >= 3 {
                    probe.command = Some(parts[0].to_string());
                    probe.pid = Some(parts[1].to_string());
                    probe.user = Some(parts[2].to_string());
                }
            }
        }
    }

    probe
}
