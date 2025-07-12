use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::{NamedTempFile, TempDir};

/// Helper function to create a temporary playbook file
fn create_temp_playbook(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    fs::write(&file, content).expect("Failed to write to temp file");
    file
}

/// Helper function to create a temporary inventory file
fn create_temp_inventory(content: &str) -> NamedTempFile {
    let file = NamedTempFile::new().expect("Failed to create temp file");
    fs::write(&file, content).expect("Failed to write to temp file");
    file
}

#[test]
fn test_cli_no_arguments() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No playbook file specified"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains(
        "Parse Ansible playbooks and inventory files",
    ));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_cli_simple_playbook_json_output() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--output").arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test playbook"))
        .stdout(predicate::str::contains("Test task"));
}

#[test]
fn test_cli_simple_playbook_yaml_output() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--output").arg("yaml");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("name: Test playbook"));
}

#[test]
fn test_cli_binary_output_not_implemented() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--output").arg("binary");

    cmd.assert().failure().stderr(predicate::str::contains(
        "Binary output format not yet implemented",
    ));
}

#[test]
fn test_cli_nonexistent_playbook() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("nonexistent_playbook.yml");

    cmd.assert().failure().stderr(
        predicate::str::contains("FileNotFound")
            .or(predicate::str::contains("No such file or directory")
                .or(predicate::str::contains("cannot find the file"))),
    );
}

#[test]
fn test_cli_invalid_yaml_playbook() {
    let invalid_playbook = r#"
---
- name: Invalid playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
      invalid_yaml: [unclosed bracket
"#;

    let playbook_file = create_temp_playbook(invalid_playbook);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Yaml").or(predicate::str::contains("parse")));
}

#[test]
fn test_cli_syntax_check_valid() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--syntax-check");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Syntax validation passed"));
}

#[test]
fn test_cli_syntax_check_invalid() {
    let invalid_playbook = r#"
---
- name: Invalid playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
      invalid_yaml: [unclosed bracket
"#;

    let playbook_file = create_temp_playbook(invalid_playbook);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--syntax-check");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Syntax validation failed"));
}

#[test]
fn test_cli_list_tasks() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: First task
      debug:
        msg: "Hello"
      tags: ["test", "debug"]
    - name: Second task
      shell: echo "World"
      when: ansible_os_family == "RedHat"
  handlers:
    - name: Restart service
      service:
        name: httpd
        state: restarted
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--list-tasks");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Play 1: Test playbook"))
        .stdout(predicate::str::contains("Task 1: First task (debug)"))
        .stdout(predicate::str::contains("Task 2: Second task (shell)"))
        .stdout(predicate::str::contains("Tags: test, debug"))
        .stdout(predicate::str::contains(
            "When: ansible_os_family == \"RedHat\"",
        ))
        .stdout(predicate::str::contains("Handlers:"))
        .stdout(predicate::str::contains(
            "Handler 1: Restart service (service)",
        ));
}

#[test]
fn test_cli_list_hosts_with_inventory() {
    let inventory_content = r#"
[webservers]
web1.example.com ansible_host=192.168.1.10 ansible_user=admin
web2.example.com ansible_host=192.168.1.11 ansible_user=admin

[databases]
db1.example.com ansible_host=192.168.1.20 ansible_port=5432

[webservers:vars]
http_port=80
max_clients=200
"#;

    let playbook_content = r#"
---
- name: Test playbook
  hosts: all
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let inventory_file = create_temp_inventory(inventory_content);
    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--inventory")
        .arg(inventory_file.path())
        .arg("--list-hosts");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("web1.example.com"))
        .stdout(predicate::str::contains("web2.example.com"))
        .stdout(predicate::str::contains("db1.example.com"))
        .stdout(predicate::str::contains("address: 192.168.1.10"))
        .stdout(predicate::str::contains("user: admin"))
        .stdout(predicate::str::contains("port: 5432"));
}

#[test]
fn test_cli_list_hosts_no_inventory() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--list-hosts");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No inventory file specified"));
}

#[test]
fn test_cli_extra_vars() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "{{ test_var }}"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--extra-vars")
        .arg("test_var=hello_world,number_var=42,bool_var=true");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello_world"));
}

#[test]
fn test_cli_cache_directory() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--cache-dir")
        .arg(cache_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test playbook"));
}

#[test]
fn test_cli_vault_password_file() {
    let vault_password = "test_password_123";
    let vault_file = NamedTempFile::new().unwrap();
    fs::write(&vault_file, vault_password).unwrap();

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--vault-password-file")
        .arg(vault_file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test playbook"));
}

#[test]
fn test_cli_vault_password_file_nonexistent() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--vault-password-file")
        .arg("nonexistent_vault_file.txt");

    cmd.assert().failure().stderr(
        predicate::str::contains("No such file or directory")
            .or(predicate::str::contains("cannot find the file")),
    );
}

#[test]
fn test_cli_dry_run() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--dry-run");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Parsing completed successfully"));
}

#[test]
fn test_cli_verbose_logging() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--verbose");

    cmd.assert().success().stderr(
        predicate::str::contains("DEBUG")
            .or(predicate::str::contains("Parsing completed successfully")),
    );
}

#[test]
fn test_cli_stdin_input_unsupported() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("-")
        .write_stdin("---\n- name: Test\n  hosts: localhost\n  tasks: []");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("stdin input"));
}

#[test]
fn test_cli_invalid_extra_vars_format() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    // Test with malformed extra vars (no equals sign)
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--extra-vars")
        .arg("invalid_format_no_equals");

    // Should still succeed as malformed vars are ignored
    cmd.assert().success();
}

#[test]
fn test_cli_complex_extra_vars() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    // Test with JSON values in extra vars
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--extra-vars")
        .arg(r#"str_var=hello,num_var=42,bool_var=true,json_var={"key":"value"}"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test playbook"));
}

#[test]
fn test_cli_vault_password_file_empty() {
    let vault_file = NamedTempFile::new().unwrap();
    // Write empty content to vault file
    fs::write(&vault_file, "").unwrap();

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--vault-password-file")
        .arg(vault_file.path());

    // Empty vault file should be handled gracefully
    cmd.assert().success();
}

#[test]
fn test_cli_vault_password_file_directory_error() {
    let temp_dir = TempDir::new().unwrap();

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--vault-password-file")
        .arg(temp_dir.path()); // Pass directory instead of file

    cmd.assert().failure().stderr(
        predicate::str::contains("Is a directory").or(predicate::str::contains("Access is denied")),
    );
}

#[test]
fn test_cli_vault_password_file_whitespace_only() {
    let vault_file = NamedTempFile::new().unwrap();
    // Write only whitespace to vault file
    fs::write(&vault_file, "   \n\t  \n  ").unwrap();

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--vault-password-file")
        .arg(vault_file.path());

    // Whitespace-only vault file should be handled gracefully (trimmed to empty)
    cmd.assert().success();
}

#[test]
fn test_cli_syntax_check_exit_code_on_failure() {
    let invalid_playbook = r#"
---
- name: Invalid playbook
  hosts: localhost
  tasks:
    - name: Invalid YAML structure
      module_name: [unclosed_array
"#;

    let playbook_file = create_temp_playbook(invalid_playbook);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--syntax-check");

    // Should exit with non-zero code for invalid syntax
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("YAML").or(predicate::str::contains("ERROR")));
}

#[test]
fn test_cli_list_hosts_invalid_inventory_format() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let invalid_inventory = r#"
{
  "invalid": "json structure for inventory"
  "missing_comma": true
}
"#;

    let playbook_file = create_temp_playbook(playbook_content);
    let inventory_file = create_temp_inventory(invalid_inventory);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--inventory")
        .arg(inventory_file.path())
        .arg("--list-hosts");

    // Should handle invalid inventory gracefully
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Json").or(predicate::str::contains("Error")));
}

#[test]
fn test_cli_binary_output_not_implemented_error() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello World"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--output").arg("binary");

    // Binary output should return not implemented error
    cmd.assert().failure().stderr(predicate::str::contains(
        "Binary output format not yet implemented",
    ));
}
