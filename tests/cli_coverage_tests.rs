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
fn test_cli_list_hosts_with_all_host_attributes() {
    let inventory_content = r#"
[webservers]
web1.example.com ansible_host=192.168.1.10 ansible_user=admin ansible_port=8022 custom_var=value1
web2.example.com ansible_user=deploy

[databases]
db1.example.com

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
        .stdout(predicate::str::contains("address: 192.168.1.10"))
        .stdout(predicate::str::contains("port: 8022"))
        .stdout(predicate::str::contains("user: admin"))
        .stdout(predicate::str::contains("custom_var: \"value1\""))
        .stdout(predicate::str::contains("web2.example.com"))
        .stdout(predicate::str::contains("user: deploy"))
        .stdout(predicate::str::contains("db1.example.com"));
}

#[test]
fn test_cli_list_tasks_no_handlers() {
    let playbook_content = r#"
---
- name: Test playbook without handlers
  hosts: localhost
  tasks:
    - name: Simple task
      debug:
        msg: "Hello"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--list-tasks");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Play 1: Test playbook without handlers",
        ))
        .stdout(predicate::str::contains("Task 1: Simple task (debug)"))
        .stdout(predicate::str::contains("Handlers:").not());
}

#[test]
fn test_cli_list_tasks_no_tags_no_when() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Task without tags or when
      debug:
        msg: "Hello"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--list-tasks");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Task 1: Task without tags or when (debug)",
        ))
        .stdout(predicate::str::contains("Tags:").not())
        .stdout(predicate::str::contains("When:").not());
}

#[test]
fn test_cli_extra_vars_empty_string() {
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
    cmd.arg(playbook_file.path()).arg("--extra-vars").arg("");

    cmd.assert().success();
}

#[test]
fn test_cli_extra_vars_complex_json_values() {
    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "{{ test_array }}"
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--extra-vars")
        .arg(r#"test_array=[1,2,3],test_obj={"nested":{"key":"value"}},test_null=null"#);

    cmd.assert().success();
}

#[test]
fn test_cli_extra_vars_whitespace_handling() {
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
        .arg("  test_var  =  value_with_spaces  ,  another_var = test  ");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("value_with_spaces"));
}

#[test]
fn test_cli_multiple_plays() {
    let playbook_content = r#"
---
- name: First play
  hosts: webservers
  tasks:
    - name: Task in first play
      debug:
        msg: "Web task"

- name: Second play
  hosts: databases
  tasks:
    - name: Task in second play
      debug:
        msg: "DB task"
    - name: Another task
      shell: echo "test"
  handlers:
    - name: Restart database
      service:
        name: postgresql
        state: restarted
"#;

    let playbook_file = create_temp_playbook(playbook_content);

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path()).arg("--list-tasks");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Play 1: First play"))
        .stdout(predicate::str::contains("Play 2: Second play"))
        .stdout(predicate::str::contains(
            "Task 1: Task in first play (debug)",
        ))
        .stdout(predicate::str::contains(
            "Task 1: Task in second play (debug)",
        ))
        .stdout(predicate::str::contains("Task 2: Another task (shell)"))
        .stdout(predicate::str::contains(
            "Handler 1: Restart database (service)",
        ));
}

#[test]
fn test_cli_vault_password_file_with_newline() {
    let vault_password = "test_password_123\n";
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
fn test_cli_dry_run_with_list_tasks() {
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
        .arg("--dry-run")
        .arg("--list-tasks");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Play 1: Test playbook"))
        .stdout(predicate::str::contains("Task 1: Test task (debug)"));
}

#[test]
fn test_cli_syntax_check_with_extra_options() {
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
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--syntax-check")
        .arg("--extra-vars")
        .arg("test_var=hello")
        .arg("--cache-dir")
        .arg(cache_dir.path())
        .arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Syntax validation passed"));
}

#[test]
fn test_cli_list_hosts_with_inventory_and_extra_flags() {
    let inventory_content = r#"
[all:vars]
global_var=test

[webservers]
web1.example.com
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
        .arg("--list-hosts")
        .arg("--verbose")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("web1.example.com"));
}

#[test]
fn test_cli_no_playbook_specified_error_path() {
    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg("--output").arg("json");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No playbook file specified"));
}

#[test]
fn test_cli_cache_with_vault_password() {
    let vault_password = "secure_password";
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
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--cache-dir")
        .arg(cache_dir.path())
        .arg("--vault-password-file")
        .arg(vault_file.path())
        .arg("--output")
        .arg("yaml");

    cmd.assert().success();
}

#[test]
fn test_cli_all_flags_combination() {
    let inventory_content = r#"
[test]
localhost
"#;

    let vault_password = "test123";
    let vault_file = NamedTempFile::new().unwrap();
    fs::write(&vault_file, vault_password).unwrap();

    let playbook_content = r#"
---
- name: Complex test
  hosts: test
  tasks:
    - name: Test task
      debug:
        msg: "{{ custom_var }}"
"#;

    let inventory_file = create_temp_inventory(inventory_content);
    let playbook_file = create_temp_playbook(playbook_content);
    let cache_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("rustle-parse").unwrap();
    cmd.arg(playbook_file.path())
        .arg("--inventory")
        .arg(inventory_file.path())
        .arg("--extra-vars")
        .arg("custom_var=test_value")
        .arg("--cache-dir")
        .arg(cache_dir.path())
        .arg("--vault-password-file")
        .arg(vault_file.path())
        .arg("--verbose")
        .arg("--output")
        .arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Complex test"))
        .stdout(predicate::str::contains("test_value"));
}
