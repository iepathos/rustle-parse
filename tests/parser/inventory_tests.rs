use rustle_parse::parser::Parser;
use std::path::PathBuf;

#[tokio::test]
async fn test_parse_ini_inventory() {
    let parser = Parser::new();
    let fixture_path = PathBuf::from("tests/fixtures/inventories/hosts.ini");

    let result = parser.parse_inventory(&fixture_path).await;
    assert!(result.is_ok(), "Failed to parse INI inventory: {result:?}");

    let inventory = result.unwrap();

    // Check hosts from the fixture
    assert!(inventory.hosts.contains_key("web1"));
    assert!(inventory.hosts.contains_key("web2"));
    assert!(inventory.hosts.contains_key("db1"));
    assert!(inventory.hosts.contains_key("db2"));

    // Check host details
    let web1 = &inventory.hosts["web1"];
    assert_eq!(web1.address, Some("192.168.1.10".to_string()));
    assert_eq!(web1.name, "web1");

    // Check groups
    assert!(inventory.groups.contains_key("all"));

    let all_group = &inventory.groups["all"];
    assert!(all_group.hosts.contains(&"web1".to_string()));
    assert!(all_group.hosts.contains(&"web2".to_string()));
    assert!(all_group.hosts.contains(&"db1".to_string()));
    assert!(all_group.hosts.contains(&"db2".to_string()));
}

#[tokio::test]
async fn test_parse_nonexistent_inventory() {
    let parser = Parser::new();
    let nonexistent_path = PathBuf::from("nonexistent_inventory.ini");

    let result = parser.parse_inventory(&nonexistent_path).await;
    assert!(result.is_err());
}
