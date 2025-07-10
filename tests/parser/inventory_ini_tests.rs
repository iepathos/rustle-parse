use once_cell::sync::Lazy;
use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::inventory::ini::InventoryParserConfig;
use rustle_parse::parser::{InventoryParser, TemplateEngine};
use std::collections::HashMap;
use std::io::Write;

static TEMPLATE_ENGINE: Lazy<TemplateEngine> = Lazy::new(TemplateEngine::new);
static EXTRA_VARS: Lazy<HashMap<String, serde_json::Value>> = Lazy::new(HashMap::new);

fn create_test_parser() -> InventoryParser<'static> {
    InventoryParser::new(&TEMPLATE_ENGINE, &EXTRA_VARS)
}

fn create_test_parser_with_config(config: InventoryParserConfig) -> InventoryParser<'static> {
    InventoryParser::with_config(&TEMPLATE_ENGINE, &EXTRA_VARS, config)
}

#[tokio::test]
async fn test_simple_ini_inventory() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/hosts.ini");

    let inventory = parser.parse(path).await.unwrap();

    // Check hosts
    assert_eq!(inventory.hosts.len(), 4); // web1, web2, db1, db2

    // Verify web1 host
    let web1 = inventory.hosts.get("web1").unwrap();
    assert_eq!(web1.address, Some("192.168.1.10".to_string()));
    assert_eq!(web1.user, Some("deploy".to_string()));
    assert!(web1.groups.contains(&"webservers".to_string()));

    // Verify groups
    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("databases"));
    assert!(inventory.groups.contains_key("production"));

    // Check group variables
    let webservers = inventory.groups.get("webservers").unwrap();
    assert_eq!(
        webservers.vars.get("http_port").unwrap().as_u64().unwrap(),
        80
    );

    // Check group children
    let production = inventory.groups.get("production").unwrap();
    assert!(production.children.contains(&"webservers".to_string()));
    assert!(production.children.contains(&"databases".to_string()));
}

#[tokio::test]
async fn test_complex_ini_inventory() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/complex.ini");

    let inventory = parser.parse(path).await.unwrap();

    // Should have expanded all patterns
    assert!(inventory.hosts.len() > 10);

    // Test pattern expansion
    assert!(inventory.hosts.contains_key("web01"));
    assert!(inventory.hosts.contains_key("web02"));
    assert!(inventory.hosts.contains_key("web03"));

    assert!(inventory.hosts.contains_key("db-a"));
    assert!(inventory.hosts.contains_key("db-b"));
    assert!(inventory.hosts.contains_key("db-c"));

    // Test list patterns
    assert!(inventory.hosts.contains_key("redis1"));
    assert!(inventory.hosts.contains_key("redis3"));
    assert!(inventory.hosts.contains_key("redis5"));

    // Verify group hierarchy
    let production = inventory.groups.get("production").unwrap();
    assert!(production.children.contains(&"infrastructure".to_string()));

    let infrastructure = inventory.groups.get("infrastructure").unwrap();
    assert!(infrastructure.children.contains(&"backend".to_string()));
    assert!(infrastructure.children.contains(&"frontend".to_string()));

    // Test variable inheritance
    let web01 = inventory.hosts.get("web01").unwrap();
    assert_eq!(
        web01.vars.get("env").unwrap().as_str().unwrap(),
        "production"
    );
    assert_eq!(web01.vars.get("http_port").unwrap().as_u64().unwrap(), 80);
}

#[tokio::test]
async fn test_host_pattern_expansion() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/patterns.ini");

    let inventory = parser.parse(path).await.unwrap();

    // Test numeric patterns
    for i in 1..=5 {
        let host_name = format!("web{i:02}");
        assert!(
            inventory.hosts.contains_key(&host_name),
            "Missing host: {host_name}"
        );
    }

    // Test alphabetic patterns
    for c in ['a', 'b', 'c', 'd'] {
        let host_name = format!("db-{c}");
        assert!(
            inventory.hosts.contains_key(&host_name),
            "Missing host: {host_name}"
        );
    }

    // Test list patterns
    for num in [1, 3, 5, 7, 9] {
        let host_name = format!("worker{num}");
        assert!(
            inventory.hosts.contains_key(&host_name),
            "Missing host: {host_name}"
        );
    }

    for color in ["red", "blue", "green"] {
        let host_name = format!("queue{color}");
        assert!(
            inventory.hosts.contains_key(&host_name),
            "Missing host: {host_name}"
        );
    }

    // Test zero-padded patterns
    for i in 1..=10 {
        let host_name = format!("host{i:03}");
        assert!(
            inventory.hosts.contains_key(&host_name),
            "Missing host: {host_name}"
        );
    }
}

#[tokio::test]
async fn test_variable_inheritance() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/inheritance.ini");

    let inventory = parser.parse(path).await.unwrap();

    // Test host variable precedence (host vars should override group vars)
    let web1 = inventory.hosts.get("web1").unwrap();
    assert_eq!(
        web1.vars.get("host_override").unwrap().as_str().unwrap(),
        "from_host"
    );
    assert_eq!(
        web1.vars.get("host_var").unwrap().as_str().unwrap(),
        "host_value"
    );

    // Test group variable inheritance
    assert_eq!(
        web1.vars.get("env").unwrap().as_str().unwrap(),
        "production"
    );
    assert_eq!(
        web1.vars.get("global_var").unwrap().as_str().unwrap(),
        "global_value"
    );

    // Test variable precedence chain (host > group > parent group > global)
    // host_override should be from host level (highest precedence)
    assert_eq!(
        web1.vars.get("host_override").unwrap().as_str().unwrap(),
        "from_host"
    );

    // common_var should follow precedence rules
    let db1 = inventory.hosts.get("db1").unwrap();
    assert_eq!(
        db1.vars.get("host_override").unwrap().as_str().unwrap(),
        "from_host_db"
    );
}

#[tokio::test]
async fn test_edge_cases() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/edge_cases.ini");

    let inventory = parser.parse(path).await.unwrap();

    // Test quoted variables
    let host1 = inventory.hosts.get("host1").unwrap();
    assert_eq!(host1.address, Some("192.168.1.100".to_string()));
    assert_eq!(
        host1.vars.get("description").unwrap().as_str().unwrap(),
        "Host with spaces"
    );

    // Test boolean values
    let host_with_bools = inventory.hosts.get("host1").unwrap();
    assert!(
        host_with_bools
            .vars
            .get("enabled")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        !host_with_bools
            .vars
            .get("disabled")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Test numeric values
    assert_eq!(
        host_with_bools.vars.get("port").unwrap().as_u64().unwrap(),
        22
    );
    assert_eq!(
        host_with_bools.vars.get("ratio").unwrap().as_f64().unwrap(),
        0.75
    );

    // Test special character hosts
    assert!(inventory.hosts.contains_key("host-with-dashes"));
    assert!(inventory.hosts.contains_key("host_with_underscores"));
    assert!(inventory.hosts.contains_key("host.with.dots"));
}

#[tokio::test]
async fn test_validation_errors() {
    let parser = create_test_parser();

    // Test invalid patterns
    let result = parser.expand_host_pattern("web[05:01]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));

    // Test invalid bracket matching
    let result = parser.expand_host_pattern("web[01:03");
    assert!(result.is_err());

    // Test empty brackets
    let result = parser.expand_host_pattern("web[]");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_circular_dependency_detection() {
    let ini_content = r#"
[group1:children]
group2

[group2:children]
group1

[all]
localhost
"#;

    // Create a temporary file for this test
    use std::io::Write;
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    temp_file.write_all(ini_content.as_bytes()).unwrap();

    let parser = create_test_parser();
    let result = parser.parse(temp_file.path()).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::CircularGroupDependency { .. }
    ));
}

#[tokio::test]
async fn test_strict_mode() {
    let config = InventoryParserConfig {
        strict_mode: true,
        ..Default::default()
    };
    let parser = create_test_parser_with_config(config);

    // Test with duplicate hosts - should fail in strict mode
    let ini_content = r#"
[group1]
dup_host ansible_host=192.168.1.1

[group2]
dup_host ansible_host=192.168.1.2

[all]
dup_host
"#;

    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    temp_file.write_all(ini_content.as_bytes()).unwrap();

    let result = parser.parse(temp_file.path()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::DuplicateHost { .. }
    ));
}

#[tokio::test]
async fn test_pattern_expansion_limits() {
    let config = InventoryParserConfig {
        max_pattern_expansion: 5,
        ..Default::default()
    };
    let parser = create_test_parser_with_config(config);

    // This should exceed the limit
    let result = parser.expand_host_pattern("web[01:10]");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ParseError::InvalidHostPattern { .. }
    ));

    // This should be within the limit
    let result = parser.expand_host_pattern("web[01:05]");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 5);
}

#[tokio::test]
async fn test_host_variable_parsing() {
    let parser = create_test_parser();

    let vars = parser
        .parse_host_variables("ansible_host=192.168.1.10 ansible_port=22 custom='hello world'")
        .unwrap();

    assert_eq!(
        vars.get("ansible_host").unwrap().as_str().unwrap(),
        "192.168.1.10"
    );
    assert_eq!(vars.get("ansible_port").unwrap().as_str().unwrap(), "22");
    assert_eq!(vars.get("custom").unwrap().as_str().unwrap(), "hello world");

    // Test with complex quoting
    let vars = parser
        .parse_host_variables(r#"path="/var/log/app" command='echo "hello"' flag=true"#)
        .unwrap();

    assert_eq!(vars.get("path").unwrap().as_str().unwrap(), "/var/log/app");
    assert_eq!(
        vars.get("command").unwrap().as_str().unwrap(),
        r#"echo "hello""#
    );
    assert_eq!(vars.get("flag").unwrap().as_str().unwrap(), "true");
}

#[tokio::test]
async fn test_all_group_membership() {
    let parser = create_test_parser();
    let path = std::path::Path::new("tests/fixtures/inventories/complex.ini");

    let inventory = parser.parse(path).await.unwrap();

    // All hosts should be members of the 'all' group
    for (host_name, host) in &inventory.hosts {
        assert!(
            host.groups.contains(&"all".to_string()),
            "Host '{host_name}' is not in the 'all' group"
        );
    }

    // The 'all' group should contain all hosts
    let all_group = inventory.groups.get("all").unwrap();
    assert_eq!(all_group.hosts.len(), inventory.hosts.len());

    for host_name in inventory.hosts.keys() {
        assert!(
            all_group.hosts.contains(host_name),
            "Host '{host_name}' is not listed in the 'all' group"
        );
    }
}

#[tokio::test]
async fn test_connection_parameter_extraction() {
    let ini_content = r#"
[hosts]
host1 ansible_host=192.168.1.10 ansible_port=2222 ansible_user=custom
host2 ansible_ssh_host=192.168.1.11 ansible_ssh_port=22 ansible_ssh_user=ssh_user
host3 # No connection params

[all]
host1
host2
host3
"#;

    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    temp_file.write_all(ini_content.as_bytes()).unwrap();

    let parser = create_test_parser();
    let inventory = parser.parse(temp_file.path()).await.unwrap();

    let host1 = inventory.hosts.get("host1").unwrap();
    assert_eq!(host1.address, Some("192.168.1.10".to_string()));
    assert_eq!(host1.port, Some(2222));
    assert_eq!(host1.user, Some("custom".to_string()));

    let host2 = inventory.hosts.get("host2").unwrap();
    assert_eq!(host2.address, Some("192.168.1.11".to_string()));
    assert_eq!(host2.port, Some(22));
    assert_eq!(host2.user, Some("ssh_user".to_string()));

    let host3 = inventory.hosts.get("host3").unwrap();
    assert_eq!(host3.address, None);
    assert_eq!(host3.port, None);
    assert_eq!(host3.user, None);
}
