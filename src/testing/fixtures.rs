//! Test fixtures for consistent testing across the codebase

use crate::types::parsed::{
    ParsedGroup, ParsedHost, ParsedInventory, ParsedPlay, ParsedPlaybook, ParsedTask,
};
use std::collections::HashMap;

/// Create a simple test inventory for testing
pub fn create_test_inventory() -> ParsedInventory {
    let mut hosts = HashMap::new();
    let mut groups = HashMap::new();

    // Create test hosts
    hosts.insert(
        "web1".to_string(),
        ParsedHost {
            name: "web1".to_string(),
            address: Some("192.168.1.10".to_string()),
            port: Some(22),
            user: None,
            vars: HashMap::new(),
            groups: vec!["webservers".to_string()],
        },
    );

    hosts.insert(
        "web2".to_string(),
        ParsedHost {
            name: "web2".to_string(),
            address: Some("192.168.1.11".to_string()),
            port: Some(22),
            user: None,
            vars: HashMap::new(),
            groups: vec!["webservers".to_string()],
        },
    );

    hosts.insert(
        "db1".to_string(),
        ParsedHost {
            name: "db1".to_string(),
            address: Some("192.168.1.20".to_string()),
            port: Some(22),
            user: None,
            vars: {
                let mut vars = HashMap::new();
                vars.insert(
                    "mysql_port".to_string(),
                    serde_json::Value::Number(3306.into()),
                );
                vars
            },
            groups: vec!["databases".to_string()],
        },
    );

    // Create test groups
    groups.insert(
        "webservers".to_string(),
        ParsedGroup {
            name: "webservers".to_string(),
            hosts: vec!["web1".to_string(), "web2".to_string()],
            children: vec![],
            vars: HashMap::new(),
        },
    );

    groups.insert(
        "databases".to_string(),
        ParsedGroup {
            name: "databases".to_string(),
            hosts: vec!["db1".to_string()],
            children: vec![],
            vars: HashMap::new(),
        },
    );

    groups.insert(
        "all".to_string(),
        ParsedGroup {
            name: "all".to_string(),
            hosts: vec![],
            children: vec!["webservers".to_string(), "databases".to_string()],
            vars: HashMap::new(),
        },
    );

    ParsedInventory {
        hosts,
        groups,
        variables: HashMap::new(),
    }
}

/// Create a simple test playbook for testing
pub fn create_test_playbook() -> ParsedPlaybook {
    use crate::types::parsed::{HostPattern, PlaybookMetadata};

    let tasks = vec![
        ParsedTask {
            id: "task1".to_string(),
            name: "Install nginx".to_string(),
            module: "package".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert(
                    "name".to_string(),
                    serde_json::Value::String("nginx".to_string()),
                );
                args.insert(
                    "state".to_string(),
                    serde_json::Value::String("present".to_string()),
                );
                args
            },
            vars: HashMap::new(),
            when: None,
            loop_items: None,
            tags: vec![],
            notify: vec![],
            changed_when: None,
            failed_when: None,
            ignore_errors: false,
            delegate_to: None,
            dependencies: vec![],
        },
        ParsedTask {
            id: "task2".to_string(),
            name: "Start nginx service".to_string(),
            module: "service".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert(
                    "name".to_string(),
                    serde_json::Value::String("nginx".to_string()),
                );
                args.insert(
                    "state".to_string(),
                    serde_json::Value::String("started".to_string()),
                );
                args.insert("enabled".to_string(), serde_json::Value::Bool(true));
                args
            },
            vars: HashMap::new(),
            when: None,
            loop_items: None,
            tags: vec!["service".to_string()],
            notify: vec![],
            changed_when: None,
            failed_when: None,
            ignore_errors: false,
            delegate_to: None,
            dependencies: vec![],
        },
    ];

    let play = ParsedPlay {
        name: "Configure web servers".to_string(),
        hosts: HostPattern::Single("webservers".to_string()),
        tasks,
        vars: HashMap::new(),
        handlers: vec![],
        roles: vec![],
        strategy: crate::types::parsed::ExecutionStrategy::default(),
        serial: None,
        max_fail_percentage: None,
    };

    ParsedPlaybook {
        metadata: PlaybookMetadata {
            file_path: "test_playbook.yml".to_string(),
            version: Some("1.0".to_string()),
            created_at: chrono::Utc::now(),
            checksum: "test_checksum".to_string(),
        },
        plays: vec![play],
        variables: HashMap::new(),
        facts_required: false,
        vault_ids: vec![],
    }
}

/// Sample YAML content for testing playbook parsing
pub const SIMPLE_PLAYBOOK_YAML: &str = r#"
---
- name: Configure web servers
  hosts: webservers
  tasks:
    - name: Install nginx
      package:
        name: nginx
        state: present
    
    - name: Start nginx service
      service:
        name: nginx
        state: started
        enabled: yes
      tags:
        - service
"#;

/// Sample INI inventory content for testing
pub const SIMPLE_INI_INVENTORY: &str = r#"
[webservers]
web1 ansible_host=192.168.1.10
web2 ansible_host=192.168.1.11

[databases]
db1 ansible_host=192.168.1.20 mysql_port=3306

[all:children]
webservers
databases
"#;

/// Complex playbook YAML with various features for comprehensive testing
pub const COMPLEX_PLAYBOOK_YAML: &str = r#"
---
- name: Complex playbook with multiple features
  hosts: "{{ target_hosts | default('all') }}"
  vars:
    app_name: "my-app"
    app_version: "1.0.0"
    config_file: "/etc/{{ app_name }}/config.yml"
  
  tasks:
    - name: Create application directory
      file:
        path: "/opt/{{ app_name }}"
        state: directory
        mode: '0755'
      when: app_name is defined
      tags:
        - setup
        - filesystem
    
    - name: Template configuration file
      template:
        src: "{{ app_name }}.conf.j2"
        dest: "{{ config_file }}"
        backup: yes
      notify:
        - restart application
      tags:
        - config
    
    - name: Install dependencies
      package:
        name: "{{ item }}"
        state: present
      loop:
        - python3
        - python3-pip
        - git
      tags:
        - dependencies

  handlers:
    - name: restart application
      service:
        name: "{{ app_name }}"
        state: restarted
"#;

/// Invalid YAML for testing error handling
pub const INVALID_YAML: &str = r#"
---
- name: Invalid playbook
  hosts: test
  tasks:
    - name: Unclosed bracket
      debug:
        msg: "This has invalid YAML [
"#;

/// Sample vault encrypted content for testing
pub const VAULT_ENCRYPTED_CONTENT: &str = r#"
$ANSIBLE_VAULT;1.1;AES256
66396439633937376662326235343938376666663861326639323834646265383562623762396133
3933316164366231363565396262636639653163636264370a623061386462663061613264623137
36643735316636343939623634376463623638306265333335323638613163616264343033663736
3266326365616566650a323433346361313166333364333039326439623831653664323634623461
34623665383735313139343139316534366166313439363065316464386434666164
"#;
