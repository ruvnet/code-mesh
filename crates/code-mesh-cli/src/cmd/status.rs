//! Status command implementation - System health checks and diagnostics

use crate::cmd::{CliError, Result, UI, Config};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Execute the status command
pub async fn execute(detailed: bool) -> Result<()> {
    let mut ui = UI::new();
    
    ui.print_logo()?;
    ui.info("Code Mesh System Status")?;
    ui.println("")?;

    // Overall health check
    let health_checks = run_health_checks().await?;
    let all_healthy = health_checks.iter().all(|(_, status)| status.healthy);

    if all_healthy {
        ui.success("All systems operational")?;
    } else {
        ui.warning("Some issues detected")?;
    }
    ui.println("")?;

    // Display system information
    display_system_info(&mut ui, detailed).await?;
    
    // Display health checks
    display_health_checks(&mut ui, &health_checks, detailed).await?;
    
    // Display configuration status
    display_config_status(&mut ui, detailed).await?;
    
    // Display authentication status
    display_auth_status(&mut ui, detailed).await?;
    
    // Display recent activity (if detailed)
    if detailed {
        display_recent_activity(&mut ui).await?;
    }

    Ok(())
}

/// System information
#[derive(Debug)]
struct SystemInfo {
    version: String,
    platform: String,
    architecture: String,
    rust_version: String,
    config_dir: String,
    data_dir: String,
    cache_dir: String,
}

/// Health check status
#[derive(Debug)]
struct HealthStatus {
    healthy: bool,
    message: String,
    details: Option<String>,
}

/// Run all health checks
async fn run_health_checks() -> Result<HashMap<String, HealthStatus>> {
    let mut checks = HashMap::new();

    // Configuration check
    checks.insert("Configuration".to_string(), check_configuration().await);
    
    // Directory permissions check
    checks.insert("Directories".to_string(), check_directories().await);
    
    // Authentication check
    checks.insert("Authentication".to_string(), check_authentication().await);
    
    // Dependencies check
    checks.insert("Dependencies".to_string(), check_dependencies().await);
    
    // Network connectivity check (basic)
    checks.insert("Network".to_string(), check_network().await);

    Ok(checks)
}

/// Check configuration status
async fn check_configuration() -> HealthStatus {
    match Config::load() {
        Ok(config) => {
            match config.validate() {
                Ok(()) => HealthStatus {
                    healthy: true,
                    message: "Configuration valid".to_string(),
                    details: Some(format!("Version: {}", config.version)),
                },
                Err(e) => HealthStatus {
                    healthy: false,
                    message: "Configuration invalid".to_string(),
                    details: Some(e.to_string()),
                },
            }
        }
        Err(e) => HealthStatus {
            healthy: false,
            message: "Configuration not found".to_string(),
            details: Some(e.to_string()),
        },
    }
}

/// Check directory permissions
async fn check_directories() -> HealthStatus {
    let dirs_to_check = vec![
        ("Config", Config::config_path()),
        ("Data", Config::data_dir()),
        ("Cache", Config::cache_dir()),
    ];

    let mut issues = Vec::new();

    for (name, dir_result) in dirs_to_check {
        match dir_result {
            Ok(dir) => {
                if let Some(parent) = dir.parent() {
                    if !parent.exists() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            issues.push(format!("{} directory not accessible: {}", name, e));
                        }
                    }
                }
            }
            Err(e) => {
                issues.push(format!("{} directory path error: {}", name, e));
            }
        }
    }

    if issues.is_empty() {
        HealthStatus {
            healthy: true,
            message: "All directories accessible".to_string(),
            details: None,
        }
    } else {
        HealthStatus {
            healthy: false,
            message: "Directory access issues".to_string(),
            details: Some(issues.join("; ")),
        }
    }
}

/// Check authentication status
async fn check_authentication() -> HealthStatus {
    // Check environment variables
    let env_vars = vec![
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GOOGLE_API_KEY",
        "GITHUB_TOKEN",
    ];

    let env_auth_count = env_vars.iter()
        .filter(|var| std::env::var(var).is_ok())
        .count();

    // TODO: Check stored credentials using AuthManager
    let stored_auth_count = 0; // Placeholder

    let total_auth = env_auth_count + stored_auth_count;

    if total_auth > 0 {
        HealthStatus {
            healthy: true,
            message: format!("{} provider(s) configured", total_auth),
            details: Some(format!(
                "Environment: {}, Stored: {}",
                env_auth_count, stored_auth_count
            )),
        }
    } else {
        HealthStatus {
            healthy: false,
            message: "No authentication configured".to_string(),
            details: Some("Run 'code-mesh auth login' to set up authentication".to_string()),
        }
    }
}

/// Check dependencies
async fn check_dependencies() -> HealthStatus {
    let mut missing = Vec::new();
    let mut present = Vec::new();

    // Check for optional external tools
    let tools = vec![
        ("git", "Git version control"),
        ("rg", "Ripgrep for fast searching"),
        ("fzf", "Fuzzy finder"),
    ];

    for (tool, description) in tools {
        if crate::cmd::utils::Utils::command_exists(tool) {
            present.push(format!("{} ({})", tool, description));
        } else {
            missing.push(format!("{} ({})", tool, description));
        }
    }

    if missing.is_empty() {
        HealthStatus {
            healthy: true,
            message: "All optional tools available".to_string(),
            details: Some(format!("Found: {}", present.join(", "))),
        }
    } else {
        HealthStatus {
            healthy: true, // Not critical
            message: format!("{} optional tools missing", missing.len()),
            details: Some(format!("Missing: {}", missing.join(", "))),
        }
    }
}

/// Check network connectivity
async fn check_network() -> HealthStatus {
    // Simple connectivity check - try to resolve a DNS name
    // This is a basic check and doesn't test actual API endpoints
    
    use std::time::Duration;
    use tokio::time::timeout;
    
    let check_result = timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect("8.8.8.8:53")
    ).await;

    match check_result {
        Ok(Ok(_)) => HealthStatus {
            healthy: true,
            message: "Network connectivity available".to_string(),
            details: None,
        },
        Ok(Err(e)) => HealthStatus {
            healthy: false,
            message: "Network connectivity issues".to_string(),
            details: Some(e.to_string()),
        },
        Err(_) => HealthStatus {
            healthy: false,
            message: "Network check timed out".to_string(),
            details: Some("Connection attempt took too long".to_string()),
        },
    }
}

/// Display system information
async fn display_system_info(ui: &mut UI, detailed: bool) -> Result<()> {
    ui.info("System Information")?;
    ui.println("")?;

    let info = SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        rust_version: "Unknown".to_string(), // Could use build-time detection
        config_dir: Config::config_path()?.display().to_string(),
        data_dir: Config::data_dir()?.display().to_string(),
        cache_dir: Config::cache_dir()?.display().to_string(),
    };

    let mut table = crate::cmd::ui::Table::new(vec![
        "Property".to_string(),
        "Value".to_string(),
    ]);

    table.add_row(vec!["Version".to_string(), info.version]);
    table.add_row(vec!["Platform".to_string(), format!("{}/{}", info.platform, info.architecture)]);
    
    if detailed {
        table.add_row(vec!["Config Directory".to_string(), info.config_dir]);
        table.add_row(vec!["Data Directory".to_string(), info.data_dir]);
        table.add_row(vec!["Cache Directory".to_string(), info.cache_dir]);
    }

    table.print(ui)?;
    ui.println("")?;

    Ok(())
}

/// Display health check results
async fn display_health_checks(
    ui: &mut UI,
    checks: &HashMap<String, HealthStatus>,
    detailed: bool,
) -> Result<()> {
    ui.info("Health Checks")?;
    ui.println("")?;

    let mut table = crate::cmd::ui::Table::new(vec![
        "Component".to_string(),
        "Status".to_string(),
        "Message".to_string(),
    ]);

    for (component, status) in checks {
        let status_icon = if status.healthy { "✓" } else { "✗" };
        let status_text = format!("{} {}", status_icon, if status.healthy { "OK" } else { "ISSUE" });
        
        table.add_row(vec![
            component.clone(),
            status_text,
            status.message.clone(),
        ]);
    }

    table.print(ui)?;

    // Show details if requested
    if detailed {
        ui.println("")?;
        ui.info("Details")?;
        ui.println("")?;

        for (component, status) in checks {
            if let Some(ref details) = status.details {
                ui.dim(&format!("{}: {}", component, details))?;
            }
        }
    }

    ui.println("")?;
    Ok(())
}

/// Display configuration status
async fn display_config_status(ui: &mut UI, detailed: bool) -> Result<()> {
    ui.info("Configuration")?;
    ui.println("")?;

    match Config::load() {
        Ok(config) => {
            let mut table = crate::cmd::ui::Table::new(vec![
                "Setting".to_string(),
                "Value".to_string(),
            ]);

            table.add_row(vec!["Default Profile".to_string(), config.default_profile.clone()]);
            table.add_row(vec!["Profiles".to_string(), config.profiles.len().to_string()]);
            
            if detailed {
                table.add_row(vec!["Auto Save Sessions".to_string(), config.global.auto_save_sessions.to_string()]);
                table.add_row(vec!["Session History Limit".to_string(), config.global.session_history_limit.to_string()]);
                table.add_row(vec!["Log Level".to_string(), config.global.log_level.clone()]);
                table.add_row(vec!["Theme".to_string(), config.global.theme.clone()]);
            }

            table.print(ui)?;
        }
        Err(e) => {
            ui.error(&format!("Failed to load configuration: {}", e))?;
        }
    }

    ui.println("")?;
    Ok(())
}

/// Display authentication status
async fn display_auth_status(ui: &mut UI, detailed: bool) -> Result<()> {
    ui.info("Authentication")?;
    ui.println("")?;

    let providers = vec![
        ("Anthropic", "ANTHROPIC_API_KEY"),
        ("OpenAI", "OPENAI_API_KEY"),
        ("Google", "GOOGLE_API_KEY"),
        ("GitHub", "GITHUB_TOKEN"),
    ];

    let mut table = crate::cmd::ui::Table::new(vec![
        "Provider".to_string(),
        "Environment".to_string(),
        "Stored".to_string(),
    ]);

    for (provider_name, env_var) in providers {
        let env_status = if std::env::var(env_var).is_ok() {
            "✓".to_string()
        } else {
            "✗".to_string()
        };

        // TODO: Check stored credentials
        let stored_status = "✗".to_string(); // Placeholder

        table.add_row(vec![provider_name.to_string(), env_status, stored_status]);
    }

    table.print(ui)?;
    ui.println("")?;

    Ok(())
}

/// Display recent activity
async fn display_recent_activity(ui: &mut UI) -> Result<()> {
    ui.info("Recent Activity")?;
    ui.println("")?;

    // TODO: Implement actual session/activity tracking
    ui.dim("No recent activity (feature not yet implemented)")?;
    ui.println("")?;

    Ok(())
}