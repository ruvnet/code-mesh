//! Init command implementation - Project initialization

use crate::cmd::{CliError, Result, UI, Config};
use std::fs;
use std::path::{Path, PathBuf};

/// Execute the init command
pub async fn execute(path: &str) -> Result<()> {
    let mut ui = UI::new();
    let project_path = PathBuf::from(path);
    
    ui.print_logo()?;
    ui.info(&format!("Initializing Code Mesh project in: {}", project_path.display()))?;
    ui.println("")?;

    // Validate path
    if !project_path.exists() {
        return Err(CliError::FileSystem(format!(
            "Path does not exist: {}",
            project_path.display()
        )));
    }

    if !project_path.is_dir() {
        return Err(CliError::FileSystem(format!(
            "Path is not a directory: {}",
            project_path.display()
        )));
    }

    // Detect project type
    let project_type = crate::cmd::utils::ProjectDetector::detect_project_type(&project_path);
    let project_name = crate::cmd::utils::ProjectDetector::get_project_name(&project_path);

    ui.info(&format!("Project name: {}", project_name))?;
    if let Some(ref ptype) = project_type {
        ui.info(&format!("Detected project type: {}", ptype))?;
    } else {
        ui.info("Project type: Generic")?;
    }

    // Check if already initialized
    let config_path = project_path.join(".code-mesh");
    if config_path.exists() {
        let overwrite = ui.confirm(
            "Code Mesh is already initialized in this directory. Overwrite?",
            false,
        )?;
        
        if !overwrite {
            ui.info("Initialization cancelled")?;
            return Ok(());
        }
    }

    ui.println("")?;

    // Create project configuration
    let progress = ui.progress_bar(5, "Initializing project...");
    
    // Step 1: Create .code-mesh directory
    progress.set_message("Creating configuration directory...");
    create_config_directory(&project_path)?;
    progress.inc(1);

    // Step 2: Create project configuration
    progress.set_message("Creating project configuration...");
    create_project_config(&project_path, &project_name, project_type.as_deref())?;
    progress.inc(1);

    // Step 3: Create gitignore entries
    progress.set_message("Updating .gitignore...");
    update_gitignore(&project_path)?;
    progress.inc(1);

    // Step 4: Create templates
    progress.set_message("Creating templates...");
    create_templates(&project_path, project_type.as_deref())?;
    progress.inc(1);

    // Step 5: Initialize global config if needed
    progress.set_message("Checking global configuration...");
    ensure_global_config()?;
    progress.inc(1);

    progress.finish_with_message("Project initialization complete!");

    ui.println("")?;
    ui.success("Code Mesh project initialized successfully!")?;
    ui.println("")?;

    // Show next steps
    show_next_steps(&mut ui, &project_path)?;

    Ok(())
}

/// Create the .code-mesh configuration directory
fn create_config_directory(project_path: &Path) -> Result<()> {
    let config_dir = project_path.join(".code-mesh");
    
    if config_dir.exists() {
        fs::remove_dir_all(&config_dir)
            .map_err(|e| CliError::FileSystem(format!("Failed to remove existing config: {}", e)))?;
    }

    fs::create_dir_all(&config_dir)
        .map_err(|e| CliError::FileSystem(format!("Failed to create config directory: {}", e)))?;

    // Create subdirectories
    let subdirs = ["sessions", "templates", "cache"];
    for subdir in &subdirs {
        fs::create_dir_all(config_dir.join(subdir))
            .map_err(|e| CliError::FileSystem(format!("Failed to create {}: {}", subdir, e)))?;
    }

    Ok(())
}

/// Create project configuration file
fn create_project_config(
    project_path: &Path,
    project_name: &str,
    project_type: Option<&str>,
) -> Result<()> {
    let config = ProjectConfig {
        name: project_name.to_string(),
        project_type: project_type.map(|s| s.to_string()),
        version: "1.0.0".to_string(),
        created_at: chrono::Utc::now(),
        settings: ProjectSettings {
            default_model: None,
            default_provider: None,
            auto_save_sessions: true,
            session_history_limit: 50,
            include_git_context: true,
            exclude_patterns: default_exclude_patterns(project_type),
            custom_prompts: std::collections::HashMap::new(),
        },
        tools: ToolConfig {
            enabled: vec![
                "code_analysis".to_string(),
                "file_operations".to_string(),
                "git_integration".to_string(),
                "web_search".to_string(),
            ],
            disabled: vec![],
            custom: std::collections::HashMap::new(),
        },
    };

    let config_path = project_path.join(".code-mesh").join("project.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| CliError::Json(e))?;

    fs::write(&config_path, config_json)
        .map_err(|e| CliError::FileSystem(format!("Failed to write project config: {}", e)))?;

    Ok(())
}

/// Update .gitignore file
fn update_gitignore(project_path: &Path) -> Result<()> {
    let gitignore_path = project_path.join(".gitignore");
    
    let gitignore_entries = vec![
        "# Code Mesh",
        ".code-mesh/sessions/",
        ".code-mesh/cache/",
        "*.code-mesh.tmp",
    ];

    let mut existing_content = String::new();
    if gitignore_path.exists() {
        existing_content = fs::read_to_string(&gitignore_path)
            .map_err(|e| CliError::FileSystem(format!("Failed to read .gitignore: {}", e)))?;
    }

    // Check if Code Mesh entries already exist
    if existing_content.contains("# Code Mesh") {
        return Ok(()); // Already configured
    }

    // Append Code Mesh entries
    if !existing_content.is_empty() && !existing_content.ends_with('\n') {
        existing_content.push('\n');
    }
    
    existing_content.push('\n');
    for entry in gitignore_entries {
        existing_content.push_str(entry);
        existing_content.push('\n');
    }

    fs::write(&gitignore_path, existing_content)
        .map_err(|e| CliError::FileSystem(format!("Failed to update .gitignore: {}", e)))?;

    Ok(())
}

/// Create template files (placeholder implementation)
fn create_templates(project_path: &Path, _project_type: Option<&str>) -> Result<()> {
    let templates_dir = project_path.join(".code-mesh").join("templates");

    // Create basic template files
    let basic_template = r#"# Code Review Template

## Summary
Brief description of the changes

## Changes Made
- Change 1
- Change 2
- Change 3

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
"#;

    fs::write(templates_dir.join("code_review.md"), basic_template)
        .map_err(|e| CliError::FileSystem(format!("Failed to create template: {}", e)))?;

    Ok(())
}

/// Ensure global configuration exists
fn ensure_global_config() -> Result<()> {
    let global_config = Config::load();
    match global_config {
        Ok(_) => Ok(()), // Global config already exists
        Err(_) => {
            // Create default global config
            let config = Config::default();
            config.save()?;
            Ok(())
        }
    }
}

/// Show next steps to the user
fn show_next_steps(ui: &mut UI, project_path: &Path) -> Result<()> {
    ui.info("Next steps:")?;
    ui.println("")?;
    
    ui.dim("1. Set up authentication:")?;
    ui.dim("   code-mesh auth login")?;
    ui.println("")?;
    
    ui.dim("2. Start a conversation:")?;
    ui.dim("   code-mesh run \"Help me understand this codebase\"")?;
    ui.println("")?;
    
    ui.dim("3. Use different modes:")?;
    ui.dim("   code-mesh run --mode code \"Add error handling to this function\"")?;
    ui.dim("   code-mesh run --mode review \"Review my latest changes\"")?;
    ui.println("")?;
    
    ui.dim("4. Start the API server:")?;
    ui.dim("   code-mesh serve")?;
    ui.println("")?;

    // Show project-specific tips
    let config_path = project_path.join(".code-mesh").join("project.json");
    ui.dim(&format!("Project configuration: {}", config_path.display()))?;
    
    Ok(())
}

/// Get default exclude patterns for different project types
fn default_exclude_patterns(project_type: Option<&str>) -> Vec<String> {
    let mut patterns = vec![
        "node_modules/".to_string(),
        ".git/".to_string(),
        "target/".to_string(),
        "build/".to_string(),
        "dist/".to_string(),
        "*.log".to_string(),
        "*.tmp".to_string(),
        ".env".to_string(),
        ".env.local".to_string(),
    ];

    match project_type {
        Some("rust") => {
            patterns.extend_from_slice(&[
                "Cargo.lock".to_string(),
                "target/".to_string(),
            ]);
        }
        Some("javascript") | Some("typescript") => {
            patterns.extend_from_slice(&[
                "package-lock.json".to_string(),
                "yarn.lock".to_string(),
                "node_modules/".to_string(),
            ]);
        }
        Some("python") => {
            patterns.extend_from_slice(&[
                "__pycache__/".to_string(),
                "*.pyc".to_string(),
                ".venv/".to_string(),
                "venv/".to_string(),
            ]);
        }
        _ => {}
    }

    patterns
}

// Configuration types

#[derive(serde::Serialize, serde::Deserialize)]
struct ProjectConfig {
    name: String,
    project_type: Option<String>,
    version: String,
    created_at: chrono::DateTime<chrono::Utc>,
    settings: ProjectSettings,
    tools: ToolConfig,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ProjectSettings {
    default_model: Option<String>,
    default_provider: Option<String>,
    auto_save_sessions: bool,
    session_history_limit: u32,
    include_git_context: bool,
    exclude_patterns: Vec<String>,
    custom_prompts: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ToolConfig {
    enabled: Vec<String>,
    disabled: Vec<String>,
    custom: std::collections::HashMap<String, serde_json::Value>,
}