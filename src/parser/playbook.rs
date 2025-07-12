use crate::parser::error::ParseError;
use crate::parser::template::TemplateEngine;
use crate::types::parsed::*;
use chrono::Utc;
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub struct PlaybookParser<'a> {
    template_engine: &'a TemplateEngine,
    extra_vars: &'a HashMap<String, serde_json::Value>,
}

impl<'a> PlaybookParser<'a> {
    pub fn new(
        template_engine: &'a TemplateEngine,
        extra_vars: &'a HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            template_engine,
            extra_vars,
        }
    }

    pub async fn parse(&self, path: &Path) -> Result<ParsedPlaybook, ParseError> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ParseError::FileNotFound {
                    path: path.to_string_lossy().to_string(),
                }
            } else {
                ParseError::Io(e)
            }
        })?;

        // Calculate checksum
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        // Parse YAML - Ansible playbooks are arrays of plays
        let raw_plays: Vec<RawPlay> = serde_yaml::from_str(&content)?;

        // Transform to parsed format
        let mut parsed_plays = Vec::new();
        let mut playbook_vars = HashMap::new();
        let mut facts_required = false;
        let vault_ids = Vec::new(); // TODO: Extract vault IDs from content

        // Merge extra vars
        playbook_vars.extend(self.extra_vars.clone());

        // Process each play
        for raw_play in raw_plays {
            let parsed_play = self.parse_play(raw_play, &playbook_vars).await?;

            // Check if any task requires facts
            if parsed_play
                .tasks
                .iter()
                .any(|t| t.module == "setup" || t.module == "gather_facts")
            {
                facts_required = true;
            }

            parsed_plays.push(parsed_play);
        }

        // No global playbook vars in this simple structure

        let metadata = PlaybookMetadata {
            file_path: path.to_string_lossy().to_string(),
            version: None, // TODO: Extract version from playbook if present
            created_at: Utc::now(),
            checksum,
        };

        Ok(ParsedPlaybook {
            metadata,
            plays: parsed_plays,
            variables: playbook_vars,
            facts_required,
            vault_ids,
        })
    }

    async fn parse_play(
        &self,
        raw_play: RawPlay,
        global_vars: &HashMap<String, serde_json::Value>,
    ) -> Result<ParsedPlay, ParseError> {
        let mut play_vars = global_vars.clone();

        // Merge play vars and render any templates in them
        if let Some(vars) = raw_play.vars {
            // First pass: add all raw variables
            for (key, value) in &vars {
                play_vars.insert(key.clone(), value.clone());
            }

            // Second pass: render templates that may reference other variables
            for (key, value) in vars {
                let rendered_value = self.template_engine.render_value(&value, &play_vars)?;
                play_vars.insert(key, rendered_value);
            }
        }

        // Parse hosts pattern and render templates
        let hosts = match raw_play.hosts {
            Some(RawHostPattern::Single(host)) => {
                let rendered_host = if host.contains("{{") && host.contains("}}") {
                    self.template_engine.render_string(&host, &play_vars)?
                } else {
                    host
                };

                if rendered_host == "all" {
                    HostPattern::All
                } else {
                    HostPattern::Single(rendered_host)
                }
            }
            Some(RawHostPattern::Multiple(hosts)) => {
                let mut rendered_hosts = Vec::new();
                for host in hosts {
                    let rendered_host = if host.contains("{{") && host.contains("}}") {
                        self.template_engine.render_string(&host, &play_vars)?
                    } else {
                        host
                    };
                    rendered_hosts.push(rendered_host);
                }
                HostPattern::Multiple(rendered_hosts)
            }
            Some(RawHostPattern::All) => HostPattern::All,
            None => HostPattern::All,
        };

        // Parse tasks
        let mut tasks = Vec::new();
        if let Some(raw_tasks) = raw_play.tasks {
            for (index, raw_task) in raw_tasks.into_iter().enumerate() {
                let task = self.parse_task(raw_task, &play_vars, index).await?;
                tasks.push(task);
            }
        }

        // Parse handlers
        let mut handlers = Vec::new();
        if let Some(raw_handlers) = raw_play.handlers {
            for (index, raw_handler) in raw_handlers.into_iter().enumerate() {
                let handler = self.parse_task(raw_handler, &play_vars, index).await?;
                handlers.push(handler);
            }
        }

        // Parse roles
        let mut roles = Vec::new();
        if let Some(raw_roles) = raw_play.roles {
            for raw_role in raw_roles {
                let role = self.parse_role(raw_role)?;
                roles.push(role);
            }
        }

        // Render play name through template engine
        let rendered_name = if let Some(name) = raw_play.name {
            if name.contains("{{") && name.contains("}}") {
                self.template_engine.render_string(&name, &play_vars)?
            } else {
                name
            }
        } else {
            "Unnamed play".to_string()
        };

        Ok(ParsedPlay {
            name: rendered_name,
            hosts,
            vars: play_vars,
            tasks,
            handlers,
            roles,
            strategy: raw_play.strategy.unwrap_or_default(),
            serial: raw_play.serial,
            max_fail_percentage: raw_play.max_fail_percentage,
        })
    }

    async fn parse_task(
        &self,
        raw_task: RawTask,
        vars: &HashMap<String, serde_json::Value>,
        index: usize,
    ) -> Result<ParsedTask, ParseError> {
        let id = raw_task
            .id
            .clone()
            .unwrap_or_else(|| format!("task_{index}"));
        let name = raw_task
            .name
            .clone()
            .unwrap_or_else(|| "Unnamed task".to_string());

        // Find the module and its arguments
        let (module, args) = self.extract_module_and_args(&raw_task)?;

        // Render templates in args
        let rendered_args = self.render_task_args(args, vars).await?;

        Ok(ParsedTask {
            id,
            name,
            module,
            args: rendered_args,
            vars: raw_task.vars.unwrap_or_default(),
            when: raw_task.when,
            loop_items: raw_task.loop_items,
            tags: raw_task.tags.unwrap_or_default(),
            notify: raw_task.notify.unwrap_or_default(),
            changed_when: raw_task.changed_when,
            failed_when: raw_task.failed_when,
            ignore_errors: raw_task.ignore_errors.unwrap_or(false),
            delegate_to: raw_task.delegate_to,
            dependencies: Vec::new(), // TODO: Extract dependencies from task relationships
        })
    }

    fn parse_role(&self, raw_role: RawRole) -> Result<ParsedRole, ParseError> {
        match raw_role {
            RawRole::String(name) => Ok(ParsedRole {
                name,
                src: None,
                version: None,
                vars: HashMap::new(),
                tags: Vec::new(),
            }),
            RawRole::Object(role_obj) => Ok(ParsedRole {
                name: role_obj.name,
                src: role_obj.src,
                version: role_obj.version,
                vars: role_obj.vars.unwrap_or_default(),
                tags: role_obj.tags.unwrap_or_default(),
            }),
        }
    }

    fn extract_module_and_args(
        &self,
        raw_task: &RawTask,
    ) -> Result<(String, HashMap<String, serde_json::Value>), ParseError> {
        // Look for known module keys
        let module_keys = [
            "shell",
            "command",
            "copy",
            "file",
            "template",
            "service",
            "package",
            "yum",
            "apt",
            "git",
            "debug",
            "set_fact",
            "include",
            "include_tasks",
            "import_tasks",
            "block",
            "rescue",
            "always",
            "meta",
            "setup",
            "gather_facts",
            "ping",
            "uri",
            "get_url",
            "unarchive",
            "lineinfile",
            "replace",
            "stat",
            "find",
            "user",
            "group",
            "cron",
            "systemd",
        ];

        for &key in &module_keys {
            if let Some(value) = raw_task.module_args.get(key) {
                let args = match value {
                    serde_json::Value::String(s) => {
                        let mut args = HashMap::new();
                        args.insert(
                            "_raw_params".to_string(),
                            serde_json::Value::String(s.clone()),
                        );
                        args
                    }
                    serde_json::Value::Object(obj) => {
                        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                    }
                    _ => {
                        let mut args = HashMap::new();
                        args.insert("_raw_params".to_string(), value.clone());
                        args
                    }
                };
                return Ok((key.to_string(), args));
            }
        }

        Err(ParseError::InvalidStructure {
            message: "No valid module found in task".to_string(),
        })
    }

    async fn render_task_args(
        &self,
        args: HashMap<String, serde_json::Value>,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let mut rendered_args = HashMap::new();

        for (key, value) in args {
            let normalized_value = self.normalize_yaml_value(value);
            let rendered_value = self.template_engine.render_value(&normalized_value, vars)?;
            rendered_args.insert(key, rendered_value);
        }

        Ok(rendered_args)
    }

    fn normalize_yaml_value(&self, value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                // Convert YAML boolean strings to actual booleans
                match s.to_lowercase().as_str() {
                    "yes" | "true" | "on" => serde_json::Value::Bool(true),
                    "no" | "false" | "off" => serde_json::Value::Bool(false),
                    _ => serde_json::Value::String(s),
                }
            }
            _ => value,
        }
    }
}

// Raw data structures for YAML parsing
#[derive(Debug, Deserialize)]
struct RawPlay {
    name: Option<String>,
    hosts: Option<RawHostPattern>,
    vars: Option<HashMap<String, serde_json::Value>>,
    tasks: Option<Vec<RawTask>>,
    handlers: Option<Vec<RawTask>>,
    roles: Option<Vec<RawRole>>,
    strategy: Option<ExecutionStrategy>,
    serial: Option<u32>,
    max_fail_percentage: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawHostPattern {
    Single(String),
    Multiple(Vec<String>),
    All,
}

#[derive(Debug, Deserialize)]
struct RawTask {
    id: Option<String>,
    name: Option<String>,
    vars: Option<HashMap<String, serde_json::Value>>,
    when: Option<String>,
    #[serde(rename = "loop")]
    loop_items: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_notify", default)]
    notify: Option<Vec<String>>,
    changed_when: Option<String>,
    failed_when: Option<String>,
    ignore_errors: Option<bool>,
    delegate_to: Option<String>,
    #[serde(flatten)]
    module_args: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawRole {
    String(String),
    Object(RawRoleObject),
}

#[derive(Debug, Deserialize)]
struct RawRoleObject {
    name: String,
    src: Option<String>,
    version: Option<String>,
    vars: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
}

// Custom deserializer for notify field that accepts both string and array
fn deserialize_notify<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};

    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(vec![s])),
        Some(serde_json::Value::Array(arr)) => {
            let strings: Result<Vec<String>, _> = arr
                .into_iter()
                .map(|v| match v {
                    serde_json::Value::String(s) => Ok(s),
                    _ => Err(Error::invalid_type(
                        Unexpected::Other("non-string in array"),
                        &"string",
                    )),
                })
                .collect();
            strings.map(Some)
        }
        Some(_) => Err(Error::invalid_type(
            Unexpected::Other("non-string/array"),
            &"string or array of strings",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::template::TemplateEngine;
    use std::collections::HashMap;

    fn create_test_parser() -> (TemplateEngine, HashMap<String, serde_json::Value>) {
        (TemplateEngine::default(), HashMap::new())
    }

    #[test]
    fn test_normalize_yaml_value_boolean_strings() {
        let (template_engine, extra_vars) = create_test_parser();
        let parser = PlaybookParser::new(&template_engine, &extra_vars);

        // Test "yes" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("yes".to_string())),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("YES".to_string())),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("Yes".to_string())),
            serde_json::Value::Bool(true)
        );

        // Test "true" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("true".to_string())),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("TRUE".to_string())),
            serde_json::Value::Bool(true)
        );

        // Test "on" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("on".to_string())),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("ON".to_string())),
            serde_json::Value::Bool(true)
        );

        // Test "no" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("no".to_string())),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("NO".to_string())),
            serde_json::Value::Bool(false)
        );

        // Test "false" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("false".to_string())),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("FALSE".to_string())),
            serde_json::Value::Bool(false)
        );

        // Test "off" variations
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("off".to_string())),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            parser.normalize_yaml_value(serde_json::Value::String("OFF".to_string())),
            serde_json::Value::Bool(false)
        );
    }

    #[test]
    fn test_normalize_yaml_value_non_boolean_strings() {
        let (template_engine, extra_vars) = create_test_parser();
        let parser = PlaybookParser::new(&template_engine, &extra_vars);

        // Test regular strings that should not be converted
        let test_cases = vec![
            "hello", "world", "maybe", "nope", "truthy", "falsy", "1", "0", "",
        ];

        for test_case in test_cases {
            assert_eq!(
                parser.normalize_yaml_value(serde_json::Value::String(test_case.to_string())),
                serde_json::Value::String(test_case.to_string()),
                "String '{}' should not be normalized",
                test_case
            );
        }
    }

    #[test]
    fn test_normalize_yaml_value_non_string_types() {
        let (template_engine, extra_vars) = create_test_parser();
        let parser = PlaybookParser::new(&template_engine, &extra_vars);

        // Test that non-string values are passed through unchanged
        let test_cases = vec![
            serde_json::Value::Bool(true),
            serde_json::Value::Bool(false),
            serde_json::Value::Number(serde_json::Number::from(42)),
            serde_json::Value::Number(serde_json::Number::from_f64(3.14).unwrap()),
            serde_json::Value::Null,
            serde_json::json!({"key": "value"}),
            serde_json::json!(["item1", "item2"]),
        ];

        for test_case in test_cases {
            let result = parser.normalize_yaml_value(test_case.clone());
            assert_eq!(result, test_case, "Non-string value should be unchanged");
        }
    }

    #[test]
    fn test_deserialize_notify_valid_cases() {
        // Test single string
        let yaml_single = r#"
name: Test task
notify: restart_service
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_single);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.notify, Some(vec!["restart_service".to_string()]));

        // Test array of strings
        let yaml_array = r#"
name: Test task
notify:
  - restart_service
  - reload_config
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_array);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(
            task.notify,
            Some(vec![
                "restart_service".to_string(),
                "reload_config".to_string()
            ])
        );

        // Test null/missing notify field
        let yaml_null = r#"
name: Test task
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_null);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.notify, None);
    }

    #[test]
    fn test_deserialize_notify_error_cases() {
        // Test invalid type (number)
        let yaml_number = r#"
name: Test task
notify: 123
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_number);
        assert!(task.is_err());

        // Test invalid type (boolean)
        let yaml_bool = r#"
name: Test task
notify: true
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_bool);
        assert!(task.is_err());

        // Test invalid type (object)
        let yaml_object = r#"
name: Test task
notify:
  handler: restart_service
  param: value
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_object);
        assert!(task.is_err());

        // Test array with non-string elements
        let yaml_mixed_array = r#"
name: Test task
notify:
  - restart_service
  - 123
  - reload_config
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_mixed_array);
        assert!(task.is_err());

        // Test array with nested objects
        let yaml_nested_array = r#"
name: Test task
notify:
  - restart_service
  - { handler: reload_config }
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_nested_array);
        assert!(task.is_err());
    }
}
