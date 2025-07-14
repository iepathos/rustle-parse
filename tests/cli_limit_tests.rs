use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn test_limit_single_host() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("web01")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    assert_eq!(hosts.len(), 1);
    assert!(hosts.contains_key("web01"));
}

#[test]
fn test_limit_group() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("webservers")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    assert_eq!(hosts.len(), 4);
    assert!(hosts.contains_key("web01"));
    assert!(hosts.contains_key("web02"));
    assert!(hosts.contains_key("web03"));
    assert!(hosts.contains_key("web-lb"));
}

#[test]
fn test_limit_glob_pattern() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("db-*")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    assert_eq!(hosts.len(), 4);
    assert!(hosts.contains_key("db-a"));
    assert!(hosts.contains_key("db-b"));
    assert!(hosts.contains_key("db-c"));
    assert!(hosts.contains_key("db-master"));
}

#[test]
fn test_limit_multiple_hosts() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("web01,db-master,redis1")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    assert_eq!(hosts.len(), 3);
    assert!(hosts.contains_key("web01"));
    assert!(hosts.contains_key("db-master"));
    assert!(hosts.contains_key("redis1"));
}

#[test]
fn test_limit_host_pattern() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/patterns.ini")
        .arg("--limit")
        .arg("web[02:04]")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    assert_eq!(hosts.len(), 3);
    assert!(hosts.contains_key("web02"));
    assert!(hosts.contains_key("web03"));
    assert!(hosts.contains_key("web04"));
}

#[test]
fn test_limit_group_prefix() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/patterns.ini")
        .arg("--limit")
        .arg(":numeric_patterns")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let hosts = json["inventory"]["hosts"].as_object().unwrap();
    // Should have web01-05, app1-3, api10-12
    assert_eq!(hosts.len(), 11);
}

#[test]
fn test_limit_nonexistent_host() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "No hosts matched the limit pattern",
        ));
}

#[test]
fn test_limit_with_list_hosts() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("--list-hosts")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("--limit")
        .arg("web01,web02")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(output.contains("web01:"));
    assert!(output.contains("web02:"));
    assert!(!output.contains("web03:"));
    assert!(!output.contains("db-master:"));
}

#[test]
fn test_limit_updates_all_group() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    let assert = cmd
        .arg("tests/fixtures/playbooks/simple.yml")
        .arg("-i")
        .arg("tests/fixtures/inventories/complex.ini")
        .arg("--limit")
        .arg("web01,web02")
        .assert()
        .success();

    let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(&output).unwrap();

    let all_group = &json["inventory"]["groups"]["all"];
    let all_hosts = all_group["hosts"].as_array().unwrap();
    assert_eq!(all_hosts.len(), 2);
    assert!(all_hosts.contains(&Value::String("web01".to_string())));
    assert!(all_hosts.contains(&Value::String("web02".to_string())));
}
