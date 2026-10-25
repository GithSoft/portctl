use clap::{Parser, Subcommand};
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};
use std::env;
use std::fs;
use std::process;

use portctl::allocator::PortAllocator;
use portctl::detector::{detect_project_name, get_machine_node_id};
use portctl::probe::probe_port;
use portctl::registry::PortRegistry;
use portctl::runner::execute_with_port;
use portctl::tunnel::TunnelManager;

#[derive(Parser)]
#[command(
    name = "portctl",
    author = "tackelua <tackelua@gmail.com>",
    version = "2.0.0",
    about = "Universal Port Manager & Automated Cloudflare Ingress CLI",
    long_about = "Lightweight, zero-daemon, local-first Port Manager and automated Edge Ingress CLI written in Rust."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered ports and active process socket statuses
    List {
        /// Output results in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Get allocated port for an app (defaults to current project directory)
    Get {
        /// App or service name
        app_name: Option<String>,
    },

    /// Allocate a new port or register a fixed service
    Assign {
        /// App or service name (defaults to current project directory)
        app_name: Option<String>,

        /// Register a fixed port number (for 3rd-party or static services)
        #[arg(short, long)]
        fixed: Option<u16>,

        /// Scope: local (machine-local) or global (canonical-global)
        #[arg(short, long, default_value = "global")]
        scope: String,

        /// Associated Cloudflare Tunnel subdomain
        #[arg(short, long)]
        domain: Option<String>,

        /// Description of the service
        #[arg(long)]
        desc: Option<String>,
    },

    /// Inspect socket status and process owning a port or app
    Check {
        /// Target port number or application name (defaults to current directory)
        target: Option<String>,
    },

    /// Expose an application to the Internet via Cloudflare Tunnel
    Expose {
        /// Application name
        app_name: String,

        /// Public domain name (e.g. app.example.com)
        #[arg(short, long)]
        domain: String,

        /// Bypass security blacklist check for sensitive ports
        #[arg(long)]
        force: bool,
    },

    /// Generate or update PORT environment variable
    Env {
        /// Write or update PORT in .env or .env.local file
        #[arg(short, long)]
        write: bool,

        /// Print shell export command (export PORT=xxxx)
        #[arg(short, long)]
        export: bool,
    },

    /// Execute any command with $PORT injected into environment
    Run {
        /// Command to execute (e.g. npm run dev, cargo run, python -m http.server)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Sync port registry with local disk and system state
    Sync,
}

fn main() {
    let cli = Cli::parse();
    let registry = PortRegistry::new();

    match cli.command {
        Commands::List { json } => {
            let all = registry.get_all_merged();
            let node_id = get_machine_node_id();

            if json {
                if let Ok(serialized) = serde_json::to_string_pretty(&all) {
                    println!("{}", serialized);
                }
                return;
            }

            println!(
                "\n📌 {} (Node: {})\n",
                "PORTCTL REGISTRY".bold(),
                node_id.cyan().bold()
            );

            let mut table = Table::new();
            table
                .load_style(UTF8_FULL.with_rounded_corners())
                .set_header(vec![
                    Cell::new("Service / App Name").fg(Color::Cyan),
                    Cell::new("Port").fg(Color::Yellow),
                    Cell::new("Scope").fg(Color::White),
                    Cell::new("Status").fg(Color::Green),
                    Cell::new("Ingress Domain / Tunnel").fg(Color::Blue),
                ]);

            for item in all {
                let probe = probe_port(item.port);
                let status_cell = if probe.is_listening {
                    Cell::new(format!(
                        "ON (PID {})",
                        probe.pid.unwrap_or_else(|| "?".to_string())
                    ))
                    .fg(Color::Green)
                } else {
                    Cell::new("OFFLINE").fg(Color::DarkGrey)
                };

                let tunnel_cell = match item.tunnel {
                    Some(ref t) => Cell::new(format!("🔗 {}", t)).fg(Color::Blue),
                    None => Cell::new("-").fg(Color::DarkGrey),
                };

                table.add_row(Row::from(vec![
                    Cell::new(item.name).fg(Color::White),
                    Cell::new(item.port.to_string()).fg(Color::Yellow),
                    Cell::new(item.scope).fg(Color::Cyan),
                    status_cell,
                    tunnel_cell,
                ]));
            }

            println!("{}\n", table);
        }

        Commands::Get { app_name } => {
            let cwd = env::current_dir().unwrap_or_default();
            let target_name = app_name.unwrap_or_else(|| detect_project_name(&cwd));

            if let Some(entry) = registry.find_by_name(&target_name) {
                println!("{}", entry.port);
            } else {
                eprintln!(
                    "{} No port configuration found for '{}'. Run 'portctl assign' to allocate one.",
                    "Error:".bold().red(),
                    target_name
                );
                process::exit(1);
            }
        }

        Commands::Assign {
            app_name,
            fixed,
            scope,
            domain,
            desc,
        } => {
            let cwd = env::current_dir().unwrap_or_default();
            let target_name = app_name.unwrap_or_else(|| detect_project_name(&cwd));

            let allocator = PortAllocator::new(&registry);
            match allocator.allocate(
                &target_name,
                fixed,
                Some(&scope),
                domain.as_deref(),
                desc.as_deref(),
                None,
            ) {
                Ok(result) => {
                    if result.is_new {
                        println!(
                            "✅ Successfully allocated for '{}': Port {}",
                            target_name.cyan(),
                            result.port.to_string().bold().green()
                        );
                    } else {
                        println!(
                            "ℹ️ '{}' already assigned: Port {}",
                            target_name.cyan(),
                            result.port.to_string().bold().green()
                        );
                    }

                    if let Some(ref d) = domain {
                        println!("Configuring Cloudflare Tunnel for {}...", d);
                        let tunnel = TunnelManager::new(&registry);
                        if let Err(e) = tunnel.expose(&target_name, d, false) {
                            eprintln!("{} {}", "Warning: Failed to configure tunnel:".yellow(), e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "❌ Port allocation failed:".bold().red(), e);
                    process::exit(1);
                }
            }
        }

        Commands::Check { target } => {
            let cwd = env::current_dir().unwrap_or_default();
            let (port, target_name) = match target {
                None => {
                    let name = detect_project_name(&cwd);
                    let p = registry.find_by_name(&name).map(|e| e.port);
                    (p, Some(name))
                }
                Some(t) => {
                    if let Ok(p) = t.parse::<u16>() {
                        (Some(p), None)
                    } else {
                        let p = registry.find_by_name(&t).map(|e| e.port);
                        (p, Some(t))
                    }
                }
            };

            let port = match port {
                Some(p) => p,
                None => {
                    eprintln!(
                        "{} Target '{}' not found in port registry.",
                        "Error:".bold().red(),
                        target_name.unwrap_or_else(|| "unknown".to_string())
                    );
                    process::exit(1);
                }
            };

            println!(
                "\n🔍 Socket Inspection for Port {}:",
                port.to_string().bold().yellow()
            );
            if let Some(entry) = registry.find_by_port(port) {
                println!(
                    "- Registry: Claimed by '{}' (Scope: {})",
                    entry.name.cyan(),
                    entry.scope
                );
            } else {
                println!("- Registry: Not registered.");
            }

            let probe = probe_port(port);
            if probe.is_listening {
                println!("- Status: {}", "LISTENING".bold().green());
                println!(
                    "  • Process: {}",
                    probe.command.unwrap_or_else(|| "?".to_string())
                );
                println!(
                    "  • PID:     {}",
                    probe.pid.unwrap_or_else(|| "?".to_string())
                );
                println!(
                    "  • User:    {}",
                    probe.user.unwrap_or_else(|| "?".to_string())
                );
            } else {
                println!(
                    "- Status: {}",
                    "PORT IS IDLE (No process listening)".dimmed()
                );
            }
            println!();
        }

        Commands::Expose {
            app_name,
            domain,
            force,
        } => {
            println!(
                "🚀 Exposing '{}' to {}...",
                app_name.cyan(),
                domain.bold().green()
            );
            let tunnel = TunnelManager::new(&registry);
            match tunnel.expose(&app_name, &domain, force) {
                Ok(res) => {
                    println!("✅ Successfully exposed!");
                    println!("   Public URL  : {}", res.url.bold().blue());
                    println!("   Local Target: http://localhost:{}", res.port);
                }
                Err(e) => {
                    eprintln!("{} {}", "❌ Ingress failed:".bold().red(), e);
                    process::exit(1);
                }
            }
        }

        Commands::Env { write, export } => {
            let cwd = env::current_dir().unwrap_or_default();
            let app_name = detect_project_name(&cwd);

            let port = match registry.find_by_name(&app_name) {
                Some(entry) => entry.port,
                None => {
                    let allocator = PortAllocator::new(&registry);
                    match allocator.allocate(&app_name, None, None, None, None, None) {
                        Ok(res) => res.port,
                        Err(e) => {
                            eprintln!("Failed to allocate port: {}", e);
                            process::exit(1);
                        }
                    }
                }
            };

            if export {
                println!("export PORT={}", port);
                return;
            }

            if write {
                let is_next = cwd.join("next.config.js").exists()
                    || cwd.join("next.config.mjs").exists()
                    || cwd.join("next.config.ts").exists();

                let env_filename = if is_next { ".env.local" } else { ".env" };
                let env_path = cwd.join(env_filename);

                let mut content = if env_path.exists() {
                    fs::read_to_string(&env_path).unwrap_or_default()
                } else {
                    String::new()
                };

                let port_re = regex::Regex::new(r"(?m)^PORT=.*").unwrap();
                if port_re.is_match(&content) {
                    content = port_re
                        .replace(&content, format!("PORT={}", port))
                        .to_string();
                } else if !content.trim().is_empty() {
                    content = format!("{}\nPORT={}\n", content.trim(), port);
                } else {
                    content = format!("PORT={}\n", port);
                }

                if let Err(e) = fs::write(&env_path, content) {
                    eprintln!("Failed to write {}: {}", env_filename, e);
                    process::exit(1);
                }

                println!(
                    "✅ Updated {}: PORT={} (for '{}')",
                    env_filename,
                    port.to_string().bold().green(),
                    app_name.cyan()
                );
                return;
            }

            println!("PORT={}", port);
        }

        Commands::Run { command } => match execute_with_port(&registry, None, &command) {
            Ok(code) => process::exit(code),
            Err(e) => {
                eprintln!("{} {}", "❌ Error:".bold().red(), e);
                process::exit(1);
            }
        },

        Commands::Sync => {
            if let Err(e) = registry.sync_ports_json() {
                eprintln!("{} {}", "Failed to sync:".bold().red(), e);
                process::exit(1);
            }
            println!("✅ Successfully synchronized registry and ports.json");
        }
    }
}
