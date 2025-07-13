use rustle_parse::parser::{
    include::{ImportSpec, IncludeConfig, IncludeContext, IncludeHandler, IncludeSpec},
    template::TemplateEngine,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

#[cfg(test)]
mod include_tests {
    use super::*;

    #[tokio::test]
    async fn test_include_playbook_functionality() {
        let temp_dir = create_test_environment().await;
        let base_path = temp_dir.path().to_path_buf();

        // Create main playbook
        let main_playbook = r#"---
- name: Main Play
  hosts: all
  vars:
    main_var: "main_value"
  tasks:
    - name: Before include
      debug:
        msg: "Before include"
"#;
        fs::write(base_path.join("main.yml"), main_playbook)
            .await
            .unwrap();

        // Create included playbook
        let included_playbook = r#"---
- name: Included Play
  hosts: web_servers
  vars:
    included_var: "included_value"
  tasks:
    - name: Included task
      debug:
        msg: "From included playbook: {{ main_var | default('not_found') }}"
"#;
        fs::write(base_path.join("included.yml"), included_playbook)
            .await
            .unwrap();

        // Test include_playbook
        let template_engine = TemplateEngine::new();
        let config = IncludeConfig {
            strict_file_permissions: false, // Allow testing
            ..IncludeConfig::default()
        };
        let mut handler =
            IncludeHandler::new(base_path.clone(), template_engine).with_config(config);

        let include_spec = IncludeSpec {
            file: "included.yml".to_string(),
            vars: Some({
                let mut vars = HashMap::new();
                vars.insert(
                    "override_var".to_string(),
                    serde_json::Value::String("override_value".to_string()),
                );
                vars
            }),
            when_condition: None,
            tags: Some(vec!["included".to_string()]),
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };

        let context = IncludeContext {
            variables: {
                let mut vars = HashMap::new();
                vars.insert(
                    "main_var".to_string(),
                    serde_json::Value::String("main_value".to_string()),
                );
                vars
            },
            current_file: base_path.join("main.yml"),
            include_depth: 0,
            tags: vec![],
            when_condition: None,
        };

        let result = handler.include_playbook(&include_spec, &context).await;

        match result {
            Ok(plays) => {
                assert!(!plays.is_empty(), "Should have included plays");
                assert_eq!(plays.len(), 1, "Should have exactly one included play");

                let included_play = &plays[0];
                assert_eq!(included_play.name, "Included Play");

                // Check variable inheritance
                assert!(included_play.vars.contains_key("included_var"));
                assert!(included_play.vars.contains_key("override_var"));
                assert!(included_play.vars.contains_key("main_var")); // Should inherit from context

                // Check tags are applied
                for task in &included_play.tasks {
                    assert!(task.tags.contains(&"included".to_string()));
                }

                println!("✅ include_playbook test passed");
            }
            Err(e) => {
                panic!("include_playbook failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_import_playbook_functionality() {
        let temp_dir = create_test_environment().await;
        let base_path = temp_dir.path().to_path_buf();

        // Create import playbook
        let import_playbook = r#"---
- name: Imported Play
  hosts: db_servers
  vars:
    imported_var: "imported_value"
  tasks:
    - name: Imported task
      command: echo "Imported task executed"
      register: import_result

    - name: Show import result
      debug:
        var: import_result.stdout
"#;
        fs::write(base_path.join("import.yml"), import_playbook)
            .await
            .unwrap();

        let template_engine = TemplateEngine::new();
        let config = IncludeConfig {
            strict_file_permissions: false, // Allow testing
            ..IncludeConfig::default()
        };
        let mut handler =
            IncludeHandler::new(base_path.clone(), template_engine).with_config(config);

        let import_spec = ImportSpec {
            file: "import.yml".to_string(),
            vars: Some({
                let mut vars = HashMap::new();
                vars.insert(
                    "import_var".to_string(),
                    serde_json::Value::String("import_value".to_string()),
                );
                vars
            }),
            when_condition: None,
            tags: Some(vec!["imported".to_string()]),
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: base_path.join("main.yml"),
            include_depth: 0,
            tags: vec![],
            when_condition: None,
        };

        let result = handler.import_playbook(&import_spec, &context).await;

        match result {
            Ok(plays) => {
                assert!(!plays.is_empty(), "Should have imported plays");
                assert_eq!(plays.len(), 1, "Should have exactly one imported play");

                let imported_play = &plays[0];
                assert_eq!(imported_play.name, "Imported Play");

                // Check variable merging for imports
                assert!(imported_play.vars.contains_key("imported_var"));
                assert!(imported_play.vars.contains_key("import_var"));

                println!("✅ import_playbook test passed");
            }
            Err(e) => {
                panic!("import_playbook failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_playbook_include_with_conditions() {
        let temp_dir = create_test_environment().await;
        let base_path = temp_dir.path().to_path_buf();

        // Create conditional playbook
        let conditional_playbook = r#"---
- name: Conditional Play
  hosts: all
  tasks:
    - name: Always runs
      debug:
        msg: "This always runs"

    - name: Conditional task
      debug:
        msg: "This runs conditionally"
      when: run_conditional | default(false)
"#;
        fs::write(base_path.join("conditional.yml"), conditional_playbook)
            .await
            .unwrap();

        let template_engine = TemplateEngine::new();
        let config = IncludeConfig {
            strict_file_permissions: false, // Allow testing
            ..IncludeConfig::default()
        };
        let mut handler =
            IncludeHandler::new(base_path.clone(), template_engine).with_config(config);

        // Test with when condition that should be false
        let include_spec = IncludeSpec {
            file: "conditional.yml".to_string(),
            vars: None,
            when_condition: Some("include_this | default(false)".to_string()),
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };

        let context = IncludeContext {
            variables: {
                let mut vars = HashMap::new();
                vars.insert("include_this".to_string(), serde_json::Value::Bool(false));
                vars
            },
            current_file: base_path.join("main.yml"),
            include_depth: 0,
            tags: vec![],
            when_condition: None,
        };

        let result = handler.include_playbook(&include_spec, &context).await;

        match result {
            Ok(plays) => {
                // Should return empty because when condition is false
                assert!(
                    plays.is_empty(),
                    "Should not include plays when condition is false"
                );
                println!("✅ Conditional include test passed (correctly excluded)");
            }
            Err(e) => {
                panic!("Conditional include failed: {:?}", e);
            }
        }

        // Test with when condition that should be true
        let mut context_true = context.clone();
        context_true
            .variables
            .insert("include_this".to_string(), serde_json::Value::Bool(true));

        let result_true = handler.include_playbook(&include_spec, &context_true).await;

        match result_true {
            Ok(plays) => {
                assert!(
                    !plays.is_empty(),
                    "Should include plays when condition is true"
                );
                println!("✅ Conditional include test passed (correctly included)");
            }
            Err(e) => {
                panic!("Conditional include (true case) failed: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_playbook_include_with_apply_block() {
        let temp_dir = create_test_environment().await;
        let base_path = temp_dir.path().to_path_buf();

        // Create playbook for apply test
        let apply_playbook = r#"---
- name: Apply Test Play
  hosts: all
  tasks:
    - name: Task 1
      debug:
        msg: "Task 1"

    - name: Task 2
      debug:
        msg: "Task 2"
      when: existing_condition | default(true)
"#;
        fs::write(base_path.join("apply_test.yml"), apply_playbook)
            .await
            .unwrap();

        let template_engine = TemplateEngine::new();
        let config = IncludeConfig {
            strict_file_permissions: false, // Allow testing
            ..IncludeConfig::default()
        };
        let mut handler =
            IncludeHandler::new(base_path.clone(), template_engine).with_config(config);

        let include_spec = IncludeSpec {
            file: "apply_test.yml".to_string(),
            vars: None,
            when_condition: None,
            tags: Some(vec!["include_tag".to_string()]),
            apply: Some(rustle_parse::parser::include::ApplySpec {
                tags: Some(vec!["apply_tag".to_string()]),
                when_condition: Some("apply_condition".to_string()),
                r#become: Some(true),
                become_user: Some("root".to_string()),
            }),
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: base_path.join("main.yml"),
            include_depth: 0,
            tags: vec![],
            when_condition: None,
        };

        let result = handler.include_playbook(&include_spec, &context).await;

        match result {
            Ok(plays) => {
                assert!(!plays.is_empty(), "Should have included plays");

                let play = &plays[0];
                for task in &play.tasks {
                    // Check that apply tags are added
                    assert!(task.tags.contains(&"include_tag".to_string()));
                    assert!(task.tags.contains(&"apply_tag".to_string()));

                    // Check that apply when condition is combined with existing conditions
                    if let Some(when_condition) = &task.when {
                        assert!(when_condition.contains("apply_condition"));
                    }
                }

                println!("✅ Apply block test passed");
            }
            Err(e) => {
                panic!("Apply block test failed: {:?}", e);
            }
        }
    }

    async fn create_test_environment() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        temp_dir
    }
}
