use rustle_parse::parser::{ParseError, Parser};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::fs;

#[tokio::test]
async fn test_parse_simple_playbook() {
    let parser = Parser::new();
    let fixture_path = PathBuf::from("tests/fixtures/playbooks/simple.yml");

    let result = parser.parse_playbook(&fixture_path).await;
    assert!(
        result.is_ok(),
        "Failed to parse simple playbook: {:?}",
        result
    );

    let playbook = result.unwrap();
    assert_eq!(playbook.plays.len(), 1);

    let play = &playbook.plays[0];
    assert_eq!(play.name, "Simple test playbook");
    assert_eq!(play.tasks.len(), 3);
    assert_eq!(play.handlers.len(), 1);

    // Check variables
    assert!(play.vars.contains_key("test_var"));
    assert!(play.vars.contains_key("number_var"));

    // Check first task
    let first_task = &play.tasks[0];
    assert_eq!(first_task.name, "Print a message");
    assert_eq!(first_task.module, "debug");
    assert!(first_task.tags.contains(&"debug".to_string()));
    assert!(first_task.tags.contains(&"test".to_string()));
}

#[tokio::test]
async fn test_parse_invalid_yaml() {
    let parser = Parser::new();

    // Create a temporary file with invalid YAML
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), "invalid: yaml: content: [unclosed")
        .await
        .unwrap();

    let result = parser.parse_playbook(temp_file.path()).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::Yaml(_) => {} // Expected
        other => panic!("Expected YAML error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_parse_with_extra_vars() {
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "extra_test".to_string(),
        serde_json::Value::String("from_cli".to_string()),
    );

    let parser = Parser::new().with_extra_vars(extra_vars);
    let fixture_path = PathBuf::from("tests/fixtures/playbooks/simple.yml");

    let result = parser.parse_playbook(&fixture_path).await;
    assert!(result.is_ok());

    let playbook = result.unwrap();
    assert!(playbook.variables.contains_key("extra_test"));
}

#[tokio::test]
async fn test_parse_nonexistent_file() {
    let parser = Parser::new();
    let nonexistent_path = PathBuf::from("nonexistent.yml");

    let result = parser.parse_playbook(&nonexistent_path).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::FileNotFound { .. } => {} // Expected
        other => panic!("Expected FileNotFound error, got: {:?}", other),
    }
}
