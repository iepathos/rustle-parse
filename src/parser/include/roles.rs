use crate::parser::error::ParseError;
use crate::parser::include::{IncludeContext, RoleIncludeSpec};
use crate::types::parsed::{ParsedRole, ParsedTask};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Handler for role include/import functionality
pub struct RoleIncludeProcessor;

impl RoleIncludeProcessor {
    /// Process include_role directive
    pub async fn include_role(
        role_spec: &RoleIncludeSpec,
        context: &IncludeContext,
    ) -> Result<RoleIncludeResult, ParseError> {
        Self::validate_role_spec(role_spec)?;

        let role_path = Self::resolve_role_path(&role_spec.name, &context.current_file)?;

        let mut result = RoleIncludeResult::new(role_spec.name.clone());

        // Load role components based on specification
        if let Some(tasks_from) = &role_spec.tasks_from {
            let tasks = Self::load_role_tasks(&role_path, tasks_from, context).await?;
            result.tasks = tasks;
        } else {
            // Load default main.yml if no specific tasks_from specified
            let tasks = Self::load_default_role_tasks(&role_path, context).await?;
            result.tasks = tasks;
        }

        // Load role variables
        if let Some(vars_from) = &role_spec.vars_from {
            let vars = Self::load_role_vars(&role_path, vars_from, context).await?;
            result.vars.extend(vars);
        } else {
            // Load default vars/main.yml
            let vars = Self::load_default_role_vars(&role_path, context).await?;
            result.vars.extend(vars);
        }

        // Load role defaults
        if let Some(defaults_from) = &role_spec.defaults_from {
            let defaults = Self::load_role_defaults(&role_path, defaults_from, context).await?;
            // Defaults have lower precedence than vars
            for (key, value) in defaults {
                result.vars.entry(key).or_insert(value);
            }
        } else {
            // Load default defaults/main.yml
            let defaults = Self::load_default_role_defaults(&role_path, context).await?;
            for (key, value) in defaults {
                result.vars.entry(key).or_insert(value);
            }
        }

        // Load role handlers if specified
        if let Some(handlers_from) = &role_spec.handlers_from {
            let handlers = Self::load_role_handlers(&role_path, handlers_from, context).await?;
            result.handlers = handlers;
        } else {
            // Load default handlers/main.yml
            let handlers = Self::load_default_role_handlers(&role_path, context).await?;
            result.handlers = handlers;
        }

        // Merge role spec variables (highest precedence)
        if let Some(spec_vars) = &role_spec.vars {
            result.vars.extend(spec_vars.clone());
        }

        // Apply tags
        if let Some(tags) = &role_spec.tags {
            result.tags.extend(tags.clone());
        }

        // Apply when conditions to all tasks and handlers
        if let Some(when_condition) = &role_spec.when_condition {
            result.tasks = Self::apply_when_to_tasks(result.tasks, when_condition);
            result.handlers = Self::apply_when_to_tasks(result.handlers, when_condition);
        }

        // Apply apply block if present
        if let Some(apply_spec) = &role_spec.apply {
            if let Some(apply_tags) = &apply_spec.tags {
                result.tags.extend(apply_tags.clone());
                // Apply tags to all tasks and handlers
                for task in &mut result.tasks {
                    task.tags.extend(apply_tags.clone());
                }
                for handler in &mut result.handlers {
                    handler.tags.extend(apply_tags.clone());
                }
            }

            if let Some(apply_when) = &apply_spec.when_condition {
                result.tasks = Self::apply_when_to_tasks(result.tasks, apply_when);
                result.handlers = Self::apply_when_to_tasks(result.handlers, apply_when);
            }
        }

        Ok(result)
    }

    /// Process import_role directive
    pub async fn import_role(
        role_spec: &RoleIncludeSpec,
        context: &IncludeContext,
    ) -> Result<RoleIncludeResult, ParseError> {
        // Import role is similar to include role but processed at parse time
        Self::include_role(role_spec, context).await
    }

    /// Validate role specification
    fn validate_role_spec(role_spec: &RoleIncludeSpec) -> Result<(), ParseError> {
        if role_spec.name.is_empty() {
            return Err(ParseError::InvalidIncludeDirective {
                message: "Role name cannot be empty".to_string(),
            });
        }

        // Validate role name format
        if !Self::is_valid_role_name(&role_spec.name) {
            return Err(ParseError::InvalidIncludeDirective {
                message: format!("Invalid role name '{}'. Role names must contain only letters, numbers, underscores, and hyphens.", role_spec.name),
            });
        }

        Ok(())
    }

    /// Check if role name is valid
    fn is_valid_role_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Resolve role path from role name
    fn resolve_role_path(role_name: &str, current_file: &Path) -> Result<PathBuf, ParseError> {
        let current_dir = current_file.parent().unwrap_or_else(|| Path::new("."));

        // Try multiple locations for roles following Ansible conventions
        let search_paths = vec![
            // Local roles directory relative to current file
            current_dir.join("roles"),
            // Parent directory roles (common in Ansible project structures)
            current_dir.join("..").join("roles"),
            // Current directory (for role-specific layouts)
            current_dir.to_path_buf(),
            // System-wide Ansible roles
            PathBuf::from("/etc/ansible/roles"),
            // User Ansible roles
            PathBuf::from("~/.ansible/roles"), // TODO: Expand ~ properly
        ];

        for search_path in &search_paths {
            let role_path = search_path.join(role_name);
            if role_path.exists() && role_path.is_dir() && Self::is_valid_role_directory(&role_path)
            {
                return Ok(role_path);
            }
        }

        Err(ParseError::RoleNotFound {
            role: role_name.to_string(),
            searched_paths: search_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        })
    }

    /// Check if directory looks like a valid Ansible role
    fn is_valid_role_directory(role_path: &Path) -> bool {
        // A valid role should have at least one of these directories
        let role_dirs = [
            "tasks",
            "handlers",
            "templates",
            "files",
            "vars",
            "defaults",
            "meta",
        ];

        role_dirs.iter().any(|dir| {
            let dir_path = role_path.join(dir);
            dir_path.exists() && dir_path.is_dir()
        })
    }

    /// Load role tasks from specific file
    async fn load_role_tasks(
        role_path: &Path,
        tasks_from: &str,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        let tasks_path = role_path.join("tasks").join(format!("{tasks_from}.yml"));
        if !tasks_path.exists() {
            let tasks_path_yaml = role_path.join("tasks").join(format!("{tasks_from}.yaml"));
            if !tasks_path_yaml.exists() {
                return Err(ParseError::IncludeFileNotFound {
                    file: tasks_path.to_string_lossy().to_string(),
                });
            }
            return Self::parse_tasks_file(&tasks_path_yaml, context).await;
        }
        Self::parse_tasks_file(&tasks_path, context).await
    }

    /// Load default role tasks (tasks/main.yml)
    async fn load_default_role_tasks(
        role_path: &Path,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        let main_tasks = role_path.join("tasks").join("main.yml");
        if main_tasks.exists() {
            Self::parse_tasks_file(&main_tasks, context).await
        } else {
            let main_tasks_yaml = role_path.join("tasks").join("main.yaml");
            if main_tasks_yaml.exists() {
                Self::parse_tasks_file(&main_tasks_yaml, context).await
            } else {
                Ok(Vec::new()) // No tasks file found, return empty
            }
        }
    }

    /// Load role variables from specific file
    async fn load_role_vars(
        role_path: &Path,
        vars_from: &str,
        _context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let vars_path = role_path.join("vars").join(format!("{vars_from}.yml"));
        if !vars_path.exists() {
            let vars_path_yaml = role_path.join("vars").join(format!("{vars_from}.yaml"));
            if !vars_path_yaml.exists() {
                return Ok(HashMap::new());
            }
            return Self::parse_vars_file(&vars_path_yaml).await;
        }
        Self::parse_vars_file(&vars_path).await
    }

    /// Load default role variables (vars/main.yml)
    async fn load_default_role_vars(
        role_path: &Path,
        _context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let main_vars = role_path.join("vars").join("main.yml");
        if main_vars.exists() {
            Self::parse_vars_file(&main_vars).await
        } else {
            let main_vars_yaml = role_path.join("vars").join("main.yaml");
            if main_vars_yaml.exists() {
                Self::parse_vars_file(&main_vars_yaml).await
            } else {
                Ok(HashMap::new())
            }
        }
    }

    /// Load role defaults from specific file
    async fn load_role_defaults(
        role_path: &Path,
        defaults_from: &str,
        _context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let defaults_path = role_path
            .join("defaults")
            .join(format!("{defaults_from}.yml"));
        if !defaults_path.exists() {
            let defaults_path_yaml = role_path
                .join("defaults")
                .join(format!("{defaults_from}.yaml"));
            if !defaults_path_yaml.exists() {
                return Ok(HashMap::new());
            }
            return Self::parse_vars_file(&defaults_path_yaml).await;
        }
        Self::parse_vars_file(&defaults_path).await
    }

    /// Load default role defaults (defaults/main.yml)
    async fn load_default_role_defaults(
        role_path: &Path,
        _context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let main_defaults = role_path.join("defaults").join("main.yml");
        if main_defaults.exists() {
            Self::parse_vars_file(&main_defaults).await
        } else {
            let main_defaults_yaml = role_path.join("defaults").join("main.yaml");
            if main_defaults_yaml.exists() {
                Self::parse_vars_file(&main_defaults_yaml).await
            } else {
                Ok(HashMap::new())
            }
        }
    }

    /// Load role handlers from specific file
    async fn load_role_handlers(
        role_path: &Path,
        handlers_from: &str,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        let handlers_path = role_path
            .join("handlers")
            .join(format!("{handlers_from}.yml"));
        if !handlers_path.exists() {
            let handlers_path_yaml = role_path
                .join("handlers")
                .join(format!("{handlers_from}.yaml"));
            if !handlers_path_yaml.exists() {
                return Ok(Vec::new());
            }
            return Self::parse_tasks_file(&handlers_path_yaml, context).await;
        }
        Self::parse_tasks_file(&handlers_path, context).await
    }

    /// Load default role handlers (handlers/main.yml)
    async fn load_default_role_handlers(
        role_path: &Path,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        let main_handlers = role_path.join("handlers").join("main.yml");
        if main_handlers.exists() {
            Self::parse_tasks_file(&main_handlers, context).await
        } else {
            let main_handlers_yaml = role_path.join("handlers").join("main.yaml");
            if main_handlers_yaml.exists() {
                Self::parse_tasks_file(&main_handlers_yaml, context).await
            } else {
                Ok(Vec::new())
            }
        }
    }

    /// Parse tasks from a YAML file
    async fn parse_tasks_file(
        file_path: &Path,
        _context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        let content =
            fs::read_to_string(file_path)
                .await
                .map_err(|_| ParseError::IncludeFileNotFound {
                    file: file_path.to_string_lossy().to_string(),
                })?;

        // Parse as YAML array of tasks
        let raw_tasks: Vec<serde_yaml::Value> =
            serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

        let mut parsed_tasks = Vec::new();
        for (index, _raw_task_value) in raw_tasks.into_iter().enumerate() {
            // This is a simplified task parser - in a full implementation,
            // this would use the full task parsing logic
            let task = ParsedTask {
                id: format!("role_task_{index}"),
                name: format!("Role task {index}"),
                module: "placeholder".to_string(),
                args: HashMap::new(),
                vars: HashMap::new(),
                when: None,
                loop_items: None,
                tags: Vec::new(),
                notify: Vec::new(),
                changed_when: None,
                failed_when: None,
                ignore_errors: false,
                delegate_to: None,
                dependencies: Vec::new(),
            };
            parsed_tasks.push(task);
        }

        Ok(parsed_tasks)
    }

    /// Parse variables from a YAML file
    async fn parse_vars_file(
        file_path: &Path,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let content =
            fs::read_to_string(file_path)
                .await
                .map_err(|_| ParseError::IncludeFileNotFound {
                    file: file_path.to_string_lossy().to_string(),
                })?;

        let vars: HashMap<String, serde_json::Value> =
            serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

        Ok(vars)
    }

    /// Apply when condition to a list of tasks
    fn apply_when_to_tasks(mut tasks: Vec<ParsedTask>, when_condition: &str) -> Vec<ParsedTask> {
        for task in &mut tasks {
            if let Some(existing_when) = &task.when {
                task.when = Some(format!("({existing_when}) and ({when_condition})"));
            } else {
                task.when = Some(when_condition.to_string());
            }
        }
        tasks
    }
}

/// Result of role include processing
#[derive(Debug, Clone)]
pub struct RoleIncludeResult {
    pub name: String,
    pub tasks: Vec<ParsedTask>,
    pub handlers: Vec<ParsedTask>,
    pub vars: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
}

impl RoleIncludeResult {
    fn new(name: String) -> Self {
        Self {
            name,
            tasks: Vec::new(),
            handlers: Vec::new(),
            vars: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Convert to ParsedRole for compatibility
    pub fn to_parsed_role(&self) -> ParsedRole {
        ParsedRole {
            name: self.name.clone(),
            src: None,
            version: None,
            vars: self.vars.clone(),
            tags: self.tags.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_role_spec() {
        // Valid role spec
        let valid_spec = RoleIncludeSpec {
            name: "web_server".to_string(),
            tasks_from: None,
            vars_from: None,
            defaults_from: None,
            handlers_from: None,
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
        };
        assert!(RoleIncludeProcessor::validate_role_spec(&valid_spec).is_ok());

        // Invalid empty name
        let invalid_spec = RoleIncludeSpec {
            name: "".to_string(),
            tasks_from: None,
            vars_from: None,
            defaults_from: None,
            handlers_from: None,
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
        };
        assert!(RoleIncludeProcessor::validate_role_spec(&invalid_spec).is_err());

        // Invalid characters in name
        let invalid_char_spec = RoleIncludeSpec {
            name: "web@server".to_string(),
            tasks_from: None,
            vars_from: None,
            defaults_from: None,
            handlers_from: None,
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
        };
        assert!(RoleIncludeProcessor::validate_role_spec(&invalid_char_spec).is_err());
    }

    #[test]
    fn test_is_valid_role_name() {
        assert!(RoleIncludeProcessor::is_valid_role_name("web_server"));
        assert!(RoleIncludeProcessor::is_valid_role_name("web-server"));
        assert!(RoleIncludeProcessor::is_valid_role_name("webserver123"));
        assert!(!RoleIncludeProcessor::is_valid_role_name("web@server"));
        assert!(!RoleIncludeProcessor::is_valid_role_name("web server"));
        assert!(!RoleIncludeProcessor::is_valid_role_name(""));
    }

    #[test]
    fn test_is_valid_role_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Invalid - empty directory
        assert!(!RoleIncludeProcessor::is_valid_role_directory(
            temp_dir.path()
        ));

        // Valid - has tasks directory
        fs::create_dir_all(temp_dir.path().join("tasks")).unwrap();
        assert!(RoleIncludeProcessor::is_valid_role_directory(
            temp_dir.path()
        ));
    }

    #[tokio::test]
    async fn test_resolve_role_path() {
        let temp_dir = TempDir::new().unwrap();

        // Create role directory structure
        let role_path = temp_dir.path().join("roles/web_server");
        fs::create_dir_all(role_path.join("tasks")).unwrap();
        fs::write(role_path.join("tasks/main.yml"), "").unwrap();

        let current_file = temp_dir.path().join("playbook.yml");
        let resolved =
            RoleIncludeProcessor::resolve_role_path("web_server", &current_file).unwrap();

        assert!(resolved.ends_with("roles/web_server"));
        assert!(resolved.join("tasks").exists());
    }

    #[tokio::test]
    async fn test_load_default_role_vars() {
        let temp_dir = TempDir::new().unwrap();
        let role_path = temp_dir.path().join("web_server");

        // Create role vars
        fs::create_dir_all(role_path.join("vars")).unwrap();
        fs::write(
            role_path.join("vars/main.yml"),
            "web_server_port: 80\nweb_server_name: nginx",
        )
        .unwrap();

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: PathBuf::from("playbook.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let vars = RoleIncludeProcessor::load_default_role_vars(&role_path, &context)
            .await
            .unwrap();

        assert_eq!(vars["web_server_port"], serde_json::json!(80));
        assert_eq!(vars["web_server_name"], serde_json::json!("nginx"));
    }

    #[test]
    fn test_apply_when_to_tasks() {
        let tasks = vec![
            ParsedTask {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                module: "debug".to_string(),
                args: HashMap::new(),
                vars: HashMap::new(),
                when: None,
                loop_items: None,
                tags: Vec::new(),
                notify: Vec::new(),
                changed_when: None,
                failed_when: None,
                ignore_errors: false,
                delegate_to: None,
                dependencies: Vec::new(),
            },
            ParsedTask {
                id: "task2".to_string(),
                name: "Task 2".to_string(),
                module: "debug".to_string(),
                args: HashMap::new(),
                vars: HashMap::new(),
                when: Some("existing_condition".to_string()),
                loop_items: None,
                tags: Vec::new(),
                notify: Vec::new(),
                changed_when: None,
                failed_when: None,
                ignore_errors: false,
                delegate_to: None,
                dependencies: Vec::new(),
            },
        ];

        let result = RoleIncludeProcessor::apply_when_to_tasks(tasks, "role_condition");

        assert_eq!(result[0].when, Some("role_condition".to_string()));
        assert_eq!(
            result[1].when,
            Some("(existing_condition) and (role_condition)".to_string())
        );
    }
}
