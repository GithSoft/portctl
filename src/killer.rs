use std::thread;
use std::time::{Duration, Instant};

use crate::constants::get_sensitive_ports;
use crate::probe::{probe_port, SocketProbe};
use crate::registry::PortRegistry;

#[derive(Debug, Clone)]
pub struct KillResult {
    pub target: String,
    pub port: u16,
    pub pid: Option<String>,
    pub process_name: Option<String>,
    pub signal_sent: &'static str,
    pub socket_released: bool,
    pub in_time_wait: bool,
    pub message: String,
}

pub fn kill_target(
    registry: &PortRegistry,
    target: &str,
    force: bool,
) -> Result<KillResult, String> {
    // 1. Resolve target to port number
    let (port, app_display) = if let Ok(parsed_port) = target.parse::<u16>() {
        (parsed_port, format!("port {}", parsed_port))
    } else if let Some(entry) = registry.find_by_name(target) {
        (
            entry.port,
            format!("app '{}' (port {})", target, entry.port),
        )
    } else {
        return Err(format!(
            "Target '{}' is neither an active port number nor a registered application.",
            target
        ));
    };

    // 2. Security guardrail: sensitive port blacklist
    let sensitive = get_sensitive_ports();
    if sensitive.contains(&port) && !force {
        return Err(format!(
            "[SECURITY GUARDRAIL] Port {} is in the sensitive/system blacklist. Use '--force' if you intentionally wish to terminate this process.",
            port
        ));
    }

    // 3. Inspect socket and owning PID
    let probe = probe_port(port);
    if !probe.is_listening {
        return Ok(KillResult {
            target: target.to_string(),
            port,
            pid: None,
            process_name: None,
            signal_sent: "NONE",
            socket_released: true,
            in_time_wait: false,
            message: format!("Port {} is already idle (no process listening).", port),
        });
    }

    let pid_str = match probe.pid.clone() {
        Some(p) if !p.trim().is_empty() && p != "?" => p,
        _ => {
            return Err(format!(
                "Port {} is actively listening, but its owning PID could not be determined. Run with elevated privileges: 'sudo portctl kill {}'",
                port, target
            ));
        }
    };

    let pid: i32 = pid_str
        .parse()
        .map_err(|_| format!("Invalid PID '{}' returned by process probe.", pid_str))?;

    // Refuse to kill PID 1 or negative
    if pid <= 1 && !force {
        return Err(format!(
            "[SECURITY GUARDRAIL] Refusing to terminate root process with PID {}. Use '--force' to override.",
            pid
        ));
    }

    let cmd_name = probe
        .command
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    // 4. Terminate process with graceful escalation (SIGTERM -> 3000ms -> SIGKILL)
    terminate_process(pid, &cmd_name, port, target, &app_display, &probe)
}

#[cfg(unix)]
fn terminate_process(
    pid: i32,
    cmd_name: &str,
    port: u16,
    target: &str,
    app_display: &str,
    _probe: &SocketProbe,
) -> Result<KillResult, String> {
    let pid_t = pid as libc::pid_t;

    // Check if process is accessible
    let check = unsafe { libc::kill(pid_t, 0) };
    if check != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EPERM) {
            return Err(format!(
                "Permission denied attempting to terminate PID {} (process '{}'). Run with elevated privileges: 'sudo portctl kill {}'",
                pid, cmd_name, target
            ));
        }
    }

    // Send SIGTERM
    let term_res = unsafe { libc::kill(pid_t, libc::SIGTERM) };
    if term_res != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EPERM) {
            return Err(format!(
                "Permission denied attempting to terminate PID {} (process '{}'). Run with elevated privileges: 'sudo portctl kill {}'",
                pid, cmd_name, target
            ));
        }
    }

    let mut signal_sent = "SIGTERM";
    let start = Instant::now();
    let timeout = Duration::from_millis(3000);
    let mut is_dead = false;

    // Poll every 100ms up to 3000ms
    while start.elapsed() < timeout {
        // Check if process still exists
        let res = unsafe { libc::kill(pid_t, 0) };
        if res != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                is_dead = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    // If still alive, escalate to SIGKILL
    if !is_dead {
        unsafe {
            let _ = libc::kill(pid_t, libc::SIGKILL);
        }
        signal_sent = "SIGKILL";
        thread::sleep(Duration::from_millis(200));

        let res = unsafe { libc::kill(pid_t, 0) };
        if res != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                is_dead = true;
            }
        }
    }

    // Verify socket availability
    let post_probe = probe_port(port);
    let socket_released = !post_probe.is_listening;
    let in_time_wait = is_dead && !socket_released;

    let message = if socket_released {
        format!(
            "Successfully terminated PID {} ('{}') on {}. Socket is free.",
            pid, cmd_name, app_display
        )
    } else if in_time_wait {
        format!(
            "Process PID {} ('{}') terminated. Socket {} is in TIME_WAIT and will be reclaimed by the OS kernel.",
            pid, cmd_name, port
        )
    } else {
        format!(
            "Signal {} sent to PID {}, but socket {} remains occupied.",
            signal_sent, pid, port
        )
    };

    Ok(KillResult {
        target: target.to_string(),
        port,
        pid: Some(pid.to_string()),
        process_name: Some(cmd_name.to_string()),
        signal_sent,
        socket_released,
        in_time_wait,
        message,
    })
}

#[cfg(not(unix))]
fn terminate_process(
    pid: i32,
    cmd_name: &str,
    port: u16,
    target: &str,
    app_display: &str,
    _probe: &SocketProbe,
) -> Result<KillResult, String> {
    use std::process::Command;

    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

    let socket_released = !probe_port(port).is_listening;

    Ok(KillResult {
        target: target.to_string(),
        port,
        pid: Some(pid.to_string()),
        process_name: Some(cmd_name.to_string()),
        signal_sent: "TASKKILL",
        socket_released,
        in_time_wait: false,
        message: if socket_released {
            format!(
                "Successfully terminated PID {} ('{}') on {}.",
                pid, cmd_name, app_display
            )
        } else {
            format!(
                "Termination signal sent to PID {}, but socket {} remains busy.",
                pid, port
            )
        },
    })
}
