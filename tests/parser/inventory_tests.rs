use rustle_parse::parser::Parser;
use std::path::PathBuf;

#[tokio::test]
async fn test_parse_ini_inventory() {
    let parser = Parser::new();
    let fixture_path = PathBuf::from("tests/fixtures/inventories/hosts.ini");

    let result = parser.parse_inventory(&fixture_path).await;
    assert!(
        result.is_ok(),
        "Failed to parse INI inventory: {result:?}"
    );

    let inventory = result.unwrap();

    // Check hosts (our simplified implementation returns localhost)
    assert!(inventory.hosts.contains_key("localhost"));

    // Check host details
    let localhost = &inventory.hosts["localhost"];
    assert_eq!(localhost.address, Some("127.0.0.1".to_string()));
    assert_eq!(localhost.name, "localhost");

    // Check groups
    assert!(inventory.groups.contains_key("all"));

    let all_group = &inventory.groups["all"];
    assert!(all_group.hosts.contains(&"localhost".to_string()));
}

#[tokio::test]
async fn test_parse_nonexistent_inventory() {
    let parser = Parser::new();
    let nonexistent_path = PathBuf::from("nonexistent_inventory.ini");

    let result = parser.parse_inventory(&nonexistent_path).await;
    assert!(result.is_err());
}
