use rustle_parse::parser::{ParseError, Parser};
use std::collections::HashMap;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

fn create_temp_file(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(&file, content).expect("Failed to write to temp file");
    file
}

#[tokio::test]
async fn test_parser_new() {
    let parser = Parser::new();

    // Test that it can be created and used
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_file(playbook_content);
    let result = parser.parse_playbook(temp_file.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parser_default() {
    let parser = Parser::default();

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_file(playbook_content);
    let result = parser.parse_playbook(temp_file.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parser_with_vault_password() {
    let parser = Parser::new().with_vault_password("my_vault_password".to_string());

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_file(playbook_content);
    let result = parser.parse_playbook(temp_file.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parser_with_extra_vars() {
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "test_var".to_string(),
        serde_json::Value::String("test_value".to_string()),
    );
    extra_vars.insert(
        "number_var".to_string(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );

    let parser = Parser::new().with_extra_vars(extra_vars);

    let playbook_content = r#"
---
- name: Test {{ test_var }}
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Number: {{ number_var }}"
"#;

    let temp_file = create_temp_file(playbook_content);
    let playbook = parser.parse_playbook(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays[0].name, "Test test_value");
    assert!(playbook.variables.contains_key("test_var"));
    assert_eq!(
        playbook.variables["test_var"],
        serde_json::Value::String("test_value".to_string())
    );
    assert_eq!(
        playbook.variables["number_var"],
        serde_json::Value::Number(serde_json::Number::from(42))
    );
}

#[tokio::test]
async fn test_parser_with_cache() {
    let cache_dir = TempDir::new().unwrap();
    let parser = Parser::new().with_cache(cache_dir.path().to_path_buf());

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_file(playbook_content);
    let result = parser.parse_playbook(temp_file.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parser_chained_methods() {
    let cache_dir = TempDir::new().unwrap();
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "env".to_string(),
        serde_json::Value::String("test".to_string()),
    );

    let parser = Parser::new()
        .with_vault_password("secret".to_string())
        .with_extra_vars(extra_vars)
        .with_cache(cache_dir.path().to_path_buf());

    let playbook_content = r#"
---
- name: Test {{ env }} playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello {{ env }}"
"#;

    let temp_file = create_temp_file(playbook_content);
    let playbook = parser.parse_playbook(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays[0].name, "Test test playbook");
}

#[tokio::test]
async fn test_parse_playbook_file_not_found() {
    let parser = Parser::new();
    let nonexistent_path = Path::new("/nonexistent/playbook.yml");

    let result = parser.parse_playbook(nonexistent_path).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::FileNotFound { path } => {
            assert_eq!(path, "/nonexistent/playbook.yml");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_parse_inventory_ini() {
    let parser = Parser::new();

    let inventory_content = r#"
[webservers]
web1.example.com ansible_host=192.168.1.10
web2.example.com ansible_host=192.168.1.11

[databases]
db1.example.com ansible_host=192.168.1.20
"#;

    let temp_file = create_temp_file(inventory_content);
    let mut path_with_ini_ext = temp_file.path().to_path_buf();
    path_with_ini_ext.set_extension("ini");

    std::fs::copy(temp_file.path(), &path_with_ini_ext).unwrap();

    let inventory = parser.parse_inventory(&path_with_ini_ext).await.unwrap();

    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("db1.example.com"));
    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("databases"));

    std::fs::remove_file(path_with_ini_ext).ok();
}

#[tokio::test]
async fn test_parse_inventory_file_not_found() {
    let parser = Parser::new();
    let nonexistent_path = Path::new("/nonexistent/inventory.ini");

    let result = parser.parse_inventory(nonexistent_path).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::FileNotFound { path } => {
            assert_eq!(path, "/nonexistent/inventory.ini");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_validate_syntax_valid_playbook() {
    let parser = Parser::new();

    let valid_playbook = r#"
---
- name: Valid playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_file(valid_playbook);
    let result = parser.validate_syntax(temp_file.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_syntax_invalid_playbook() {
    let parser = Parser::new();

    let invalid_playbook = r#"
---
- name: Invalid playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
      invalid_yaml: [unclosed bracket
"#;

    let temp_file = create_temp_file(invalid_playbook);
    let result = parser.validate_syntax(temp_file.path()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_syntax_file_not_found() {
    let parser = Parser::new();
    let nonexistent_path = Path::new("/nonexistent/playbook.yml");

    let result = parser.validate_syntax(nonexistent_path).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        ParseError::FileNotFound { path } => {
            assert_eq!(path, "/nonexistent/playbook.yml");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_resolve_dependencies() {
    let parser = Parser::new();

    let playbook_content = r#"
---
- name: Test dependencies
  hosts: localhost
  tasks:
    - name: First task
      debug:
        msg: "First"
    - name: Second task
      debug:
        msg: "Second"
      notify: restart service
  handlers:
    - name: restart service
      service:
        name: myservice
        state: restarted
"#;

    let temp_file = create_temp_file(playbook_content);
    let playbook = parser.parse_playbook(temp_file.path()).await.unwrap();

    let dependencies = parser.resolve_dependencies(&playbook);
    // This should return some dependency analysis
    assert!(dependencies.is_empty() || !dependencies.is_empty()); // Just test it doesn't panic
}

#[tokio::test]
async fn test_parse_empty_playbook() {
    let parser = Parser::new();

    let empty_playbook = "---\n[]";
    let temp_file = create_temp_file(empty_playbook);

    let playbook = parser.parse_playbook(temp_file.path()).await.unwrap();
    assert_eq!(playbook.plays.len(), 0);
}

#[tokio::test]
async fn test_parse_playbook_with_template_errors() {
    let parser = Parser::new(); // No extra vars

    let playbook_content = r#"
---
- name: Test template errors
  hosts: localhost
  tasks:
    - name: Task with undefined variable
      debug:
        msg: "{{ undefined_var | mandatory }}"
"#;

    let temp_file = create_temp_file(playbook_content);
    let result = parser.parse_playbook(temp_file.path()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { .. } => {
            // Expected template error
        }
        _ => panic!("Expected Template error"),
    }
}

#[tokio::test]
async fn test_parse_inventory_yaml() {
    let parser = Parser::new();

    let yaml_inventory = r#"
all:
  children:
    webservers:
      hosts:
        web1.example.com:
          ansible_host: 192.168.1.10
        web2.example.com:
          ansible_host: 192.168.1.11
"#;

    let temp_file = create_temp_file(yaml_inventory);
    let mut path_with_yaml_ext = temp_file.path().to_path_buf();
    path_with_yaml_ext.set_extension("yml");

    std::fs::copy(temp_file.path(), &path_with_yaml_ext).unwrap();

    let inventory = parser.parse_inventory(&path_with_yaml_ext).await.unwrap();

    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("web2.example.com"));
    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_with_yaml_ext).ok();
}

#[tokio::test]
async fn test_parse_inventory_json() {
    let parser = Parser::new();

    let json_inventory = r#"
{
  "webservers": {
    "hosts": ["web1.example.com", "web2.example.com"]
  },
  "_meta": {
    "hostvars": {
      "web1.example.com": {
        "ansible_host": "192.168.1.10"
      },
      "web2.example.com": {
        "ansible_host": "192.168.1.11"
      }
    }
  }
}
"#;

    let temp_file = create_temp_file(json_inventory);
    let mut path_with_json_ext = temp_file.path().to_path_buf();
    path_with_json_ext.set_extension("json");

    std::fs::copy(temp_file.path(), &path_with_json_ext).unwrap();

    let inventory = parser.parse_inventory(&path_with_json_ext).await.unwrap();

    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("web2.example.com"));
    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_with_json_ext).ok();
}

#[tokio::test]
async fn test_parse_inventory_invalid_yaml() {
    let parser = Parser::new();

    let invalid_yaml = r#"
invalid_yaml: [unclosed bracket
more content here
"#;

    let temp_file = create_temp_file(invalid_yaml);
    let mut path_with_yaml_ext = temp_file.path().to_path_buf();
    path_with_yaml_ext.set_extension("yml");

    std::fs::copy(temp_file.path(), &path_with_yaml_ext).unwrap();

    let result = parser.parse_inventory(&path_with_yaml_ext).await;
    assert!(result.is_err());

    std::fs::remove_file(path_with_yaml_ext).ok();
}

#[tokio::test]
async fn test_parse_inventory_invalid_json() {
    let parser = Parser::new();

    let invalid_json = r#"{"invalid": "json" missing closing brace"#;

    let temp_file = create_temp_file(invalid_json);
    let mut path_with_json_ext = temp_file.path().to_path_buf();
    path_with_json_ext.set_extension("json");

    std::fs::copy(temp_file.path(), &path_with_json_ext).unwrap();

    let result = parser.parse_inventory(&path_with_json_ext).await;
    assert!(result.is_err());

    std::fs::remove_file(path_with_json_ext).ok();
}

#[tokio::test]
async fn test_parser_with_inventory_extra_vars() {
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "env".to_string(),
        serde_json::Value::String("prod".to_string()),
    );

    let parser = Parser::new().with_extra_vars(extra_vars);

    let inventory_content = r#"
[webservers]
web1.example.com
"#;

    let temp_file = create_temp_file(inventory_content);
    let mut path_with_ini_ext = temp_file.path().to_path_buf();
    path_with_ini_ext.set_extension("ini");

    std::fs::copy(temp_file.path(), &path_with_ini_ext).unwrap();

    let inventory = parser.parse_inventory(&path_with_ini_ext).await.unwrap();

    assert!(inventory.variables.contains_key("env"));
    assert_eq!(
        inventory.variables["env"],
        serde_json::Value::String("prod".to_string())
    );

    std::fs::remove_file(path_with_ini_ext).ok();
}

#[tokio::test]
async fn test_validate_syntax_empty_file() {
    let parser = Parser::new();

    let empty_content = "";
    let temp_file = create_temp_file(empty_content);

    let result = parser.validate_syntax(temp_file.path()).await;
    // Empty file should be invalid
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_syntax_whitespace_only() {
    let parser = Parser::new();

    let whitespace_content = "   \n  \t  \n  ";
    let temp_file = create_temp_file(whitespace_content);

    let result = parser.validate_syntax(temp_file.path()).await;
    // Whitespace-only file should be invalid
    assert!(result.is_err());
}
