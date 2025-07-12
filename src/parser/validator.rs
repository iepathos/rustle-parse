use crate::parser::error::ParseError;
use std::path::Path;
use tokio::fs;

pub async fn validate_playbook_syntax(path: &Path) -> Result<(), ParseError> {
    let content = fs::read_to_string(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ParseError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            }
        } else {
            ParseError::Io(e)
        }
    })?;

    // Check for empty or whitespace-only content
    if content.trim().is_empty() {
        return Err(ParseError::InvalidStructure {
            message: "Playbook file cannot be empty".to_string(),
        });
    }

    // Basic YAML syntax validation
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)?;

    // Check if it's a valid playbook structure (should be an array)
    if !yaml_value.is_sequence() && !yaml_value.is_null() {
        return Err(ParseError::InvalidStructure {
            message: "Playbook must be a YAML array of plays".to_string(),
        });
    }

    // TODO: Add more Ansible-specific validation rules

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixtures::{INVALID_YAML, SIMPLE_PLAYBOOK_YAML};
    use crate::testing::helpers::create_temp_file;

    #[tokio::test]
    async fn test_validate_playbook_syntax_valid_file() {
        let temp_file = create_temp_file(SIMPLE_PLAYBOOK_YAML).unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_invalid_yaml() {
        let temp_file = create_temp_file(INVALID_YAML).unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::Yaml(_)));
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_file_not_found() {
        let nonexistent_path = Path::new("/nonexistent/file.yml");
        let result = validate_playbook_syntax(nonexistent_path).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::FileNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_empty_file() {
        let temp_file = create_temp_file("").unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        // Empty file should be invalid for playbooks
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_whitespace_only() {
        let temp_file = create_temp_file("   \n     \n   ").unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        // Whitespace-only file should be invalid for playbooks
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_complex_valid_yaml() {
        let complex_yaml = r#"
---
- name: Complex test playbook
  hosts: "{{ target_hosts | default('all') }}"
  become: yes
  vars:
    app_name: test-app
    app_config:
      - name: config1
        value: "{{ app_name }}-value1"
      - name: config2
        value: "{{ app_name }}-value2"
  tasks:
    - name: Install packages
      package:
        name: "{{ item }}"
        state: present
      loop: "{{ packages | default([]) }}"
      when: packages is defined
      tags:
        - install
        - packages
"#;
        let temp_file = create_temp_file(complex_yaml).unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_malformed_scenarios() {
        let malformed_scenarios = vec![
            ("unclosed_bracket", "key: [unclosed"),
            ("unterminated_string", r#"key: "unterminated"#),
            ("invalid_character", "key: value\x00"),
        ];

        for (name, content) in malformed_scenarios {
            let temp_file = create_temp_file(content).unwrap();
            let result = validate_playbook_syntax(temp_file.path()).await;
            assert!(
                result.is_err(),
                "Expected error for malformed YAML scenario: {name}"
            );
            assert!(
                matches!(result.unwrap_err(), ParseError::Yaml(_)),
                "Expected YamlError for scenario: {name}"
            );
        }
    }

    #[tokio::test]
    async fn test_validate_playbook_syntax_non_array_structure() {
        let non_array_yaml = r#"
name: This is not a playbook
description: A playbook must be an array of plays
hosts: localhost
tasks:
  - name: Example task
    debug:
      msg: "This won't work"
"#;
        let temp_file = create_temp_file(non_array_yaml).unwrap();
        let result = validate_playbook_syntax(temp_file.path()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidStructure { message } => {
                assert_eq!(message, "Playbook must be a YAML array of plays");
            }
            _ => panic!("Expected ParseError::InvalidStructure"),
        }
    }
}
