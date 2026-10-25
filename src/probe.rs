use serde::Serialize;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct SocketProbe {
    pub is_listening: bool,
    pub pid: Option<String>,
    pub command: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WaitResult {
    pub port: u16,
    pub mode: String,
    pub ready: bool,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub enum WaitError {
    Timeout {
        port: u16,
        timeout_secs: u64,
        elapsed_ms: u128,
    },
    InvalidMode(String),
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitError::Timeout {
                port,
                timeout_secs,
                elapsed_ms,
            } => write!(
                f,
                "Timeout: Port {} was not ready within {}s (waited {}ms)",
                port, timeout_secs, elapsed_ms
            ),
            WaitError::InvalidMode(mode) => write!(
                f,
                "Invalid probe mode '{}'. Supported modes: tcp, http",
                mode
            ),
        }
    }
}

impl std::error::Error for WaitError {}

pub fn wait_for_port(
    port: u16,
    mode: &str,
    path: &str,
    timeout_secs: u64,
    interval_ms: u64,
) -> Result<WaitResult, WaitError> {
    let mode_lower = mode.to_lowercase();
    if mode_lower != "tcp" && mode_lower != "http" {
        return Err(WaitError::InvalidMode(mode.to_string()));
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let interval = Duration::from_millis(interval_ms.max(10));
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("Valid 127.0.0.1 address");

    loop {
        if mode_lower == "http" {
            if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
                stream
                    .set_read_timeout(Some(Duration::from_millis(500)))
                    .ok();
                stream
                    .set_write_timeout(Some(Duration::from_millis(500)))
                    .ok();
                let clean_path = if path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/{}", path)
                };
                let req = format!(
                    "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
                    clean_path, port
                );
                if stream.write_all(req.as_bytes()).is_ok() {
                    let mut buf = [0u8; 256];
                    if let Ok(n) = stream.read(&mut buf) {
                        let response = String::from_utf8_lossy(&buf[..n]);
                        // Accept any 2xx or 3xx HTTP response as server ready
                        if response.starts_with("HTTP/1.0 2")
                            || response.starts_with("HTTP/1.1 2")
                            || response.starts_with("HTTP/1.0 3")
                            || response.starts_with("HTTP/1.1 3")
                        {
                            return Ok(WaitResult {
                                port,
                                mode: "http".to_string(),
                                ready: true,
                                elapsed_ms: start.elapsed().as_millis(),
                            });
                        }
                    }
                }
            }
        } else if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(WaitResult {
                port,
                mode: "tcp".to_string(),
                ready: true,
                elapsed_ms: start.elapsed().as_millis(),
            });
        }

        if start.elapsed() >= timeout {
            return Err(WaitError::Timeout {
                port,
                timeout_secs,
                elapsed_ms: start.elapsed().as_millis(),
            });
        }

        std::thread::sleep(interval);
    }
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
