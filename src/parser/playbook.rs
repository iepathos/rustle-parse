use crate::parser::error::ParseError;
use crate::parser::include::{ImportSpec, IncludeContext, IncludeHandler, IncludeSpec};
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

    /// Parse playbook with include/import support
    pub async fn parse_with_includes(&self, path: &Path) -> Result<ParsedPlaybook, ParseError> {
        let base_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let mut include_handler = IncludeHandler::new(base_path, self.template_engine.clone());

        self.parse_playbook_recursive(path, &mut include_handler)
            .await
    }

    /// Parse playbook without include support (original method)
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

    /// Parse playbook recursively with include support
    async fn parse_playbook_recursive(
        &self,
        path: &Path,
        include_handler: &mut IncludeHandler,
    ) -> Result<ParsedPlaybook, ParseError> {
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

        // Parse YAML - Ansible playbooks can be arrays of plays or include directives
        let yaml_content: serde_yaml::Value = serde_yaml::from_str(&content)?;

        // Process each item in the playbook
        let items = if let serde_yaml::Value::Sequence(seq) = yaml_content {
            seq
        } else {
            vec![yaml_content]
        };

        // Transform to parsed format
        let mut parsed_plays = Vec::new();
        let mut playbook_vars = HashMap::new();
        let mut facts_required = false;
        let vault_ids = Vec::new(); // TODO: Extract vault IDs from content

        // Merge extra vars
        playbook_vars.extend(self.extra_vars.clone());

        // Create include context
        let include_context = IncludeContext {
            variables: playbook_vars.clone(),
            current_file: path.to_path_buf(),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        // Process each item in the playbook
        for item in items {
            // Check if this is a playbook include directive
            if let Some(map) = item.as_mapping() {
                if map.contains_key("include_playbook") || map.contains_key("import_playbook") {
                    // This is a playbook include directive
                    let included_plays = self
                        .handle_playbook_include_directive(item, include_handler, &include_context)
                        .await?;
                    parsed_plays.extend(included_plays);
                    continue;
                }
            }

            // Otherwise, parse as a regular play
            let raw_play: RawPlay = serde_yaml::from_value(item)?;
            let parsed_play = self
                .parse_play_with_includes(
                    raw_play,
                    &playbook_vars,
                    include_handler,
                    &include_context,
                )
                .await?;

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
            None => HostPattern::Single("localhost".to_string()),
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

        // Render play name if it contains templates
        let play_name = match raw_play.name {
            Some(name) if name.contains("{{") && name.contains("}}") => {
                self.template_engine.render_string(&name, &play_vars)?
            }
            Some(name) => name,
            None => "Unnamed play".to_string(),
        };

        Ok(ParsedPlay {
            name: play_name,
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

    async fn parse_play_with_includes(
        &self,
        raw_play: RawPlay,
        global_vars: &HashMap<String, serde_json::Value>,
        include_handler: &mut IncludeHandler,
        include_context: &IncludeContext,
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
            None => HostPattern::Single("localhost".to_string()),
        };

        // Parse tasks with include support
        let mut tasks = Vec::new();
        if let Some(raw_tasks) = raw_play.tasks {
            // Create a new include context with play vars
            let mut play_include_context = include_context.clone();
            play_include_context.variables = play_vars.clone();

            for (index, raw_task) in raw_tasks.into_iter().enumerate() {
                // Check if this is an include directive
                if self.is_include_task(&raw_task) {
                    let included_tasks = self
                        .process_task_include(&raw_task, include_handler, &play_include_context)
                        .await?;
                    tasks.extend(included_tasks);
                } else {
                    let task = self.parse_task(raw_task, &play_vars, index).await?;
                    tasks.push(task);
                }
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

        // Render play name if it contains templates
        let play_name = match raw_play.name {
            Some(name) if name.contains("{{") && name.contains("}}") => {
                self.template_engine.render_string(&name, &play_vars)?
            }
            Some(name) => name,
            None => "Unnamed play".to_string(),
        };

        Ok(ParsedPlay {
            name: play_name,
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

    /// Check if a raw task is an include directive
    fn is_include_task(&self, raw_task: &RawTask) -> bool {
        let include_keys = [
            "include_tasks",
            "import_tasks",
            "include_playbook",
            "import_playbook",
            "include_vars",
            "include_role",
            "import_role",
        ];

        include_keys
            .iter()
            .any(|key| raw_task.module_args.contains_key(*key))
    }

    /// Handle playbook-level include/import directives
    async fn handle_playbook_include_directive(
        &self,
        item: serde_yaml::Value,
        include_handler: &mut IncludeHandler,
        include_context: &IncludeContext,
    ) -> Result<Vec<ParsedPlay>, ParseError> {
        let map = item
            .as_mapping()
            .ok_or_else(|| ParseError::InvalidStructure {
                message: "Expected mapping for playbook include".to_string(),
            })?;

        // Determine if it's include or import
        let is_include = map.contains_key("include_playbook");
        let directive_name = if is_include {
            "include_playbook"
        } else {
            "import_playbook"
        };

        // Get the playbook path
        let playbook_path = map
            .get(directive_name)
            .and_then(|v| v.as_str())
            .ok_or_else(|| ParseError::InvalidStructure {
                message: format!("{directive_name} requires a path"),
            })?;

        // Extract optional parameters
        let vars = map.get("vars").and_then(|v| {
            serde_yaml::from_value::<HashMap<String, serde_json::Value>>(v.clone()).ok()
        });

        // For when conditions, get the raw YAML value as a string
        let when_condition = map.get("when").map(|v| {
            // Convert YAML value to string representation
            match v {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                serde_yaml::Value::Number(n) => n.to_string(),
                _ => format!("{v:?}"),
            }
        });

        let tags = map.get("tags").and_then(|v| {
            if let Some(arr) = v.as_sequence() {
                Some(
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect(),
                )
            } else if let Some(s) = v.as_str() {
                Some(vec![s.to_string()])
            } else {
                None
            }
        });

        if is_include {
            // Create IncludeSpec
            let include_spec = IncludeSpec {
                file: playbook_path.to_string(),
                vars,
                when_condition,
                tags,
                apply: None,
                delegate_to: None,
                delegate_facts: None,
                run_once: None,
            };

            include_handler
                .include_playbook(&include_spec, include_context)
                .await
        } else {
            // Create ImportSpec
            let import_spec = ImportSpec {
                file: playbook_path.to_string(),
                vars,
                when_condition,
                tags,
            };

            include_handler
                .import_playbook(&import_spec, include_context)
                .await
        }
    }

    /// Check if a raw play is a playbook include directive
    #[allow(dead_code)]
    fn is_include_playbook(&self, raw_task: &RawTask) -> bool {
        raw_task.module_args.contains_key("include_playbook")
            || raw_task.module_args.contains_key("import_playbook")
    }

    /// Process task-level include directives
    async fn process_task_include(
        &self,
        raw_task: &RawTask,
        include_handler: &mut IncludeHandler,
        include_context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        // Convert raw task to include specification
        if let Some(include_tasks_value) = raw_task.module_args.get("include_tasks") {
            let include_spec = self.parse_include_tasks_spec(include_tasks_value, raw_task)?;
            include_handler
                .include_tasks(&include_spec, include_context)
                .await
        } else if let Some(import_tasks_value) = raw_task.module_args.get("import_tasks") {
            let import_spec = self.parse_import_tasks_spec(import_tasks_value, raw_task)?;
            include_handler
                .import_tasks(&import_spec, include_context)
                .await
        } else {
            // For now, only support include_tasks and import_tasks at task level
            // include_playbook and import_playbook should be handled at play level
            // Other include types would be implemented here
            Ok(Vec::new())
        }
    }

    /// Process playbook-level include directives
    #[allow(dead_code)]
    async fn process_playbook_include(
        &self,
        raw_task: &RawTask,
        include_handler: &mut IncludeHandler,
        include_context: &IncludeContext,
    ) -> Result<Vec<ParsedPlay>, ParseError> {
        // Convert raw task to include specification
        if let Some(include_playbook_value) = raw_task.module_args.get("include_playbook") {
            let include_spec =
                self.parse_include_playbook_spec(include_playbook_value, raw_task)?;
            include_handler
                .include_playbook(&include_spec, include_context)
                .await
        } else if let Some(import_playbook_value) = raw_task.module_args.get("import_playbook") {
            let import_spec = self.parse_import_playbook_spec(import_playbook_value, raw_task)?;
            include_handler
                .import_playbook(&import_spec, include_context)
                .await
        } else {
            Ok(Vec::new())
        }
    }

    /// Parse include_tasks specification from raw task
    fn parse_include_tasks_spec(
        &self,
        include_value: &serde_json::Value,
        raw_task: &RawTask,
    ) -> Result<crate::parser::include::IncludeSpec, ParseError> {
        let file = match include_value {
            serde_json::Value::String(file_path) => file_path.clone(),
            _ => {
                return Err(ParseError::InvalidIncludeDirective {
                    message: "include_tasks must specify a file path".to_string(),
                });
            }
        };

        Ok(crate::parser::include::IncludeSpec {
            file,
            vars: raw_task.vars.clone(),
            when_condition: raw_task.when.clone(),
            tags: raw_task.tags.clone(),
            apply: None, // TODO: Parse apply block from raw task
            delegate_to: raw_task.delegate_to.clone(),
            delegate_facts: None, // TODO: Extract from raw task if present
            run_once: None,       // TODO: Extract from raw task if present
        })
    }

    /// Parse import_tasks specification from raw task
    fn parse_import_tasks_spec(
        &self,
        import_value: &serde_json::Value,
        raw_task: &RawTask,
    ) -> Result<crate::parser::include::ImportSpec, ParseError> {
        let file = match import_value {
            serde_json::Value::String(file_path) => file_path.clone(),
            _ => {
                return Err(ParseError::InvalidIncludeDirective {
                    message: "import_tasks must specify a file path".to_string(),
                });
            }
        };

        Ok(crate::parser::include::ImportSpec {
            file,
            vars: raw_task.vars.clone(),
            when_condition: raw_task.when.clone(),
            tags: raw_task.tags.clone(),
        })
    }

    /// Parse include_playbook specification from raw task
    #[allow(dead_code)]
    fn parse_include_playbook_spec(
        &self,
        include_value: &serde_json::Value,
        raw_task: &RawTask,
    ) -> Result<crate::parser::include::IncludeSpec, ParseError> {
        let file = match include_value {
            serde_json::Value::String(file_path) => file_path.clone(),
            _ => {
                return Err(ParseError::InvalidIncludeDirective {
                    message: "include_playbook must specify a file path".to_string(),
                });
            }
        };

        Ok(crate::parser::include::IncludeSpec {
            file,
            vars: raw_task.vars.clone(),
            when_condition: raw_task.when.clone(),
            tags: raw_task.tags.clone(),
            apply: None, // TODO: Parse apply block from raw task
            delegate_to: raw_task.delegate_to.clone(),
            delegate_facts: None, // TODO: Extract from raw task if present
            run_once: None,       // TODO: Extract from raw task if present
        })
    }

    /// Parse import_playbook specification from raw task
    #[allow(dead_code)]
    fn parse_import_playbook_spec(
        &self,
        import_value: &serde_json::Value,
        raw_task: &RawTask,
    ) -> Result<crate::parser::include::ImportSpec, ParseError> {
        let file = match import_value {
            serde_json::Value::String(file_path) => file_path.clone(),
            _ => {
                return Err(ParseError::InvalidIncludeDirective {
                    message: "import_playbook must specify a file path".to_string(),
                });
            }
        };

        Ok(crate::parser::include::ImportSpec {
            file,
            vars: raw_task.vars.clone(),
            when_condition: raw_task.when.clone(),
            tags: raw_task.tags.clone(),
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

        // Process boolean-or-string fields with template resolution
        let changed_when = self.convert_boolean_or_string_field(raw_task.changed_when, vars)?;
        let failed_when = self.convert_boolean_or_string_field(raw_task.failed_when, vars)?;

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
            changed_when,
            failed_when,
            ignore_errors: raw_task.ignore_errors.unwrap_or(false),
            delegate_to: raw_task.delegate_to,
            dependencies: Vec::new(), // TODO: Extract dependencies from task relationships
        })
    }

    /// Convert boolean-or-string field with template resolution
    fn convert_boolean_or_string_field(
        &self,
        field: Option<BooleanOrString>,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<Option<BooleanOrString>, ParseError> {
        match field {
            None => Ok(None),
            Some(BooleanOrString::Boolean(b)) => Ok(Some(BooleanOrString::Boolean(b))),
            Some(BooleanOrString::String(s)) => {
                let resolved = self.template_engine.render_string(&s, vars)?;
                // Try to parse resolved template as boolean
                match resolved.to_lowercase().as_str() {
                    "true" | "yes" | "on" => Ok(Some(BooleanOrString::Boolean(true))),
                    "false" | "no" | "off" => Ok(Some(BooleanOrString::Boolean(false))),
                    _ => Ok(Some(BooleanOrString::String(resolved))),
                }
            }
        }
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
            "assert",
            "postgresql_db",
            "wait_for",
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

        // Provide more context about the failed task
        let available_modules: Vec<String> = raw_task.module_args.keys().cloned().collect();
        let task_name = raw_task.name.as_deref().unwrap_or("unnamed");
        Err(ParseError::InvalidStructure {
            message: format!("No valid module found in task '{}'. Available keys: {:?}. Known modules: include_tasks={}, import_tasks={}", 
                task_name, available_modules,
                module_keys.contains(&"include_tasks"), 
                module_keys.contains(&"import_tasks")),
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
    #[serde(deserialize_with = "deserialize_boolean_or_string", default)]
    changed_when: Option<BooleanOrString>,
    #[serde(deserialize_with = "deserialize_boolean_or_string", default)]
    failed_when: Option<BooleanOrString>,
    #[serde(deserialize_with = "deserialize_yaml_bool", default)]
    ignore_errors: Option<bool>,
    delegate_to: Option<String>,
    #[serde(rename = "become", deserialize_with = "deserialize_yaml_bool", default)]
    #[allow(dead_code)]
    r#become: Option<bool>,
    #[allow(dead_code)]
    become_user: Option<String>,
    #[allow(dead_code)]
    become_method: Option<String>,
    #[allow(dead_code)]
    register: Option<String>,
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

// Custom deserializer for boolean fields that handles YAML boolean strings
fn deserialize_yaml_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_yaml::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_yaml::Value::Bool(b)) => Ok(Some(b)),
        Some(serde_yaml::Value::String(s)) => match s.to_lowercase().as_str() {
            "yes" | "true" | "on" => Ok(Some(true)),
            "no" | "false" | "off" => Ok(Some(false)),
            _ => Err(Error::custom(format!("Invalid boolean string: {s}"))),
        },
        Some(_) => Err(Error::custom("Expected boolean or boolean string")),
    }
}

// Custom deserializer for boolean-or-string fields like changed_when and failed_when
pub fn deserialize_boolean_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<BooleanOrString>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_yaml::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_yaml::Value::Bool(b)) => Ok(Some(BooleanOrString::Boolean(b))),
        Some(serde_yaml::Value::String(s)) => {
            // Try to parse as boolean first for string representations
            match s.to_lowercase().as_str() {
                "true" | "yes" | "on" => Ok(Some(BooleanOrString::Boolean(true))),
                "false" | "no" | "off" => Ok(Some(BooleanOrString::Boolean(false))),
                _ => Ok(Some(BooleanOrString::String(s))),
            }
        },
        Some(serde_yaml::Value::Number(_)) => Err(D::Error::custom(
            "numeric values not supported for boolean/string fields, use 'true'/'false' or a string expression"
        )),
        Some(serde_yaml::Value::Sequence(_)) => Err(D::Error::custom(
            "arrays not supported for boolean/string fields"
        )),
        Some(other) => Err(D::Error::invalid_type(
            serde::de::Unexpected::Other(&format!("{other:?}")),
            &"boolean literal (true/false) or string expression"
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
                "String '{test_case}' should not be normalized"
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
            serde_json::Value::Number(serde_json::Number::from_f64(std::f64::consts::PI).unwrap()),
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

    #[test]
    fn test_boolean_literal_parsing() {
        let yaml = r#"
name: Test task
changed_when: false
failed_when: true
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.changed_when, Some(BooleanOrString::Boolean(false)));
        assert_eq!(task.failed_when, Some(BooleanOrString::Boolean(true)));
    }

    #[test]
    fn test_string_boolean_parsing() {
        let yaml = r#"
name: Test task
changed_when: "false"
failed_when: "yes"
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.changed_when, Some(BooleanOrString::Boolean(false)));
        assert_eq!(task.failed_when, Some(BooleanOrString::Boolean(true)));
    }

    #[test]
    fn test_conditional_expression_parsing() {
        let yaml = r#"
name: Test task
changed_when: "result.rc != 0"
failed_when: "ansible_hostname == 'test'"
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert!(matches!(
            task.changed_when,
            Some(BooleanOrString::String(_))
        ));
        assert!(matches!(task.failed_when, Some(BooleanOrString::String(_))));

        if let Some(BooleanOrString::String(s)) = &task.changed_when {
            assert_eq!(s, "result.rc != 0");
        }
        if let Some(BooleanOrString::String(s)) = &task.failed_when {
            assert_eq!(s, "ansible_hostname == 'test'");
        }
    }

    #[test]
    fn test_boolean_string_variations() {
        // Test all YAML boolean variations
        let test_cases = vec![
            ("true", true),
            ("True", true),
            ("TRUE", true),
            ("yes", true),
            ("Yes", true),
            ("YES", true),
            ("on", true),
            ("On", true),
            ("ON", true),
            ("false", false),
            ("False", false),
            ("FALSE", false),
            ("no", false),
            ("No", false),
            ("NO", false),
            ("off", false),
            ("Off", false),
            ("OFF", false),
        ];

        for (yaml_value, expected_bool) in test_cases {
            let yaml = format!(
                r#"
name: Test task
changed_when: "{}"
"#,
                yaml_value
            );
            let task: Result<RawTask, _> = serde_yaml::from_str(&yaml);
            assert!(
                task.is_ok(),
                "Failed to parse YAML with value: {}",
                yaml_value
            );
            let task = task.unwrap();
            assert_eq!(
                task.changed_when,
                Some(BooleanOrString::Boolean(expected_bool)),
                "Value '{}' should parse as boolean {}",
                yaml_value,
                expected_bool
            );
        }
    }

    #[test]
    fn test_invalid_boolean_or_string_types() {
        // Test invalid numeric value
        let yaml_number = r#"
name: Test task
changed_when: 123
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_number);
        assert!(task.is_err());

        // Test invalid array value
        let yaml_array = r#"
name: Test task
changed_when: 
  - item1
  - item2
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_array);
        assert!(task.is_err());

        // Test invalid object value
        let yaml_object = r#"
name: Test task
changed_when:
  condition: "result.rc != 0"
  operator: "!="
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml_object);
        assert!(task.is_err());
    }

    #[test]
    fn test_mixed_boolean_and_string_usage() {
        let yaml = r#"
name: Test task
changed_when: false
failed_when: "result.failed"
"#;
        let task: Result<RawTask, _> = serde_yaml::from_str(yaml);
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.changed_when, Some(BooleanOrString::Boolean(false)));
        assert!(matches!(task.failed_when, Some(BooleanOrString::String(_))));
    }
}
