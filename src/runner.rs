use colored::Colorize;
use std::env;
use std::process::{Command, Stdio};

use crate::allocator::PortAllocator;
use crate::detector::detect_project_name;
use crate::registry::PortRegistry;

pub fn execute_with_port(
    registry: &PortRegistry,
    app_name_opt: Option<&str>,
    cmd_parts: &[String],
) -> Result<i32, String> {
    if cmd_parts.is_empty() {
        return Err("No command provided to execute.".to_string());
    }

    let cwd = env::current_dir().unwrap_or_default();
    let app_name = app_name_opt
        .map(|s| s.to_string())
        .unwrap_or_else(|| detect_project_name(&cwd));

    let allocator = PortAllocator::new(registry);
    let alloc_res = allocator.allocate(&app_name, None, None, None, None, None)?;
    let port = alloc_res.port;

    eprintln!(
        "{} Injecting PORT={} for '{}'",
        "⚡ [portctl run]".bold().yellow(),
        port.to_string().bold().green(),
        app_name.bold().cyan()
    );

    let executable = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let mut command = Command::new(executable);
    command.args(args);
    command.env("PORT", port.to_string());
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to start process '{}': {}", executable, e))?;

    let exit_status = child
        .wait()
        .map_err(|e| format!("Failed to wait for process '{}': {}", executable, e))?;

    Ok(exit_status.code().unwrap_or(0))
}
