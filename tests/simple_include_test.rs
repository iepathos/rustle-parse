use rustle_parse::parser::{
    include::{ImportSpec, IncludeConfig, IncludeContext, IncludeHandler, IncludeSpec},
    template::TemplateEngine,
};
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

/// Simple integration test for include/import playbook functionality
#[tokio::test]
async fn test_basic_include_functionality() {
    // Create temporary directory for test files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_path = temp_dir.path().to_path_buf();

    // Create test files within the temp directory
    let included_content = r#"---
- name: Included Play
  hosts: all
  tasks:
    - name: Test task
      debug:
        msg: "Hello from included playbook"
"#;

    let include_file = base_path.join("included.yml");
    fs::write(&include_file, included_content).await.unwrap();

    // Create include handler with relaxed security
    let template_engine = TemplateEngine::new();
    let config = IncludeConfig {
        strict_file_permissions: false,
        allow_absolute_paths: true,
        ..IncludeConfig::default()
    };

    let mut handler = IncludeHandler::new(base_path.clone(), template_engine).with_config(config);

    // Test include_playbook
    let include_spec = IncludeSpec {
        file: "included.yml".to_string(),
        vars: None,
        when_condition: None,
        tags: Some(vec!["test".to_string()]),
        apply: None,
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

    // Test the include_playbook functionality
    let result = handler.include_playbook(&include_spec, &context).await;

    match result {
        Ok(plays) => {
            assert!(!plays.is_empty(), "Should have included plays");
            println!("✅ Successfully included {} plays", plays.len());

            let play = &plays[0];
            assert_eq!(play.name, "Included Play");

            // Check that tags are applied
            for task in &play.tasks {
                assert!(task.tags.contains(&"test".to_string()));
            }

            println!("✅ Include playbook test passed");
        }
        Err(e) => {
            panic!("Include playbook test failed: {e:?}");
        }
    }

    // Test import_playbook
    let import_spec = ImportSpec {
        file: "included.yml".to_string(),
        vars: None,
        when_condition: None,
        tags: Some(vec!["imported".to_string()]),
    };

    let result_import = handler.import_playbook(&import_spec, &context).await;

    match result_import {
        Ok(plays) => {
            assert!(!plays.is_empty(), "Should have imported plays");
            println!("✅ Successfully imported {} plays", plays.len());
            println!("✅ Import playbook test passed");
        }
        Err(e) => {
            panic!("Import playbook test failed: {e:?}");
        }
    }
}

/// Test include handler configuration
#[tokio::test]
async fn test_include_handler_stats() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_path = temp_dir.path().to_path_buf();

    let template_engine = TemplateEngine::new();
    let handler = IncludeHandler::new(base_path, template_engine);

    let stats = handler.get_stats();
    assert_eq!(stats.current_depth, 0);
    assert_eq!(stats.max_depth, 100); // Default max depth

    println!("✅ Include handler stats test passed");
    println!(
        "✅ Max depth: {}, Current depth: {}",
        stats.max_depth, stats.current_depth
    );
}
