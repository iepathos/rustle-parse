use pretty_assertions::assert_eq;
use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::inventory::ini::InventoryParserConfig;
use rustle_parse::parser::inventory::InventoryParser;
use rustle_parse::parser::template::TemplateEngine;
use rustle_parse::types::parsed::*;
use std::collections::HashMap;
use std::path::Path;
use tempfile::NamedTempFile;

fn create_temp_file(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(&file, content).expect("Failed to write to temp file");
    file
}

fn setup_parser() -> (TemplateEngine, HashMap<String, serde_json::Value>) {
    let template_engine = TemplateEngine::new();
    let extra_vars = HashMap::new();
    (template_engine, extra_vars)
}

#[tokio::test]
async fn test_parse_ini_inventory_file() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let ini_content = r#"
[webservers]
web1.example.com ansible_host=192.168.1.10 ansible_user=admin
web2.example.com ansible_host=192.168.1.11

[databases]
db1.example.com ansible_host=192.168.1.20 ansible_port=5432

[webservers:vars]
http_port=80
max_clients=200
"#;

    let temp_file = create_temp_file(ini_content);
    let path = temp_file.path();
    let mut path_with_ini_ext = path.to_path_buf();
    path_with_ini_ext.set_extension("ini");

    // Copy content to file with .ini extension
    std::fs::copy(path, &path_with_ini_ext).unwrap();

    let inventory = parser.parse(&path_with_ini_ext).await.unwrap();

    assert_eq!(inventory.hosts.len(), 3);
    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("web2.example.com"));
    assert!(inventory.hosts.contains_key("db1.example.com"));

    let web1 = &inventory.hosts["web1.example.com"];
    assert_eq!(web1.address, Some("192.168.1.10".to_string()));
    assert_eq!(web1.user, Some("admin".to_string()));

    let db1 = &inventory.hosts["db1.example.com"];
    assert_eq!(db1.port, Some(5432));

    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("databases"));

    std::fs::remove_file(path_with_ini_ext).ok();
}

#[tokio::test]
async fn test_parse_yaml_inventory_file() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let yaml_content = r#"
all:
  vars:
    global_var: global_value
  children:
    webservers:
      hosts:
        web1.example.com:
          ansible_host: 192.168.1.10
          ansible_user: admin
        web2.example.com:
          ansible_host: 192.168.1.11
      vars:
        http_port: 80
    databases:
      hosts:
        db1.example.com:
          ansible_host: 192.168.1.20
          ansible_port: 5432
"#;

    let temp_file = create_temp_file(yaml_content);
    let path = temp_file.path();
    let mut path_with_yaml_ext = path.to_path_buf();
    path_with_yaml_ext.set_extension("yml");

    std::fs::copy(path, &path_with_yaml_ext).unwrap();

    let inventory = parser.parse(&path_with_yaml_ext).await.unwrap();

    assert!(inventory.variables.contains_key("global_var"));
    assert_eq!(
        inventory.variables["global_var"],
        serde_json::Value::String("global_value".to_string())
    );

    assert_eq!(inventory.hosts.len(), 3);
    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("db1.example.com"));

    let web1 = &inventory.hosts["web1.example.com"];
    assert_eq!(web1.address, Some("192.168.1.10".to_string()));
    assert_eq!(web1.user, Some("admin".to_string()));

    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("databases"));

    std::fs::remove_file(path_with_yaml_ext).ok();
}

#[tokio::test]
async fn test_parse_json_inventory_file() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let json_content = r#"
{
  "webservers": {
    "hosts": ["web1.example.com", "web2.example.com"],
    "vars": {
      "http_port": 80,
      "max_clients": 200
    }
  },
  "databases": {
    "hosts": ["db1.example.com"],
    "vars": {
      "db_port": 5432
    }
  },
  "_meta": {
    "hostvars": {
      "web1.example.com": {
        "ansible_host": "192.168.1.10",
        "ansible_user": "admin"
      },
      "web2.example.com": {
        "ansible_host": "192.168.1.11"
      },
      "db1.example.com": {
        "ansible_host": "192.168.1.20",
        "ansible_port": 5432
      }
    }
  }
}
"#;

    let temp_file = create_temp_file(json_content);
    let path = temp_file.path();
    let mut path_with_json_ext = path.to_path_buf();
    path_with_json_ext.set_extension("json");

    std::fs::copy(path, &path_with_json_ext).unwrap();

    let inventory = parser.parse(&path_with_json_ext).await.unwrap();

    assert_eq!(inventory.hosts.len(), 3);
    assert!(inventory.hosts.contains_key("web1.example.com"));
    assert!(inventory.hosts.contains_key("db1.example.com"));

    let web1 = &inventory.hosts["web1.example.com"];
    assert_eq!(web1.address, Some("192.168.1.10".to_string()));
    assert_eq!(web1.user, Some("admin".to_string()));

    let db1 = &inventory.hosts["db1.example.com"];
    assert_eq!(db1.port, Some(5432));

    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("databases"));

    std::fs::remove_file(path_with_json_ext).ok();
}

#[tokio::test]
async fn test_auto_detect_json_format() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let json_content = r#"{"webservers": {"hosts": ["web1.example.com"]}}"#;

    let temp_file = create_temp_file(json_content);
    let path = temp_file.path();
    let mut path_without_ext = path.to_path_buf();
    path_without_ext.set_extension("");

    std::fs::copy(path, &path_without_ext).unwrap();

    let inventory = parser.parse(&path_without_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_without_ext).ok();
}

#[tokio::test]
async fn test_auto_detect_yaml_format() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let yaml_content = r#"
---
webservers:
  hosts:
    web1.example.com:
"#;

    let temp_file = create_temp_file(yaml_content);
    let path = temp_file.path();
    let mut path_without_ext = path.to_path_buf();
    path_without_ext.set_extension("");

    std::fs::copy(path, &path_without_ext).unwrap();

    let inventory = parser.parse(&path_without_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_without_ext).ok();
}

#[tokio::test]
async fn test_auto_detect_ini_format_fallback() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let ini_content = r#"
[webservers]
web1.example.com
"#;

    let temp_file = create_temp_file(ini_content);
    let path = temp_file.path();
    let mut path_without_ext = path.to_path_buf();
    path_without_ext.set_extension("");

    std::fs::copy(path, &path_without_ext).unwrap();

    let inventory = parser.parse(&path_without_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_without_ext).ok();
}

#[tokio::test]
async fn test_file_not_found_error() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let nonexistent_path = Path::new("/nonexistent/path/inventory.ini");
    let result = parser.parse(nonexistent_path).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::FileNotFound { path } => {
            assert_eq!(path, "/nonexistent/path/inventory.ini");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_invalid_yaml_content() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let invalid_yaml = r#"
invalid_yaml: [unclosed bracket
more content here
"#;

    let temp_file = create_temp_file(invalid_yaml);
    let path = temp_file.path();
    let mut path_with_yaml_ext = path.to_path_buf();
    path_with_yaml_ext.set_extension("yml");

    std::fs::copy(path, &path_with_yaml_ext).unwrap();

    let result = parser.parse(&path_with_yaml_ext).await;

    assert!(result.is_err());
    // Should be a YAML parsing error
    assert!(matches!(result.unwrap_err(), ParseError::Yaml(_)));

    std::fs::remove_file(path_with_yaml_ext).ok();
}

#[tokio::test]
async fn test_invalid_json_content() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let invalid_json = r#"{"invalid": "json" missing closing brace"#;

    let temp_file = create_temp_file(invalid_json);
    let path = temp_file.path();
    let mut path_with_json_ext = path.to_path_buf();
    path_with_json_ext.set_extension("json");

    std::fs::copy(path, &path_with_json_ext).unwrap();

    let result = parser.parse(&path_with_json_ext).await;

    assert!(result.is_err());
    // Should be a JSON parsing error
    assert!(matches!(result.unwrap_err(), ParseError::Json(_)));

    std::fs::remove_file(path_with_json_ext).ok();
}

#[tokio::test]
async fn test_parser_with_config() {
    let (template_engine, extra_vars) = setup_parser();
    let config = InventoryParserConfig {
        strict_mode: true,
        expand_patterns: true,
        max_pattern_expansion: 1000,
        validate_hosts: true,
        resolve_dns: false,
    };
    let parser = InventoryParser::with_config(&template_engine, &extra_vars, config);

    let ini_content = r#"
[webservers]
web1.example.com
"#;

    let temp_file = create_temp_file(ini_content);
    let path = temp_file.path();
    let mut path_with_ini_ext = path.to_path_buf();
    path_with_ini_ext.set_extension("ini");

    std::fs::copy(path, &path_with_ini_ext).unwrap();

    let inventory = parser.parse(&path_with_ini_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));

    std::fs::remove_file(path_with_ini_ext).ok();
}

#[tokio::test]
async fn test_parser_with_extra_vars() {
    let template_engine = TemplateEngine::new();
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "env".to_string(),
        serde_json::Value::String("production".to_string()),
    );
    extra_vars.insert(
        "region".to_string(),
        serde_json::Value::String("us-west-2".to_string()),
    );

    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let ini_content = r#"
[webservers]
web1.example.com
"#;

    let temp_file = create_temp_file(ini_content);
    let path = temp_file.path();
    let mut path_with_ini_ext = path.to_path_buf();
    path_with_ini_ext.set_extension("ini");

    std::fs::copy(path, &path_with_ini_ext).unwrap();

    let inventory = parser.parse(&path_with_ini_ext).await.unwrap();

    assert_eq!(
        inventory.variables["env"],
        serde_json::Value::String("production".to_string())
    );
    assert_eq!(
        inventory.variables["region"],
        serde_json::Value::String("us-west-2".to_string())
    );

    std::fs::remove_file(path_with_ini_ext).ok();
}

#[test]
fn test_expand_host_pattern() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let result = parser
        .expand_host_pattern("web[01:03].example.com")
        .unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&"web01.example.com".to_string()));
    assert!(result.contains(&"web02.example.com".to_string()));
    assert!(result.contains(&"web03.example.com".to_string()));
}

#[test]
fn test_expand_host_pattern_single_host() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let result = parser.expand_host_pattern("web1.example.com").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "web1.example.com");
}

#[test]
fn test_parse_host_variables() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let result = parser
        .parse_host_variables("ansible_host=192.168.1.10 ansible_user=admin ansible_port=22")
        .unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(
        result["ansible_host"],
        serde_json::Value::String("192.168.1.10".to_string())
    );
    assert_eq!(
        result["ansible_user"],
        serde_json::Value::String("admin".to_string())
    );
    assert_eq!(
        result["ansible_port"],
        serde_json::Value::String("22".to_string())
    );
}

#[test]
fn test_parse_host_variables_empty() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let result = parser.parse_host_variables("").unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_resolve_group_inheritance() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let mut inventory = ParsedInventory {
        hosts: HashMap::new(),
        groups: HashMap::new(),
        variables: HashMap::new(),
    };

    // Add test data
    let parent_group = ParsedGroup {
        name: "parent".to_string(),
        hosts: vec!["host1".to_string()],
        children: vec!["child".to_string()],
        vars: {
            let mut vars = HashMap::new();
            vars.insert(
                "parent_var".to_string(),
                serde_json::Value::String("parent_value".to_string()),
            );
            vars
        },
    };

    let child_group = ParsedGroup {
        name: "child".to_string(),
        hosts: vec!["host2".to_string()],
        children: vec![],
        vars: {
            let mut vars = HashMap::new();
            vars.insert(
                "child_var".to_string(),
                serde_json::Value::String("child_value".to_string()),
            );
            vars
        },
    };

    inventory.groups.insert("parent".to_string(), parent_group);
    inventory.groups.insert("child".to_string(), child_group);

    let result = parser.resolve_group_inheritance(&mut inventory);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_inventory() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let mut hosts = HashMap::new();
    hosts.insert(
        "web1.example.com".to_string(),
        ParsedHost {
            name: "web1.example.com".to_string(),
            address: Some("192.168.1.10".to_string()),
            port: Some(22),
            user: Some("admin".to_string()),
            vars: HashMap::new(),
            groups: vec!["webservers".to_string()],
        },
    );

    let mut groups = HashMap::new();
    groups.insert(
        "webservers".to_string(),
        ParsedGroup {
            name: "webservers".to_string(),
            hosts: vec!["web1.example.com".to_string()],
            children: vec![],
            vars: HashMap::new(),
        },
    );

    // Ensure 'all' group exists for validation
    groups.insert(
        "all".to_string(),
        ParsedGroup {
            name: "all".to_string(),
            hosts: vec!["web1.example.com".to_string()],
            children: vec!["webservers".to_string()],
            vars: HashMap::new(),
        },
    );

    let inventory = ParsedInventory {
        hosts,
        groups,
        variables: HashMap::new(),
    };

    let result = parser.validate_inventory(&inventory);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_yaml_inventory_with_children() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let yaml_content = r#"
all:
  children:
    webservers:
      hosts:
        web1.example.com:
        web2.example.com:
      children:
        apache:
          hosts:
            apache1.example.com:
        nginx:
          hosts:
            nginx1.example.com:
"#;

    let temp_file = create_temp_file(yaml_content);
    let path = temp_file.path();
    let mut path_with_yaml_ext = path.to_path_buf();
    path_with_yaml_ext.set_extension("yml");

    std::fs::copy(path, &path_with_yaml_ext).unwrap();

    let inventory = parser.parse(&path_with_yaml_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("apache"));
    assert!(inventory.groups.contains_key("nginx"));

    let webservers = &inventory.groups["webservers"];
    assert!(webservers.children.contains(&"apache".to_string()));
    assert!(webservers.children.contains(&"nginx".to_string()));

    std::fs::remove_file(path_with_yaml_ext).ok();
}

#[tokio::test]
async fn test_json_inventory_with_children() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let json_content = r#"
{
  "webservers": {
    "hosts": ["web1.example.com"],
    "children": ["apache", "nginx"]
  },
  "apache": {
    "hosts": ["apache1.example.com"]
  },
  "nginx": {
    "hosts": ["nginx1.example.com"]
  }
}
"#;

    let temp_file = create_temp_file(json_content);
    let path = temp_file.path();
    let mut path_with_json_ext = path.to_path_buf();
    path_with_json_ext.set_extension("json");

    std::fs::copy(path, &path_with_json_ext).unwrap();

    let inventory = parser.parse(&path_with_json_ext).await.unwrap();

    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("apache"));
    assert!(inventory.groups.contains_key("nginx"));

    let webservers = &inventory.groups["webservers"];
    assert!(webservers.children.contains(&"apache".to_string()));
    assert!(webservers.children.contains(&"nginx".to_string()));

    std::fs::remove_file(path_with_json_ext).ok();
}

#[tokio::test]
async fn test_empty_inventory_file() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = InventoryParser::new(&template_engine, &extra_vars);

    let empty_content = "";

    let temp_file = create_temp_file(empty_content);
    let path = temp_file.path();
    let mut path_with_ini_ext = path.to_path_buf();
    path_with_ini_ext.set_extension("ini");

    std::fs::copy(path, &path_with_ini_ext).unwrap();

    let inventory = parser.parse(&path_with_ini_ext).await.unwrap();

    // Should still create an 'all' group
    assert!(inventory.groups.contains_key("all"));

    std::fs::remove_file(path_with_ini_ext).ok();
}
