use crate::parser::error::ParseError;
use crate::parser::include::{
    cache::IncludeCache, dependency::IncludeStack, resolver::PathResolver, ImportSpec,
    IncludeConfig, IncludeContext, IncludeSpec,
};
use crate::parser::template::TemplateEngine;
use crate::types::parsed::*;
use serde_yaml;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Main handler for include/import operations
pub struct IncludeHandler {
    path_resolver: PathResolver,
    template_engine: TemplateEngine,
    cache: IncludeCache,
    include_stack: IncludeStack,
    config: IncludeConfig,
}

impl IncludeHandler {
    pub fn new(base_path: PathBuf, template_engine: TemplateEngine) -> Self {
        let config = IncludeConfig::default();
        let path_resolver = PathResolver::new(base_path)
            .with_absolute_paths(config.allow_absolute_paths)
            .with_strict_permissions(config.strict_file_permissions);
        let cache = IncludeCache::new(1000, config.cache_ttl);
        let include_stack = IncludeStack::new(config.max_include_depth);

        Self {
            path_resolver,
            template_engine,
            cache,
            include_stack,
            config,
        }
    }

    pub fn with_config(mut self, config: IncludeConfig) -> Self {
        self.path_resolver = self
            .path_resolver
            .with_absolute_paths(config.allow_absolute_paths)
            .with_strict_permissions(config.strict_file_permissions);
        self.cache = IncludeCache::new(1000, config.cache_ttl);
        self.include_stack = IncludeStack::new(config.max_include_depth);
        self.config = config;
        self
    }

    /// Process include_tasks directive
    pub fn include_tasks<'a>(
        &'a mut self,
        include_spec: &'a IncludeSpec,
        context: &'a IncludeContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ParsedTask>, ParseError>> + 'a>,
    > {
        Box::pin(async move {
            // Check if include should be processed based on when condition
            if !self
                .should_process_include(include_spec.when_condition.as_ref(), context)
                .await?
            {
                return Ok(Vec::new());
            }

            // Resolve the file path
            let resolved_path = self
                .path_resolver
                .resolve_path(&include_spec.file, &context.current_file)?;

            // Manage include stack
            self.include_stack.push(resolved_path.clone())?;

            // Load and parse the included file
            let content = self.load_file_cached(&resolved_path).await?;

            // Create context for included tasks
            let mut include_context = context.clone();
            include_context.current_file = resolved_path.clone();
            include_context.include_depth += 1;

            // Merge include variables
            if let Some(include_vars) = &include_spec.vars {
                for (key, value) in include_vars {
                    let rendered_value = self
                        .template_engine
                        .render_value(value, &include_context.variables)?;
                    include_context
                        .variables
                        .insert(key.clone(), rendered_value);
                }
            }

            // Parse tasks from included file
            let raw_tasks: Vec<serde_yaml::Value> =
                serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

            let mut parsed_tasks = Vec::new();
            for (index, raw_task_value) in raw_tasks.into_iter().enumerate() {
                // TODO: Handle nested includes (simplified for now)
                // Check if this is a nested include directive
                // if let Some(nested_directive) = self.detect_include_directive(&raw_task_value)? {
                //     let nested_tasks = self
                //         .process_include_directive(&nested_directive, &include_context)
                //         .await?;
                //     parsed_tasks.extend(nested_tasks);
                // } else {
                // Parse as regular task
                let raw_task: RawTask =
                    serde_yaml::from_value(raw_task_value).map_err(ParseError::Yaml)?;
                let task = self
                    .parse_task_with_context(raw_task, &include_context, index)
                    .await?;

                // Apply include-level properties
                let enhanced_task = self.apply_include_properties(task, include_spec)?;
                parsed_tasks.push(enhanced_task);
                // }
            }

            // Remove from include stack
            self.include_stack.pop();

            Ok(parsed_tasks)
        })
    }

    /// Process import_tasks directive
    pub async fn import_tasks(
        &mut self,
        import_spec: &ImportSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        // Check if import should be processed
        if !self
            .should_process_include(import_spec.when_condition.as_ref(), context)
            .await?
        {
            return Ok(Vec::new());
        }

        // Import is processed at parse time, similar to include_tasks
        // but with different variable scoping rules
        let resolved_path = self
            .path_resolver
            .resolve_path(&import_spec.file, &context.current_file)?;

        self.include_stack.push(resolved_path.clone())?;

        let content = self.load_file_cached(&resolved_path).await?;

        // For imports, variables are applied at parse time
        let mut import_context = context.clone();
        import_context.current_file = resolved_path.clone();
        import_context.include_depth += 1;

        // Apply import variables to context
        if let Some(import_vars) = &import_spec.vars {
            for (key, value) in import_vars {
                let rendered_value = self
                    .template_engine
                    .render_value(value, &import_context.variables)?;
                import_context.variables.insert(key.clone(), rendered_value);
            }
        }

        // Parse and process tasks immediately
        let raw_tasks: Vec<serde_yaml::Value> =
            serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

        let mut parsed_tasks = Vec::new();
        for (index, raw_task_value) in raw_tasks.into_iter().enumerate() {
            // TODO: Handle nested includes
            // if let Some(nested_directive) = self.detect_include_directive(&raw_task_value)? {
            //     let nested_tasks = self
            //         .process_include_directive(&nested_directive, &import_context)
            //         .await?;
            //     parsed_tasks.extend(nested_tasks);
            // } else {
            let raw_task: RawTask =
                serde_yaml::from_value(raw_task_value).map_err(ParseError::Yaml)?;
            let task = self
                .parse_task_with_context(raw_task, &import_context, index)
                .await?;
            parsed_tasks.push(task);
            // }
        }

        self.include_stack.pop();
        Ok(parsed_tasks)
    }

    /// Process include_playbook directive
    pub async fn include_playbook(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedPlay>, ParseError> {
        // Check if include should be processed based on when condition
        if !self
            .should_process_include(include_spec.when_condition.as_ref(), context)
            .await?
        {
            return Ok(Vec::new());
        }

        // Resolve the playbook path
        let resolved_path = self
            .path_resolver
            .resolve_path(&include_spec.file, &context.current_file)?;

        // Manage include stack
        self.include_stack.push(resolved_path.clone())?;

        // Load and parse the included playbook
        let content = self.load_file_cached(&resolved_path).await?;

        // Create context for included playbook
        let mut include_context = context.clone();
        include_context.current_file = resolved_path.clone();
        include_context.include_depth += 1;

        // Merge include variables
        if let Some(include_vars) = &include_spec.vars {
            for (key, value) in include_vars {
                let rendered_value = self
                    .template_engine
                    .render_value(value, &include_context.variables)?;
                include_context
                    .variables
                    .insert(key.clone(), rendered_value);
            }
        }

        // Parse playbook content as array of plays
        let raw_plays: Vec<serde_yaml::Value> =
            serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

        let mut parsed_plays = Vec::new();
        for raw_play_value in raw_plays.into_iter() {
            // Parse the play with the include context
            let raw_play: RawPlay =
                serde_yaml::from_value(raw_play_value).map_err(ParseError::Yaml)?;
            let play = self
                .parse_play_with_context(raw_play, &include_context)
                .await?;

            // Apply include-level properties to the play
            let enhanced_play = self.apply_include_properties_to_play(play, include_spec)?;
            parsed_plays.push(enhanced_play);
        }

        // Remove from include stack
        self.include_stack.pop();

        Ok(parsed_plays)
    }

    /// Process import_playbook directive
    pub async fn import_playbook(
        &mut self,
        import_spec: &ImportSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedPlay>, ParseError> {
        // Check if import should be processed
        if !self
            .should_process_include(import_spec.when_condition.as_ref(), context)
            .await?
        {
            return Ok(Vec::new());
        }

        // Import is processed at parse time, similar to include_playbook
        // but with different variable scoping rules
        let resolved_path = self
            .path_resolver
            .resolve_path(&import_spec.file, &context.current_file)?;

        self.include_stack.push(resolved_path.clone())?;

        let content = self.load_file_cached(&resolved_path).await?;

        // For imports, variables are applied at parse time
        let mut import_context = context.clone();
        import_context.current_file = resolved_path.clone();
        import_context.include_depth += 1;

        // Apply import variables to context
        if let Some(import_vars) = &import_spec.vars {
            for (key, value) in import_vars {
                let rendered_value = self
                    .template_engine
                    .render_value(value, &import_context.variables)?;
                import_context.variables.insert(key.clone(), rendered_value);
            }
        }

        // Parse and process plays immediately
        let raw_plays: Vec<serde_yaml::Value> =
            serde_yaml::from_str(&content).map_err(ParseError::Yaml)?;

        let mut parsed_plays = Vec::new();
        for raw_play_value in raw_plays.into_iter() {
            let raw_play: RawPlay =
                serde_yaml::from_value(raw_play_value).map_err(ParseError::Yaml)?;
            let play = self
                .parse_play_with_context(raw_play, &import_context)
                .await?;
            parsed_plays.push(play);
        }

        self.include_stack.pop();
        Ok(parsed_plays)
    }

    /// Check if include should be processed based on when condition
    async fn should_process_include(
        &self,
        when_condition: Option<&String>,
        context: &IncludeContext,
    ) -> Result<bool, ParseError> {
        if let Some(when_condition) = when_condition {
            // Evaluate the when condition using template engine
            // If we get an undefined value error, treat it as false (following Ansible behavior)
            let result = match self
                .template_engine
                .render_string(&format!("{{{{ {when_condition} }}}}"), &context.variables)
            {
                Ok(rendered) => rendered,
                Err(ParseError::Template { message, .. }) if message.contains("undefined") => {
                    // For undefined variables in when conditions, evaluate to false
                    // This matches Ansible's behavior
                    return Ok(false);
                }
                Err(e) => return Err(e),
            };

            // Parse result as boolean
            match result.trim().to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(true),
                "false" | "no" | "0" => Ok(false),
                "" => Ok(false),
                _ => {
                    // Try to parse as boolean-ish value
                    Ok(!result.trim().is_empty())
                }
            }
        } else {
            Ok(true)
        }
    }

    // TODO: Re-implement these methods without recursion issues
    // /// Process any include directive
    // async fn process_include_directive(
    //     &mut self,
    //     directive: &IncludeDirective,
    //     context: &IncludeContext,
    // ) -> Result<Vec<ParsedTask>, ParseError> {
    //     match directive {
    //         IncludeDirective::IncludeTasks(spec) => self.include_tasks(spec, context).await,
    //         IncludeDirective::ImportTasks(spec) => self.import_tasks(spec, context).await,
    //         // TODO: Implement other directive types
    //         _ => Err(ParseError::UnsupportedFeature {
    //             feature: format!("Include directive: {:?}", directive),
    //         }),
    //     }
    // }

    // /// Detect if a YAML value represents an include directive
    // fn detect_include_directive(
    //     &self,
    //     value: &serde_yaml::Value,
    // ) -> Result<Option<IncludeDirective>, ParseError> {
    //     if let serde_yaml::Value::Mapping(map) = value {
    //         // Check for include/import keys
    //         let include_keys = [
    //             "include_tasks",
    //             "import_tasks",
    //             "include_playbook",
    //             "import_playbook",
    //             "include_vars",
    //             "include_role",
    //             "import_role",
    //         ];

    //         for key in &include_keys {
    //             if map.contains_key(&serde_yaml::Value::String(key.to_string())) {
    //                 // This is an include directive, parse it
    //                 let directive: IncludeDirective = serde_yaml::from_value(value.clone())
    //                     .map_err(|e| ParseError::InvalidIncludeDirective {
    //                         message: format!("Failed to parse {} directive: {}", key, e),
    //                     })?;
    //                 return Ok(Some(directive));
    //             }
    //         }
    //     }

    //     Ok(None)
    // }

    /// Parse a task with the given context
    async fn parse_task_with_context(
        &self,
        raw_task: RawTask,
        _context: &IncludeContext,
        index: usize,
    ) -> Result<ParsedTask, ParseError> {
        // This is a simplified task parser - in a full implementation,
        // this would delegate to the main playbook parser
        let id = raw_task
            .id
            .clone()
            .unwrap_or_else(|| format!("task_{index}"));

        // Don't render task name - preserve templates for runtime
        let name = raw_task
            .name
            .clone()
            .unwrap_or_else(|| "Unnamed task".to_string());

        // Extract module and args (simplified)
        let (module, args) = self.extract_module_and_args(&raw_task)?;

        // Don't render templates in args - preserve them for runtime evaluation
        let normalized_args = args;

        Ok(ParsedTask {
            id,
            name,
            module,
            args: normalized_args,
            vars: raw_task.vars.unwrap_or_default(),
            when: raw_task.when,
            loop_items: raw_task.loop_items,
            tags: raw_task.tags.unwrap_or_default(),
            notify: raw_task.notify.unwrap_or_default(),
            changed_when: raw_task.changed_when,
            failed_when: raw_task.failed_when,
            ignore_errors: raw_task.ignore_errors.unwrap_or(false),
            delegate_to: raw_task.delegate_to,
            dependencies: Vec::new(),
        })
    }

    /// Apply include-level properties to a task
    fn apply_include_properties(
        &self,
        mut task: ParsedTask,
        include_spec: &IncludeSpec,
    ) -> Result<ParsedTask, ParseError> {
        // Apply include-level tags
        if let Some(include_tags) = &include_spec.tags {
            task.tags.extend(include_tags.clone());
        }

        // Apply include-level when condition
        if let Some(include_when) = &include_spec.when_condition {
            if let Some(existing_when) = &task.when {
                // Combine conditions with AND
                task.when = Some(format!("({}) and ({})", existing_when, include_when));
            } else {
                task.when = Some(include_when.clone());
            }
        }

        // Apply delegate_to if specified
        if let Some(delegate_to) = &include_spec.delegate_to {
            task.delegate_to = Some(delegate_to.clone());
        }

        // Apply apply block properties
        if let Some(apply_spec) = &include_spec.apply {
            if let Some(apply_tags) = &apply_spec.tags {
                task.tags.extend(apply_tags.clone());
            }

            if let Some(apply_when) = &apply_spec.when_condition {
                if let Some(existing_when) = &task.when {
                    task.when = Some(format!("({}) and ({})", existing_when, apply_when));
                } else {
                    task.when = Some(apply_when.clone());
                }
            }
        }

        Ok(task)
    }

    /// Load file with caching
    async fn load_file_cached(&mut self, path: &Path) -> Result<String, ParseError> {
        let file_metadata =
            fs::metadata(path)
                .await
                .map_err(|_| ParseError::IncludeFileNotFound {
                    file: path.to_string_lossy().to_string(),
                })?;

        let file_modified = file_metadata
            .modified()
            .unwrap_or_else(|_| std::time::SystemTime::now());

        // Check cache first if enabled
        if self.config.enable_include_cache {
            if let Some(cached_content) = self.cache.get(&path.to_path_buf(), file_modified) {
                return Ok(cached_content.to_string());
            }
        }

        // Load file and update cache
        let content =
            fs::read_to_string(path)
                .await
                .map_err(|_| ParseError::IncludeFileNotFound {
                    file: path.to_string_lossy().to_string(),
                })?;

        if self.config.enable_include_cache {
            self.cache
                .insert(path.to_path_buf(), content.clone(), file_modified);
        }

        Ok(content)
    }

    /// Extract module and args from raw task (simplified version)
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
            "include_role",
            "import_role",
            "include_vars",
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

    /// Render task arguments through template engine
    async fn render_task_args(
        &self,
        args: HashMap<String, serde_json::Value>,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let mut rendered_args = HashMap::new();

        for (key, value) in args {
            let rendered_value = self.template_engine.render_value(&value, vars)?;
            rendered_args.insert(key, rendered_value);
        }

        Ok(rendered_args)
    }

    /// Clear include cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get include statistics
    pub fn get_stats(&self) -> IncludeStats {
        IncludeStats {
            current_depth: self.include_stack.depth(),
            max_depth: self.config.max_include_depth,
            cache_stats: self.cache.stats(),
        }
    }

    /// Parse a play with the given context (simplified version for include handlers)
    async fn parse_play_with_context(
        &self,
        raw_play: RawPlay,
        context: &IncludeContext,
    ) -> Result<ParsedPlay, ParseError> {
        // Create a simplified play parser - in a full implementation,
        // this would delegate to the main playbook parser
        let mut play_vars = context.variables.clone();

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
            for (index, raw_task_value) in raw_tasks.into_iter().enumerate() {
                let raw_task: RawTask =
                    serde_yaml::from_value(raw_task_value).map_err(ParseError::Yaml)?;
                let task = self
                    .parse_task_with_context(raw_task, context, index)
                    .await?;
                tasks.push(task);
            }
        }

        // Parse handlers
        let mut handlers = Vec::new();
        if let Some(raw_handlers) = raw_play.handlers {
            for (index, raw_handler_value) in raw_handlers.into_iter().enumerate() {
                let raw_handler: RawTask =
                    serde_yaml::from_value(raw_handler_value).map_err(ParseError::Yaml)?;
                let handler = self
                    .parse_task_with_context(raw_handler, context, index)
                    .await?;
                handlers.push(handler);
            }
        }

        // Parse roles (simplified)
        let mut roles = Vec::new();
        if let Some(raw_roles) = raw_play.roles {
            for raw_role in raw_roles {
                let role = self.parse_role_simple(raw_role)?;
                roles.push(role);
            }
        }

        // Don't render play name - preserve templates for runtime
        let play_name = raw_play.name.unwrap_or_else(|| "Unnamed play".to_string());

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

    /// Apply include-level properties to a play
    fn apply_include_properties_to_play(
        &self,
        mut play: ParsedPlay,
        include_spec: &IncludeSpec,
    ) -> Result<ParsedPlay, ParseError> {
        // Apply include-level variables to play vars
        if let Some(include_vars) = &include_spec.vars {
            for (key, value) in include_vars {
                play.vars.insert(key.clone(), value.clone());
            }
        }

        // Apply tags to all tasks in the play
        if let Some(include_tags) = &include_spec.tags {
            for task in &mut play.tasks {
                task.tags.extend(include_tags.clone());
            }
            for handler in &mut play.handlers {
                handler.tags.extend(include_tags.clone());
            }
        }

        // Apply when condition to all tasks
        if let Some(include_when) = &include_spec.when_condition {
            for task in &mut play.tasks {
                if let Some(existing_when) = &task.when {
                    task.when = Some(format!("({}) and ({})", existing_when, include_when));
                } else {
                    task.when = Some(include_when.clone());
                }
            }
            for handler in &mut play.handlers {
                if let Some(existing_when) = &handler.when {
                    handler.when = Some(format!("({}) and ({})", existing_when, include_when));
                } else {
                    handler.when = Some(include_when.clone());
                }
            }
        }

        // Apply apply block properties
        if let Some(apply_spec) = &include_spec.apply {
            if let Some(apply_tags) = &apply_spec.tags {
                for task in &mut play.tasks {
                    task.tags.extend(apply_tags.clone());
                }
                for handler in &mut play.handlers {
                    handler.tags.extend(apply_tags.clone());
                }
            }

            if let Some(apply_when) = &apply_spec.when_condition {
                for task in &mut play.tasks {
                    if let Some(existing_when) = &task.when {
                        task.when = Some(format!("({}) and ({})", existing_when, apply_when));
                    } else {
                        task.when = Some(apply_when.clone());
                    }
                }
                for handler in &mut play.handlers {
                    if let Some(existing_when) = &handler.when {
                        handler.when = Some(format!("({}) and ({})", existing_when, apply_when));
                    } else {
                        handler.when = Some(apply_when.clone());
                    }
                }
            }
        }

        Ok(play)
    }

    /// Simple role parser for included playbooks
    fn parse_role_simple(&self, raw_role: RawRole) -> Result<ParsedRole, ParseError> {
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
}

/// Statistics about include processing
#[derive(Debug, Clone)]
pub struct IncludeStats {
    pub current_depth: usize,
    pub max_depth: usize,
    pub cache_stats: crate::parser::include::cache::CacheStats,
}

// Placeholder raw task structure - should match the one in playbook.rs
#[derive(Debug, serde::Deserialize)]
struct RawTask {
    id: Option<String>,
    name: Option<String>,
    vars: Option<HashMap<String, serde_json::Value>>,
    when: Option<String>,
    #[serde(rename = "loop")]
    loop_items: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
    notify: Option<Vec<String>>,
    changed_when: Option<String>,
    failed_when: Option<String>,
    #[serde(deserialize_with = "deserialize_yaml_bool", default)]
    ignore_errors: Option<bool>,
    delegate_to: Option<String>,
    #[serde(rename = "become", deserialize_with = "deserialize_yaml_bool", default)]
    r#become: Option<bool>,
    become_user: Option<String>,
    become_method: Option<String>,
    register: Option<String>,
    #[serde(flatten)]
    module_args: HashMap<String, serde_json::Value>,
}

// Custom deserializer for boolean fields that handles YAML boolean strings
fn deserialize_yaml_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_yaml::Value> = serde::Deserialize::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_yaml::Value::Bool(b)) => Ok(Some(b)),
        Some(serde_yaml::Value::String(s)) => match s.to_lowercase().as_str() {
            "yes" | "true" | "on" => Ok(Some(true)),
            "no" | "false" | "off" => Ok(Some(false)),
            _ => Err(Error::custom(format!("Invalid boolean string: {}", s))),
        },
        Some(_) => Err(Error::custom("Expected boolean or boolean string")),
    }
}

// Raw data structures for playbook parsing
#[derive(Debug, serde::Deserialize)]
struct RawPlay {
    name: Option<String>,
    hosts: Option<RawHostPattern>,
    vars: Option<HashMap<String, serde_json::Value>>,
    tasks: Option<Vec<serde_yaml::Value>>,
    handlers: Option<Vec<serde_yaml::Value>>,
    roles: Option<Vec<RawRole>>,
    strategy: Option<ExecutionStrategy>,
    serial: Option<u32>,
    max_fail_percentage: Option<f32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum RawHostPattern {
    Single(String),
    Multiple(Vec<String>),
    All,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum RawRole {
    String(String),
    Object(RawRoleObject),
}

#[derive(Debug, serde::Deserialize)]
struct RawRoleObject {
    name: String,
    src: Option<String>,
    version: Option<String>,
    vars: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_basic_include_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create test files
        fs::create_dir_all(temp_dir.path().join("tasks")).unwrap();
        fs::write(
            temp_dir.path().join("tasks/setup.yml"),
            r#"
- name: Install package
  package:
    name: git
    state: present

- name: Create user
  user:
    name: deploy
    state: present
"#,
        )
        .unwrap();

        let template_engine = TemplateEngine::new();
        let mut include_handler = IncludeHandler::new(base_path, template_engine);

        let include_spec = IncludeSpec {
            file: "tasks/setup.yml".to_string(),
            vars: None,
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("main.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let tasks = include_handler
            .include_tasks(&include_spec, &context)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].name, "Install package");
        assert_eq!(tasks[1].name, "Create user");
    }

    #[tokio::test]
    async fn test_include_with_variables() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        // Create parameterized task file
        fs::create_dir_all(temp_dir.path().join("tasks")).unwrap();
        fs::write(
            temp_dir.path().join("tasks/parameterized.yml"),
            r#"
- name: Install {{ package_name }}
  package:
    name: "{{ package_name }}"
    state: present
"#,
        )
        .unwrap();

        let template_engine = TemplateEngine::new();
        let mut include_handler = IncludeHandler::new(base_path, template_engine);

        let mut include_vars = HashMap::new();
        include_vars.insert(
            "package_name".to_string(),
            serde_json::Value::String("nginx".to_string()),
        );

        let include_spec = IncludeSpec {
            file: "tasks/parameterized.yml".to_string(),
            vars: Some(include_vars),
            when_condition: None,
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };

        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("main.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };

        let tasks = include_handler
            .include_tasks(&include_spec, &context)
            .await
            .unwrap();

        assert_eq!(tasks.len(), 1);
        // Since we don't render templates during parsing, check for the template
        assert_eq!(tasks[0].name, "Install {{ package_name }}");
    }
}
