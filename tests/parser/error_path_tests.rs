use rustle_parse::parser::cache::ParseCache;
use rustle_parse::parser::dependency::resolve_task_dependencies;
use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::vault::VaultDecryptor;
use rustle_parse::types::parsed::*;
use std::collections::HashMap;
use tempfile::{NamedTempFile, TempDir};

fn create_temp_file(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(&file, content).expect("Failed to write to temp file");
    file
}

// Cache module tests
#[tokio::test]
async fn test_cache_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache = ParseCache::new(temp_dir.path().to_path_buf());

    // Cache should be created without error (can't access private field, just verify creation)
    let _cache = cache;
}

#[tokio::test]
async fn test_cache_get_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let cache = ParseCache::new(temp_dir.path().to_path_buf());

    let result: Option<String> = cache.get("nonexistent_key").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_set_returns_ok() {
    let temp_dir = TempDir::new().unwrap();
    let cache = ParseCache::new(temp_dir.path().to_path_buf());

    let test_value = "test_value".to_string();
    let result = cache.set("test_key", &test_value).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cache_round_trip() {
    let temp_dir = TempDir::new().unwrap();
    let cache = ParseCache::new(temp_dir.path().to_path_buf());

    let test_value = "test_value".to_string();

    // Set value
    let set_result = cache.set("test_key", &test_value).await;
    assert!(set_result.is_ok());

    // Get value
    let get_result: Option<String> = cache.get("test_key").await;

    // Since cache is not implemented, should return None
    assert!(get_result.is_none());
}

// Vault module tests
#[test]
fn test_vault_decryptor_creation() {
    let password = "test_password".to_string();
    let _decryptor = VaultDecryptor::new(password);

    // Should create without error (can't access private field)
}

#[test]
fn test_vault_decryptor_decrypt_returns_error() {
    let decryptor = VaultDecryptor::new("password".to_string());

    let encrypted_data = "fake_encrypted_data";
    let result = decryptor.decrypt(encrypted_data);

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::UnsupportedFeature { feature } => {
            assert!(feature.contains("Vault decryption"));
        }
        _ => panic!("Expected UnsupportedFeature error"),
    }
}

#[test]
fn test_vault_decryptor_decrypt_with_empty_data() {
    let decryptor = VaultDecryptor::new("password".to_string());

    let result = decryptor.decrypt("");
    assert!(result.is_err());
}

#[test]
fn test_vault_decryptor_decrypt_with_invalid_data() {
    let decryptor = VaultDecryptor::new("password".to_string());

    let result = decryptor.decrypt("invalid_vault_data");
    assert!(result.is_err());
}

#[test]
fn test_vault_decryptor_with_different_passwords() {
    let _decryptor1 = VaultDecryptor::new("password1".to_string());
    let _decryptor2 = VaultDecryptor::new("password2".to_string());

    // Both should create without error (can't access private fields to compare)
}

#[test]
fn test_vault_decryptor_error_message() {
    let decryptor = VaultDecryptor::new("test".to_string());
    let result = decryptor.decrypt("test");

    match result.unwrap_err() {
        ParseError::UnsupportedFeature { feature } => {
            assert!(feature.contains("not yet implemented"));
        }
        _ => panic!("Expected UnsupportedFeature error"),
    }
}

// Dependency resolution tests
#[test]
fn test_resolve_task_dependencies_empty() {
    let plays = Vec::new();
    let result = resolve_task_dependencies(&plays);
    assert!(result.is_empty());
}

#[test]
fn test_resolve_task_dependencies_single_task() {
    let task = ParsedTask {
        id: "task1".to_string(),
        name: "Test task".to_string(),
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
    };

    let play = ParsedPlay {
        name: "Test play".to_string(),
        hosts: HostPattern::All,
        vars: HashMap::new(),
        tasks: vec![task],
        handlers: Vec::new(),
        roles: Vec::new(),
        strategy: ExecutionStrategy::Linear,
        serial: None,
        max_fail_percentage: None,
    };

    let plays = vec![play];
    let result = resolve_task_dependencies(&plays);

    // Since dependencies are not implemented, should return task IDs
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "task1");
}

#[test]
fn test_resolve_task_dependencies_multiple_tasks() {
    let task1 = ParsedTask {
        id: "task1".to_string(),
        name: "First task".to_string(),
        module: "debug".to_string(),
        args: HashMap::new(),
        vars: HashMap::new(),
        when: None,
        loop_items: None,
        tags: Vec::new(),
        notify: vec!["handler1".to_string()],
        changed_when: None,
        failed_when: None,
        ignore_errors: false,
        delegate_to: None,
        dependencies: Vec::new(),
    };

    let task2 = ParsedTask {
        id: "task2".to_string(),
        name: "Second task".to_string(),
        module: "shell".to_string(),
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

    let handler = ParsedTask {
        id: "handler1".to_string(),
        name: "Handler task".to_string(),
        module: "service".to_string(),
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

    let play = ParsedPlay {
        name: "Test play".to_string(),
        hosts: HostPattern::All,
        vars: HashMap::new(),
        tasks: vec![task1, task2],
        handlers: vec![handler],
        roles: Vec::new(),
        strategy: ExecutionStrategy::Linear,
        serial: None,
        max_fail_percentage: None,
    };

    let plays = vec![play];
    let result = resolve_task_dependencies(&plays);

    assert_eq!(result.len(), 3); // 2 tasks + 1 handler
    assert!(result.contains(&"task1".to_string()));
    assert!(result.contains(&"task2".to_string()));
    assert!(result.contains(&"handler1".to_string()));
}

#[test]
fn test_resolve_task_dependencies_complex_scenario() {
    // Create a more complex scenario with multiple plays
    let task1 = ParsedTask {
        id: "play1_task1".to_string(),
        name: "Play 1 Task 1".to_string(),
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
    };

    let task2 = ParsedTask {
        id: "play2_task1".to_string(),
        name: "Play 2 Task 1".to_string(),
        module: "shell".to_string(),
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

    let play1 = ParsedPlay {
        name: "First play".to_string(),
        hosts: HostPattern::Single("webservers".to_string()),
        vars: HashMap::new(),
        tasks: vec![task1],
        handlers: Vec::new(),
        roles: Vec::new(),
        strategy: ExecutionStrategy::Linear,
        serial: None,
        max_fail_percentage: None,
    };

    let play2 = ParsedPlay {
        name: "Second play".to_string(),
        hosts: HostPattern::Single("databases".to_string()),
        vars: HashMap::new(),
        tasks: vec![task2],
        handlers: Vec::new(),
        roles: Vec::new(),
        strategy: ExecutionStrategy::Free,
        serial: Some(2),
        max_fail_percentage: Some(10.0),
    };

    let plays = vec![play1, play2];
    let result = resolve_task_dependencies(&plays);

    assert_eq!(result.len(), 2);
    assert!(result.contains(&"play1_task1".to_string()));
    assert!(result.contains(&"play2_task1".to_string()));
}

// Error type tests
#[test]
fn test_parse_error_display() {
    let file_not_found = ParseError::FileNotFound {
        path: "/test/path".to_string(),
    };
    assert!(format!("{}", file_not_found).contains("/test/path"));

    let invalid_structure = ParseError::InvalidStructure {
        message: "test message".to_string(),
    };
    assert!(format!("{}", invalid_structure).contains("test message"));

    let template_error = ParseError::Template {
        file: "test.yml".to_string(),
        line: 10,
        message: "undefined variable".to_string(),
    };
    let error_str = format!("{}", template_error);
    assert!(error_str.contains("test.yml"));
    assert!(error_str.contains("10"));
    assert!(error_str.contains("undefined variable"));

    let unsupported_feature = ParseError::UnsupportedFeature {
        feature: "test feature".to_string(),
    };
    assert!(format!("{}", unsupported_feature).contains("test feature"));
}

#[test]
fn test_parse_error_from_io_error() {
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
    let parse_error = ParseError::Io(io_error);

    match parse_error {
        ParseError::Io(e) => {
            assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
        }
        _ => panic!("Expected Io error"),
    }
}

#[test]
fn test_parse_error_from_yaml_error() {
    let yaml_error = serde_yaml::from_str::<serde_yaml::Value>("invalid: [yaml").unwrap_err();
    let parse_error = ParseError::Yaml(yaml_error);

    match parse_error {
        ParseError::Yaml(_) => {
            // Expected
        }
        _ => panic!("Expected Yaml error"),
    }
}

#[test]
fn test_parse_error_from_json_error() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let parse_error = ParseError::Json(json_error);

    match parse_error {
        ParseError::Json(_) => {
            // Expected
        }
        _ => panic!("Expected Json error"),
    }
}

// Test error conversions and chains
#[test]
fn test_error_source_chain() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let parse_error = ParseError::Io(io_error);

    // Test that we can access the source error
    assert!(std::error::Error::source(&parse_error).is_some());
}

#[test]
fn test_error_debug_format() {
    let error = ParseError::InvalidStructure {
        message: "test debug format".to_string(),
    };

    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("InvalidStructure"));
    assert!(debug_str.contains("test debug format"));
}

// File I/O error scenarios
#[tokio::test]
async fn test_io_error_permission_denied() {
    // Create a file with restricted permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let temp_file = create_temp_file("test content");
        let mut perms = std::fs::metadata(temp_file.path()).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        std::fs::set_permissions(temp_file.path(), perms).unwrap();

        let parser = rustle_parse::parser::Parser::new();
        let result = parser.parse_playbook(temp_file.path()).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied);
            }
            _ => panic!("Expected Io error with PermissionDenied"),
        }
    }
}

// Test malformed content handling
#[tokio::test]
async fn test_malformed_yaml_content() {
    let malformed_yaml = r#"
---
- name: Malformed YAML
  hosts: localhost
  tasks:
    - name: Task
      debug:
        msg: "test"
      # Missing closing bracket
      when: [condition
"#;

    let temp_file = create_temp_file(malformed_yaml);
    let parser = rustle_parse::parser::Parser::new();
    let result = parser.parse_playbook(temp_file.path()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Yaml(_) => {
            // Expected YAML parsing error
        }
        _ => panic!("Expected Yaml error"),
    }
}
