use pretty_assertions::assert_eq;
use rustle_parse::parser::error::ParseError;
use rustle_parse::parser::playbook::PlaybookParser;
use rustle_parse::parser::template::TemplateEngine;
use rustle_parse::types::parsed::*;
use std::collections::HashMap;
use std::path::Path;
use tempfile::NamedTempFile;

fn create_temp_playbook(content: &str) -> NamedTempFile {
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
async fn test_parse_complex_playbook_with_multiple_plays() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: First play
  hosts: webservers
  vars:
    http_port: 80
    max_clients: 200
  tasks:
    - name: Install Apache
      package:
        name: apache2
        state: present
      tags: ["install", "apache"]
    - name: Start Apache
      service:
        name: apache2
        state: started
        enabled: yes
      notify: restart apache
  handlers:
    - name: restart apache
      service:
        name: apache2
        state: restarted

- name: Second play
  hosts: databases
  vars:
    db_port: 5432
  tasks:
    - name: Install PostgreSQL
      package:
        name: postgresql
        state: present
      when: ansible_os_family == "Debian"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays.len(), 2);

    // First play
    let first_play = &playbook.plays[0];
    assert_eq!(first_play.name, "First play");
    assert_eq!(
        first_play.hosts,
        HostPattern::Single("webservers".to_string())
    );
    assert_eq!(first_play.tasks.len(), 2);
    assert_eq!(first_play.handlers.len(), 1);

    let install_task = &first_play.tasks[0];
    assert_eq!(install_task.name, "Install Apache");
    assert_eq!(install_task.module, "package");
    assert!(install_task.tags.contains(&"install".to_string()));
    assert!(install_task.tags.contains(&"apache".to_string()));

    let start_task = &first_play.tasks[1];
    assert_eq!(start_task.name, "Start Apache");
    assert!(start_task.notify.contains(&"restart apache".to_string()));

    // Second play
    let second_play = &playbook.plays[1];
    assert_eq!(second_play.name, "Second play");
    assert_eq!(
        second_play.hosts,
        HostPattern::Single("databases".to_string())
    );
    assert_eq!(second_play.tasks.len(), 1);

    let pg_task = &second_play.tasks[0];
    assert_eq!(
        pg_task.when,
        Some("ansible_os_family == \"Debian\"".to_string())
    );
}

#[tokio::test]
async fn test_parse_playbook_with_roles() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Deploy application
  hosts: all
  roles:
    - common
    - name: webserver
      src: git+https://github.com/example/webserver.git
      version: "1.2.0"
      vars:
        port: 8080
      tags: ["web"]
    - database
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays.len(), 1);
    let play = &playbook.plays[0];
    assert_eq!(play.roles.len(), 3);

    // Simple role
    let common_role = &play.roles[0];
    assert_eq!(common_role.name, "common");
    assert_eq!(common_role.src, None);
    assert_eq!(common_role.version, None);

    // Complex role
    let webserver_role = &play.roles[1];
    assert_eq!(webserver_role.name, "webserver");
    assert_eq!(
        webserver_role.src,
        Some("git+https://github.com/example/webserver.git".to_string())
    );
    assert_eq!(webserver_role.version, Some("1.2.0".to_string()));
    assert!(webserver_role.tags.contains(&"web".to_string()));
}

#[tokio::test]
async fn test_parse_playbook_with_variables_and_templating() {
    let template_engine = TemplateEngine::new();
    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "environment".to_string(),
        serde_json::Value::String("production".to_string()),
    );
    extra_vars.insert(
        "app_version".to_string(),
        serde_json::Value::String("2.1.0".to_string()),
    );

    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Deploy {{ app_version }} to {{ environment }}
  hosts: "{{ environment }}-servers"
  vars:
    app_name: myapp
    config_file: "/etc/{{ app_name }}/config.yaml"
  tasks:
    - name: Create config directory
      file:
        path: "/etc/{{ app_name }}"
        state: directory
        mode: "0755"
    - name: Deploy config
      template:
        src: "config.j2"
        dest: "{{ config_file }}"
        backup: yes
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    // Check that extra vars are included
    assert_eq!(
        playbook.variables["environment"],
        serde_json::Value::String("production".to_string())
    );
    assert_eq!(
        playbook.variables["app_version"],
        serde_json::Value::String("2.1.0".to_string())
    );

    let play = &playbook.plays[0];
    assert_eq!(play.name, "Deploy 2.1.0 to production");
    assert_eq!(
        play.hosts,
        HostPattern::Single("production-servers".to_string())
    );

    // Check templated task arguments
    let config_task = &play.tasks[1];
    assert_eq!(
        config_task.args["dest"],
        serde_json::Value::String("/etc/myapp/config.yaml".to_string())
    );
}

#[tokio::test]
async fn test_parse_playbook_with_loops_and_conditionals() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Complex task scenarios
  hosts: localhost
  tasks:
    - name: Install packages
      package:
        name: "{{ item }}"
        state: present
      loop:
        - apache2
        - mysql-server
        - php
      when: ansible_os_family == "Debian"
      tags: ["packages"]

    - name: Conditional task
      debug:
        msg: "This runs on RedHat family"
      when: ansible_os_family == "RedHat"
      changed_when: false
      failed_when: false
      ignore_errors: true

    - name: Task with delegation
      command: "uptime"
      delegate_to: "{{ item }}"
      loop:
        - server1
        - server2
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    assert_eq!(play.tasks.len(), 3);

    // Loop task
    let loop_task = &play.tasks[0];
    assert_eq!(loop_task.name, "Install packages");
    assert!(loop_task.loop_items.is_some());
    assert_eq!(
        loop_task.when,
        Some("ansible_os_family == \"Debian\"".to_string())
    );
    assert!(loop_task.tags.contains(&"packages".to_string()));

    // Conditional task
    let conditional_task = &play.tasks[1];
    assert_eq!(
        conditional_task.when,
        Some("ansible_os_family == \"RedHat\"".to_string())
    );
    assert_eq!(conditional_task.changed_when, Some("false".to_string()));
    assert_eq!(conditional_task.failed_when, Some("false".to_string()));
    assert_eq!(conditional_task.ignore_errors, true);

    // Delegation task
    let delegation_task = &play.tasks[2];
    assert!(delegation_task.delegate_to.is_some());
    assert!(delegation_task.loop_items.is_some());
}

#[tokio::test]
async fn test_parse_playbook_with_different_host_patterns() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Play with multiple hosts
  hosts: 
    - webservers
    - databases
  tasks:
    - name: Test task
      ping:

- name: Play for all hosts
  hosts: all
  tasks:
    - name: Another test
      debug:
        msg: "Hello all"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays.len(), 2);

    // Multiple hosts
    let first_play = &playbook.plays[0];
    assert_eq!(
        first_play.hosts,
        HostPattern::Multiple(vec!["webservers".to_string(), "databases".to_string()])
    );

    // All hosts
    let second_play = &playbook.plays[1];
    assert_eq!(second_play.hosts, HostPattern::All);
}

#[tokio::test]
async fn test_parse_playbook_with_execution_strategy() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Play with custom strategy
  hosts: all
  strategy: free
  serial: 3
  max_fail_percentage: 25
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    assert_eq!(play.strategy, ExecutionStrategy::Free);
    assert_eq!(play.serial, Some(3));
    assert_eq!(play.max_fail_percentage, Some(25.0));
}

#[tokio::test]
async fn test_parse_playbook_with_setup_task() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Play with fact gathering
  hosts: all
  tasks:
    - name: Gather facts
      setup:
    - name: Use facts
      debug:
        msg: "OS: {{ ansible_os_family }}"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    // Should set facts_required to true
    assert_eq!(playbook.facts_required, true);

    let play = &playbook.plays[0];
    let setup_task = &play.tasks[0];
    assert_eq!(setup_task.module, "setup");
}

#[tokio::test]
async fn test_parse_playbook_with_all_module_types() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test all module types
  hosts: localhost
  tasks:
    - name: Shell command
      shell: "echo hello"
    - name: Copy file
      copy:
        src: /source/file
        dest: /dest/file
        mode: "0644"
    - name: Service management
      service:
        name: nginx
        state: started
    - name: Package installation
      apt:
        name: vim
        state: present
    - name: Git checkout
      git:
        repo: https://github.com/example/repo.git
        dest: /opt/repo
    - name: URI call
      uri:
        url: https://api.example.com/status
        method: GET
    - name: User management
      user:
        name: testuser
        shell: /bin/bash
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    assert_eq!(play.tasks.len(), 7);

    let modules = play.tasks.iter().map(|t| &t.module).collect::<Vec<_>>();
    assert!(modules.contains(&&"shell".to_string()));
    assert!(modules.contains(&&"copy".to_string()));
    assert!(modules.contains(&&"service".to_string()));
    assert!(modules.contains(&&"apt".to_string()));
    assert!(modules.contains(&&"git".to_string()));
    assert!(modules.contains(&&"uri".to_string()));
    assert!(modules.contains(&&"user".to_string()));
}

#[tokio::test]
async fn test_parse_playbook_missing_file() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let nonexistent_path = Path::new("/nonexistent/playbook.yml");
    let result = parser.parse(nonexistent_path).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::FileNotFound { path } => {
            assert_eq!(path, "/nonexistent/playbook.yml");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_parse_playbook_invalid_yaml() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let invalid_yaml = r#"
---
- name: Invalid playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
      invalid_yaml: [unclosed bracket
"#;

    let temp_file = create_temp_playbook(invalid_yaml);
    let result = parser.parse(temp_file.path()).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ParseError::Yaml(_)));
}

#[tokio::test]
async fn test_parse_task_with_no_module() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let invalid_playbook = r#"
---
- name: Invalid task
  hosts: localhost
  tasks:
    - name: Task without module
      some_unknown_key: value
"#;

    let temp_file = create_temp_playbook(invalid_playbook);
    let result = parser.parse(temp_file.path()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::InvalidStructure { message } => {
            assert!(message.contains("No valid module found in task"));
        }
        _ => panic!("Expected InvalidStructure error"),
    }
}

#[tokio::test]
async fn test_parse_playbook_with_string_module_args() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test string module args
  hosts: localhost
  tasks:
    - name: Shell with string
      shell: "ls -la /tmp"
    - name: Command with string
      command: "uptime"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];

    let shell_task = &play.tasks[0];
    assert_eq!(shell_task.module, "shell");
    assert_eq!(
        shell_task.args["_raw_params"],
        serde_json::Value::String("ls -la /tmp".to_string())
    );

    let command_task = &play.tasks[1];
    assert_eq!(command_task.module, "command");
    assert_eq!(
        command_task.args["_raw_params"],
        serde_json::Value::String("uptime".to_string())
    );
}

#[tokio::test]
async fn test_parse_playbook_with_complex_args() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test complex args
  hosts: localhost
  tasks:
    - name: Copy with complex args
      copy:
        src: "/source/file"
        dest: "/dest/file"
        mode: "0644"
        owner: "root"
        group: "root"
        backup: yes
        validate: "visudo -cf %s"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    let copy_task = &play.tasks[0];

    assert_eq!(
        copy_task.args["src"],
        serde_json::Value::String("/source/file".to_string())
    );
    assert_eq!(
        copy_task.args["dest"],
        serde_json::Value::String("/dest/file".to_string())
    );
    assert_eq!(
        copy_task.args["mode"],
        serde_json::Value::String("0644".to_string())
    );
    assert_eq!(copy_task.args["backup"], serde_json::Value::Bool(true));
}

#[tokio::test]
async fn test_parse_playbook_metadata() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test playbook
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    // Check metadata
    assert!(playbook.metadata.file_path.contains("tmp")); // Temp file path
    assert!(!playbook.metadata.checksum.is_empty());
    assert!(playbook.metadata.created_at.timestamp() > 0);
    assert_eq!(playbook.metadata.version, None);
}

#[tokio::test]
async fn test_parse_playbook_with_unnamed_elements() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- hosts: localhost
  tasks:
    - debug:
        msg: "Unnamed task"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    assert_eq!(play.name, "Unnamed play");

    let task = &play.tasks[0];
    assert_eq!(task.name, "Unnamed task");
    assert!(task.id.starts_with("task_"));
}

#[tokio::test]
async fn test_parse_empty_playbook() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let empty_content = "---\n[]";

    let temp_file = create_temp_playbook(empty_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    assert_eq!(playbook.plays.len(), 0);
    assert_eq!(playbook.facts_required, false);
    assert!(playbook.vault_ids.is_empty());
}

#[tokio::test]
async fn test_parse_playbook_with_template_errors() {
    let template_engine = TemplateEngine::new();
    let extra_vars = HashMap::new(); // No variables defined

    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test template errors
  hosts: localhost
  tasks:
    - name: Task with undefined variable
      debug:
        msg: "{{ undefined_var | mandatory }}"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let result = parser.parse(temp_file.path()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ParseError::Template { .. } => {
            // Expected template error
        }
        _ => panic!("Expected Template error"),
    }
}

#[tokio::test]
async fn test_parse_playbook_host_pattern_fallback() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Play without hosts
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    assert_eq!(play.hosts, HostPattern::Single("localhost".to_string())); // Should default to localhost
}

#[tokio::test]
async fn test_parse_playbook_with_task_vars() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test task vars
  hosts: localhost
  tasks:
    - name: Task with vars
      debug:
        msg: "{{ local_var }}"
      vars:
        local_var: "task_value"
        another_var: 42
"#;

    let temp_file = create_temp_playbook(playbook_content);
    let playbook = parser.parse(temp_file.path()).await.unwrap();

    let play = &playbook.plays[0];
    let task = &play.tasks[0];

    assert!(task.vars.contains_key("local_var"));
    assert_eq!(
        task.vars["local_var"],
        serde_json::Value::String("task_value".to_string())
    );
    assert_eq!(
        task.vars["another_var"],
        serde_json::Value::Number(serde_json::Number::from(42))
    );
}

#[tokio::test]
async fn test_parse_playbook_checksum_consistency() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_content = r#"
---
- name: Test checksum
  hosts: localhost
  tasks:
    - name: Test task
      debug:
        msg: "Hello"
"#;

    let temp_file = create_temp_playbook(playbook_content);

    // Parse the same file twice
    let playbook1 = parser.parse(temp_file.path()).await.unwrap();
    let playbook2 = parser.parse(temp_file.path()).await.unwrap();

    // Checksums should be identical
    assert_eq!(playbook1.metadata.checksum, playbook2.metadata.checksum);
}

#[tokio::test]
async fn test_parse_system_facts_playbook() {
    let (template_engine, extra_vars) = setup_parser();
    let parser = PlaybookParser::new(&template_engine, &extra_vars);

    let playbook_path = Path::new("tests/fixtures/playbooks/system-facts-playbook.yml");
    let playbook = parser.parse(playbook_path).await.unwrap();

    // Verify metadata
    assert_eq!(
        playbook.metadata.file_path,
        "tests/fixtures/playbooks/system-facts-playbook.yml"
    );
    assert_eq!(
        playbook.metadata.checksum,
        "6dc9ca8307b63431f583dc81903f32d46def91875653464c5a8297797eb385ad"
    );
    assert_eq!(playbook.facts_required, true);

    // Verify play structure
    assert_eq!(playbook.plays.len(), 1);
    let play = &playbook.plays[0];
    assert_eq!(play.name, "System facts gathering playbook");
    assert_eq!(play.hosts, HostPattern::All);

    // Verify tasks
    assert_eq!(play.tasks.len(), 3);

    // Task 0: Gather system facts
    let gather_task = &play.tasks[0];
    assert_eq!(gather_task.name, "Gather system facts");
    assert_eq!(gather_task.module, "setup");
    assert_eq!(
        gather_task.args["gather_subset"],
        serde_json::Value::String("all".to_string())
    );
    assert_eq!(
        gather_task.args["gather_timeout"],
        serde_json::Value::Number(serde_json::Number::from(10))
    );
    assert_eq!(
        gather_task.tags,
        vec![
            "facts".to_string(),
            "setup".to_string(),
            "system".to_string()
        ]
    );

    // Task 1: Display gathered facts (template variables are resolved to empty strings)
    let display_task = &play.tasks[1];
    assert_eq!(display_task.name, "Display gathered facts");
    assert_eq!(display_task.module, "debug");
    assert_eq!(
        display_task.args["msg"],
        serde_json::Value::String("System: , OS Family: , Architecture: ".to_string())
    );

    // Task 2: Linux-only task (template variables are resolved to empty strings)
    let linux_task = &play.tasks[2];
    assert_eq!(linux_task.name, "Task for Linux systems only");
    assert_eq!(linux_task.module, "debug");
    assert_eq!(
        linux_task.args["msg"],
        serde_json::Value::String("This is a Linux system with  CPU cores".to_string())
    );
    assert_eq!(
        linux_task.when,
        Some("ansible_system == \"Linux\"".to_string())
    );
}
