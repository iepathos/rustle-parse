use crate::parser::error::ParseError;
use crate::parser::include::{ImportSpec, IncludeContext, IncludeSpec};
use crate::types::parsed::ParsedTask;
use std::collections::HashMap;

/// Task-specific include/import functionality
pub struct TaskIncludeProcessor;

impl TaskIncludeProcessor {
    /// Process include_tasks with task-specific logic
    pub async fn process_include_tasks(
        include_spec: &IncludeSpec,
        _context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        // Task include processing is handled by the main IncludeHandler
        // This module provides task-specific validation and processing

        Self::validate_task_include_spec(include_spec)?;

        // Return placeholder - actual implementation delegated to IncludeHandler
        Ok(Vec::new())
    }

    /// Process import_tasks with task-specific logic
    pub async fn process_import_tasks(
        import_spec: &ImportSpec,
        _context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        Self::validate_task_import_spec(import_spec)?;

        // Return placeholder - actual implementation delegated to IncludeHandler
        Ok(Vec::new())
    }

    /// Validate include_tasks specification
    fn validate_task_include_spec(spec: &IncludeSpec) -> Result<(), ParseError> {
        // Validate file extension
        if !Self::is_valid_task_file(&spec.file) {
            return Err(ParseError::InvalidIncludeDirective {
                message: format!(
                    "Task include file must have .yml or .yaml extension: {}",
                    spec.file
                ),
            });
        }

        // Validate that delegate_to and run_once are not used together inappropriately
        if spec.delegate_to.is_some() && spec.run_once == Some(true) {
            return Err(ParseError::InvalidIncludeDirective {
                message: "delegate_to and run_once cannot be used together in include_tasks"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Validate import_tasks specification
    fn validate_task_import_spec(spec: &ImportSpec) -> Result<(), ParseError> {
        // Validate file extension
        if !Self::is_valid_task_file(&spec.file) {
            return Err(ParseError::InvalidIncludeDirective {
                message: format!(
                    "Task import file must have .yml or .yaml extension: {}",
                    spec.file
                ),
            });
        }

        // Import tasks has fewer allowed options than include
        // Most validation is structural and handled by serde

        Ok(())
    }

    /// Check if file has valid task file extension
    fn is_valid_task_file(filename: &str) -> bool {
        filename.ends_with(".yml") || filename.ends_with(".yaml")
    }

    /// Merge task-level variables with include variables
    pub fn merge_task_variables(
        task_vars: &HashMap<String, serde_json::Value>,
        include_vars: &HashMap<String, serde_json::Value>,
        context_vars: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut merged = context_vars.clone();

        // Include variables override context variables
        merged.extend(include_vars.clone());

        // Task variables override both
        merged.extend(task_vars.clone());

        merged
    }

    /// Apply task-specific include transformations
    pub fn apply_task_transformations(
        mut task: ParsedTask,
        include_spec: &IncludeSpec,
    ) -> Result<ParsedTask, ParseError> {
        // Apply include-level delegate_to
        if let Some(delegate_to) = &include_spec.delegate_to {
            task.delegate_to = Some(delegate_to.clone());
        }

        // Apply include-level tags
        if let Some(include_tags) = &include_spec.tags {
            task.tags.extend(include_tags.clone());
        }

        // Apply when condition logic
        if let Some(include_when) = &include_spec.when_condition {
            if let Some(existing_when) = &task.when {
                // Combine with AND logic
                task.when = Some(format!("({}) and ({})", existing_when, include_when));
            } else {
                task.when = Some(include_when.clone());
            }
        }

        // Apply apply block if present
        if let Some(apply_spec) = &include_spec.apply {
            if let Some(apply_tags) = &apply_spec.tags {
                task.tags.extend(apply_tags.clone());
            }

            if let Some(apply_when) = &apply_spec.when_condition {
                if let Some(existing_when) = &task.when {
                    task.when = Some(format!("({}) and ({})", existing_when, apply_when));
                } else {
                    task.when = Some(apply_when.clone());
                }
            }

            // Apply become settings (would need to extend ParsedTask to support these)
            // For now, these are noted but not implemented in the task structure
        }

        Ok(task)
    }

    /// Extract task metadata from include context
    pub fn extract_task_metadata(
        context: &IncludeContext,
        include_spec: &IncludeSpec,
    ) -> TaskIncludeMetadata {
        TaskIncludeMetadata {
            included_from: context.current_file.clone(),
            include_depth: context.include_depth,
            delegate_to: include_spec.delegate_to.clone(),
            run_once: include_spec.run_once.unwrap_or(false),
            delegate_facts: include_spec.delegate_facts.unwrap_or(false),
        }
    }
}

/// Metadata about task inclusion
#[derive(Debug, Clone)]
pub struct TaskIncludeMetadata {
    pub included_from: std::path::PathBuf,
    pub include_depth: usize,
    pub delegate_to: Option<String>,
    pub run_once: bool,
    pub delegate_facts: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validate_task_include_spec() {
        // Valid spec
        let valid_spec = IncludeSpec {
            file: "tasks/main.yml".to_string(),
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };
        assert!(TaskIncludeProcessor::validate_task_include_spec(&valid_spec).is_ok());

        // Invalid file extension
        let invalid_spec = IncludeSpec {
            file: "tasks/main.txt".to_string(),
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };
        assert!(TaskIncludeProcessor::validate_task_include_spec(&invalid_spec).is_err());

        // Invalid combination of delegate_to and run_once
        let invalid_combo_spec = IncludeSpec {
            file: "tasks/main.yml".to_string(),
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: Some("localhost".to_string()),
            delegate_facts: None,
            run_once: Some(true),
        };
        assert!(TaskIncludeProcessor::validate_task_include_spec(&invalid_combo_spec).is_err());
    }

    #[test]
    fn test_merge_task_variables() {
        let mut context_vars = HashMap::new();
        context_vars.insert("var1".to_string(), serde_json::json!("context"));
        context_vars.insert("var2".to_string(), serde_json::json!("context"));

        let mut include_vars = HashMap::new();
        include_vars.insert("var2".to_string(), serde_json::json!("include"));
        include_vars.insert("var3".to_string(), serde_json::json!("include"));

        let mut task_vars = HashMap::new();
        task_vars.insert("var3".to_string(), serde_json::json!("task"));
        task_vars.insert("var4".to_string(), serde_json::json!("task"));

        let merged =
            TaskIncludeProcessor::merge_task_variables(&task_vars, &include_vars, &context_vars);

        // Check precedence: task > include > context
        assert_eq!(merged["var1"], serde_json::json!("context"));
        assert_eq!(merged["var2"], serde_json::json!("include"));
        assert_eq!(merged["var3"], serde_json::json!("task"));
        assert_eq!(merged["var4"], serde_json::json!("task"));
    }

    #[test]
    fn test_apply_task_transformations() {
        let mut task = ParsedTask {
            id: "test_task".to_string(),
            name: "Test Task".to_string(),
            module: "debug".to_string(),
            args: HashMap::new(),
            vars: HashMap::new(),
            when: Some("existing_condition".to_string()),
            loop_items: None,
            tags: vec!["original".to_string()],
            notify: Vec::new(),
            changed_when: None,
            failed_when: None,
            ignore_errors: false,
            delegate_to: None,
            dependencies: Vec::new(),
        };

        let include_spec = IncludeSpec {
            file: "tasks/test.yml".to_string(),
            vars: None,
            when_condition: Some("include_condition".to_string()),
            tags: Some(vec!["include_tag".to_string()]),
            apply: None,
            delegate_to: Some("test_host".to_string()),
            delegate_facts: None,
            run_once: None,
        };

        let transformed =
            TaskIncludeProcessor::apply_task_transformations(task, &include_spec).unwrap();

        // Check transformations
        assert_eq!(transformed.delegate_to, Some("test_host".to_string()));
        assert!(transformed.tags.contains(&"original".to_string()));
        assert!(transformed.tags.contains(&"include_tag".to_string()));
        assert_eq!(
            transformed.when,
            Some("(existing_condition) and (include_condition)".to_string())
        );
    }

    #[test]
    fn test_is_valid_task_file() {
        assert!(TaskIncludeProcessor::is_valid_task_file("tasks.yml"));
        assert!(TaskIncludeProcessor::is_valid_task_file("tasks.yaml"));
        assert!(TaskIncludeProcessor::is_valid_task_file(
            "path/to/tasks.yml"
        ));
        assert!(!TaskIncludeProcessor::is_valid_task_file("tasks.txt"));
        assert!(!TaskIncludeProcessor::is_valid_task_file("tasks"));
        assert!(!TaskIncludeProcessor::is_valid_task_file("tasks.json"));
    }

    #[test]
    fn test_extract_task_metadata() {
        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: PathBuf::from("/path/to/playbook.yml"),
            include_depth: 2,
            tags: Vec::new(),
            when_condition: None,
        };

        let include_spec = IncludeSpec {
            file: "tasks/test.yml".to_string(),
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: Some("test_host".to_string()),
            delegate_facts: Some(true),
            run_once: Some(false),
        };

        let metadata = TaskIncludeProcessor::extract_task_metadata(&context, &include_spec);

        assert_eq!(
            metadata.included_from,
            PathBuf::from("/path/to/playbook.yml")
        );
        assert_eq!(metadata.include_depth, 2);
        assert_eq!(metadata.delegate_to, Some("test_host".to_string()));
        assert_eq!(metadata.delegate_facts, true);
        assert_eq!(metadata.run_once, false);
    }
}
