use crate::parser::error::ParseError;
use crate::parser::include::{IncludeContext, IncludeVarsSpec};
use crate::parser::template::TemplateEngine;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Handler for include_vars functionality
pub struct VariableIncludeProcessor {
    template_engine: TemplateEngine,
}

impl VariableIncludeProcessor {
    pub fn new(template_engine: TemplateEngine) -> Self {
        Self { template_engine }
    }

    /// Process include_vars directive
    pub async fn include_vars(
        &self,
        vars_spec: &IncludeVarsSpec,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        // Check if include should be processed based on when condition
        if !self.should_process_vars(vars_spec, context)? {
            return Ok(HashMap::new());
        }

        if let Some(file) = &vars_spec.file {
            // Single file include
            self.include_vars_from_file(file, context).await
        } else if let Some(_dir) = &vars_spec.dir {
            // Directory include
            self.include_vars_from_dir(vars_spec, context).await
        } else {
            Err(ParseError::InvalidIncludeDirective {
                message: "include_vars requires either 'file' or 'dir' parameter".to_string(),
            })
        }
    }

    /// Include variables from a single file
    async fn include_vars_from_file(
        &self,
        file_path: &str,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let resolved_path = self.resolve_vars_file_path(file_path, &context.current_file)?;
        let content = fs::read_to_string(&resolved_path).await.map_err(|_| {
            ParseError::IncludeFileNotFound {
                file: resolved_path.to_string_lossy().to_string(),
            }
        })?;

        self.parse_vars_file_content(&content, &resolved_path, context)
    }

    /// Include variables from a directory
    async fn include_vars_from_dir(
        &self,
        vars_spec: &IncludeVarsSpec,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let dir_path = vars_spec.dir.as_ref().unwrap();
        let resolved_dir = self.resolve_vars_file_path(dir_path, &context.current_file)?;

        if !resolved_dir.is_dir() {
            return Err(ParseError::IncludeFileNotFound {
                file: resolved_dir.to_string_lossy().to_string(),
            });
        }

        let mut all_vars = HashMap::new();
        let max_depth = vars_spec.depth.unwrap_or(1);
        let extensions = vars_spec
            .extensions
            .as_ref()
            .cloned()
            .unwrap_or_else(|| vec!["yml".to_string(), "yaml".to_string(), "json".to_string()]);

        let files_matching = vars_spec
            .files_matching
            .as_ref()
            .map(|pattern| Regex::new(pattern).unwrap_or_else(|_| Regex::new(".*").unwrap()));

        let ignore_files = vars_spec.ignore_files.as_ref().cloned().unwrap_or_default();

        self.load_vars_recursive(
            &resolved_dir,
            &mut all_vars,
            0,
            max_depth,
            &extensions,
            &files_matching,
            &ignore_files,
            context,
        )
        .await?;

        Ok(all_vars)
    }

    /// Recursively load variables from directory
    #[allow(clippy::too_many_arguments)]
    fn load_vars_recursive<'a>(
        &'a self,
        dir: &'a Path,
        vars: &'a mut HashMap<String, serde_json::Value>,
        current_depth: usize,
        max_depth: usize,
        extensions: &'a [String],
        files_matching: &'a Option<Regex>,
        ignore_files: &'a [String],
        context: &'a IncludeContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ParseError>> + 'a>> {
        Box::pin(async move {
            if current_depth >= max_depth {
                return Ok(());
            }

            let mut entries = fs::read_dir(dir).await.map_err(ParseError::Io)?;

            while let Some(entry) = entries.next_entry().await.map_err(ParseError::Io)? {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip ignored files
                if ignore_files.contains(&file_name.to_string()) {
                    continue;
                }

                if path.is_dir() && current_depth + 1 < max_depth {
                    self.load_vars_recursive(
                        &path,
                        vars,
                        current_depth + 1,
                        max_depth,
                        extensions,
                        files_matching,
                        ignore_files,
                        context,
                    )
                    .await?;
                } else if path.is_file() {
                    // Check file extension
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if !extensions.contains(&ext.to_string()) {
                            continue;
                        }
                    }

                    // Check file matching pattern
                    if let Some(pattern) = files_matching {
                        if !pattern.is_match(file_name) {
                            continue;
                        }
                    }

                    // Load variables from this file
                    let file_vars = self.load_vars_from_path(&path, context).await?;

                    // Merge variables (later files override earlier ones)
                    vars.extend(file_vars);
                }
            }

            Ok(())
        })
    }

    /// Load variables from a specific file path
    async fn load_vars_from_path(
        &self,
        path: &Path,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let content =
            fs::read_to_string(path)
                .await
                .map_err(|_| ParseError::IncludeFileNotFound {
                    file: path.to_string_lossy().to_string(),
                })?;

        self.parse_vars_file_content(&content, path, context)
    }

    /// Parse variable file content based on file extension
    fn parse_vars_file_content(
        &self,
        content: &str,
        file_path: &Path,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("yml");

        let vars: HashMap<String, serde_json::Value> = match extension {
            "json" => {
                // JSON variables file
                serde_json::from_str(content).map_err(ParseError::Json)?
            }
            "yml" | "yaml" => {
                // YAML variables file
                serde_yaml::from_str(content).map_err(ParseError::Yaml)?
            }
            _ => {
                return Err(ParseError::InvalidIncludeDirective {
                    message: format!("Unsupported variable file extension: {extension}"),
                });
            }
        };

        // Process variables through template engine
        let mut processed_vars = HashMap::new();
        for (key, value) in vars {
            let processed_value = self
                .template_engine
                .render_value(&value, &context.variables)?;
            processed_vars.insert(key, processed_value);
        }

        Ok(processed_vars)
    }

    /// Check if variables should be included based on when condition
    fn should_process_vars(
        &self,
        vars_spec: &IncludeVarsSpec,
        context: &IncludeContext,
    ) -> Result<bool, ParseError> {
        if let Some(when_condition) = &vars_spec.when_condition {
            // Evaluate the when condition using template engine
            let result = self
                .template_engine
                .render_string(&format!("{{{{ {when_condition} }}}}"), &context.variables)?;

            // Parse result as boolean
            match result.trim().to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(true),
                "false" | "no" | "0" => Ok(false),
                "" => Ok(false),
                _ => Ok(!result.trim().is_empty()),
            }
        } else {
            Ok(true)
        }
    }

    /// Resolve variable file path relative to current context
    fn resolve_vars_file_path(
        &self,
        file_path: &str,
        current_file: &Path,
    ) -> Result<PathBuf, ParseError> {
        let path = Path::new(file_path);

        if path.is_absolute() {
            // For now, reject absolute paths for security
            return Err(ParseError::SecurityViolation {
                message: format!("Absolute paths not allowed in include_vars: {file_path}"),
            });
        }

        // Resolve relative to current file's directory
        let current_dir = current_file.parent().unwrap_or_else(|| Path::new("."));
        let resolved = current_dir.join(path);

        // Canonicalize if possible
        resolved
            .canonicalize()
            .or_else(|_| Ok(resolved.clone()))
            .map_err(|_: std::io::Error| ParseError::IncludeFileNotFound {
                file: resolved.to_string_lossy().to_string(),
            })
    }

    /// Validate variable names according to Ansible conventions
    pub fn validate_variable_names(
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<(), ParseError> {
        let var_name_regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();

        for key in vars.keys() {
            if !var_name_regex.is_match(key) {
                return Err(ParseError::InvalidVariableSyntax {
                    line: 0, // Line number not available in this context
                    message: format!(
                        "Invalid variable name '{key}'. Variable names must start with a letter or underscore and contain only letters, numbers, and underscores."
                    ),
                });
            }

            // Check for reserved variable names
            if Self::is_reserved_variable_name(key) {
                return Err(ParseError::InvalidVariableSyntax {
                    line: 0,
                    message: format!("'{key}' is a reserved variable name"),
                });
            }
        }

        Ok(())
    }

    /// Check if a variable name is reserved
    fn is_reserved_variable_name(name: &str) -> bool {
        matches!(
            name,
            "ansible_facts"
                | "ansible_connection"
                | "ansible_host"
                | "ansible_port"
                | "ansible_user"
                | "ansible_ssh_pass"
                | "ansible_ssh_private_key_file"
                | "ansible_become"
                | "ansible_become_method"
                | "ansible_become_user"
                | "ansible_become_pass"
                | "inventory_hostname"
                | "inventory_hostname_short"
                | "group_names"
                | "groups"
                | "hostvars"
                | "play_hosts"
                | "ansible_play_hosts"
                | "ansible_version"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_include_vars_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let processor = VariableIncludeProcessor::new(TemplateEngine::new());

        // Create variables file
        let vars_content = r#"
app_name: myapp
app_version: "1.0.0"
debug_mode: true
"#;
        fs::write(temp_dir.path().join("vars.yml"), vars_content).unwrap();

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("playbook.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let vars = processor
            .include_vars_from_file("vars.yml", &context)
            .await
            .unwrap();

        assert_eq!(vars["app_name"], serde_json::json!("myapp"));
        assert_eq!(vars["app_version"], serde_json::json!("1.0.0"));
        assert_eq!(vars["debug_mode"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn test_include_vars_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let processor = VariableIncludeProcessor::new(TemplateEngine::new());

        // Create directory structure with variable files
        fs::create_dir_all(temp_dir.path().join("vars")).unwrap();
        fs::write(
            temp_dir.path().join("vars/common.yml"),
            "common_var: common_value",
        )
        .unwrap();
        fs::write(temp_dir.path().join("vars/app.yml"), "app_var: app_value").unwrap();

        let vars_spec = IncludeVarsSpec {
            file: None,
            dir: Some("vars".to_string()),
            name: None,
            depth: Some(1),
            files_matching: None,
            ignore_files: None,
            extensions: Some(vec!["yml".to_string()]),
            when_condition: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("playbook.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let vars = processor
            .include_vars_from_dir(&vars_spec, &context)
            .await
            .unwrap();

        assert!(vars.contains_key("common_var"));
        assert!(vars.contains_key("app_var"));
    }

    #[tokio::test]
    async fn test_include_vars_with_pattern_matching() {
        let temp_dir = TempDir::new().unwrap();
        let processor = VariableIncludeProcessor::new(TemplateEngine::new());

        // Create files with different names
        fs::create_dir_all(temp_dir.path().join("vars")).unwrap();
        fs::write(
            temp_dir.path().join("vars/prod_vars.yml"),
            "env: production",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("vars/dev_vars.yml"),
            "env: development",
        )
        .unwrap();
        fs::write(temp_dir.path().join("vars/config.yml"), "type: config").unwrap();

        let vars_spec = IncludeVarsSpec {
            file: None,
            dir: Some("vars".to_string()),
            name: None,
            depth: Some(1),
            files_matching: Some(".*_vars\\.yml".to_string()),
            ignore_files: None,
            extensions: Some(vec!["yml".to_string()]),
            when_condition: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("playbook.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let vars = processor
            .include_vars_from_dir(&vars_spec, &context)
            .await
            .unwrap();

        // Should include files matching pattern but not config.yml
        assert!(vars.contains_key("env"));
        assert!(!vars.contains_key("type"));
    }

    #[test]
    fn test_validate_variable_names() {
        let mut valid_vars = HashMap::new();
        valid_vars.insert("valid_var".to_string(), serde_json::json!("value"));
        valid_vars.insert("_private_var".to_string(), serde_json::json!("value"));
        valid_vars.insert("var123".to_string(), serde_json::json!("value"));

        assert!(VariableIncludeProcessor::validate_variable_names(&valid_vars).is_ok());

        let mut invalid_vars = HashMap::new();
        invalid_vars.insert("123invalid".to_string(), serde_json::json!("value"));

        assert!(VariableIncludeProcessor::validate_variable_names(&invalid_vars).is_err());

        let mut reserved_vars = HashMap::new();
        reserved_vars.insert("ansible_facts".to_string(), serde_json::json!("value"));

        assert!(VariableIncludeProcessor::validate_variable_names(&reserved_vars).is_err());
    }

    #[test]
    fn test_should_process_vars() {
        let processor = VariableIncludeProcessor::new(TemplateEngine::new());

        // Test without when condition
        let vars_spec = IncludeVarsSpec {
            file: Some("vars.yml".to_string()),
            dir: None,
            name: None,
            depth: None,
            files_matching: None,
            ignore_files: None,
            extensions: None,
            when_condition: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: PathBuf::from("playbook.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        assert!(processor.should_process_vars(&vars_spec, &context).unwrap());

        // Test with when condition
        let vars_spec_with_when = IncludeVarsSpec {
            file: Some("vars.yml".to_string()),
            dir: None,
            name: None,
            depth: None,
            files_matching: None,
            ignore_files: None,
            extensions: None,
            when_condition: Some("false".to_string()),
        };

        assert!(!processor
            .should_process_vars(&vars_spec_with_when, &context)
            .unwrap());
    }
}
