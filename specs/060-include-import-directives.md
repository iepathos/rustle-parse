# Spec 060: Include and Import Directives

## Feature Summary

Implement comprehensive support for Ansible's include and import directives including `include_tasks`, `import_tasks`, `include_playbook`, `import_playbook`, `include_vars`, and `include_role`. This enables modular playbook organization, code reuse, and dynamic task loading that are essential for real-world Ansible projects.

**Problem it solves**: The current parser only handles basic playbook structure without support for includes and imports. Real Ansible projects extensively use these directives for code organization, reusability, and conditional loading of tasks, variables, and entire playbooks.

**High-level approach**: Implement recursive parsing for include/import directives with proper variable scoping, dependency tracking, conditional evaluation, and circular dependency detection. Support both static imports (parse-time) and dynamic includes (runtime).

## Goals & Requirements

### Functional Requirements
- Support all Ansible include/import directives
- Handle static imports during parse time
- Support dynamic includes with conditional evaluation
- Implement proper variable scoping and inheritance
- Track dependencies and detect circular references
- Support relative and absolute file paths
- Handle file not found errors gracefully
- Support conditional includes with `when` clauses
- Implement task tagging for included content
- Support parameterized includes with variables

### Non-functional Requirements
- **Performance**: Parse includes efficiently with caching
- **Memory**: Avoid memory leaks from circular references
- **Security**: Validate file paths to prevent directory traversal
- **Compatibility**: 100% compatible with Ansible include/import behavior
- **Error Handling**: Clear error messages with include chain context

### Success Criteria
- All include/import directives work correctly
- Circular dependency detection prevents infinite loops
- Variable scoping matches Ansible behavior exactly
- Complex real-world playbooks with includes parse successfully
- Performance acceptable for deep include hierarchies

## API/Interface Design

### Include/Import Handler Interface
```rust
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct IncludeHandler {
    base_path: PathBuf,
    template_engine: TemplateEngine,
    include_cache: HashMap<PathBuf, CachedInclude>,
    include_stack: Vec<PathBuf>,  // For circular dependency detection
    max_include_depth: usize,
}

impl IncludeHandler {
    pub fn new(base_path: PathBuf, template_engine: TemplateEngine) -> Self;
    
    /// Process include_tasks directive
    pub async fn include_tasks(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError>;
    
    /// Process import_tasks directive
    pub async fn import_tasks(
        &mut self,
        import_spec: &ImportSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError>;
    
    /// Process include_playbook directive
    pub async fn include_playbook(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<ParsedPlaybook, ParseError>;
    
    /// Process import_playbook directive
    pub async fn import_playbook(
        &mut self,
        import_spec: &ImportSpec,
        context: &IncludeContext,
    ) -> Result<ParsedPlaybook, ParseError>;
    
    /// Process include_vars directive
    pub async fn include_vars(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError>;
    
    /// Process include_role directive
    pub async fn include_role(
        &mut self,
        role_spec: &RoleIncludeSpec,
        context: &IncludeContext,
    ) -> Result<ParsedRole, ParseError>;
    
    /// Resolve file path relative to current context
    pub fn resolve_path(&self, file_path: &str, current_file: &Path) -> Result<PathBuf, ParseError>;
    
    /// Check for circular dependencies
    pub fn check_circular_dependency(&self, file_path: &Path) -> Result<(), ParseError>;
    
    /// Clear include cache
    pub fn clear_cache(&mut self);
}

#[derive(Debug, Clone)]
pub struct IncludeSpec {
    pub file: String,
    pub vars: Option<HashMap<String, serde_json::Value>>,
    pub when: Option<String>,
    pub tags: Option<Vec<String>>,
    pub apply: Option<ApplySpec>,
    pub delegate_to: Option<String>,
    pub delegate_facts: Option<bool>,
    pub run_once: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ImportSpec {
    pub file: String,
    pub vars: Option<HashMap<String, serde_json::Value>>,
    pub when: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct RoleIncludeSpec {
    pub name: String,
    pub tasks_from: Option<String>,
    pub vars_from: Option<String>,
    pub defaults_from: Option<String>,
    pub handlers_from: Option<String>,
    pub vars: Option<HashMap<String, serde_json::Value>>,
    pub when: Option<String>,
    pub tags: Option<Vec<String>>,
    pub apply: Option<ApplySpec>,
}

#[derive(Debug, Clone)]
pub struct ApplySpec {
    pub tags: Option<Vec<String>>,
    pub when: Option<String>,
    pub become: Option<bool>,
    pub become_user: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IncludeContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub current_file: PathBuf,
    pub include_depth: usize,
    pub tags: Vec<String>,
    pub when_condition: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedInclude {
    content: String,
    parsed_at: std::time::SystemTime,
    file_modified: std::time::SystemTime,
}
```

### Enhanced Playbook Parser Integration
```rust
impl<'a> PlaybookParser<'a> {
    /// Parse playbook with include/import support
    pub async fn parse_with_includes(&self, path: &Path) -> Result<ParsedPlaybook, ParseError> {
        let mut include_handler = IncludeHandler::new(
            path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
            self.template_engine.clone(),
        );
        
        self.parse_playbook_recursive(path, &mut include_handler).await
    }
    
    async fn parse_playbook_recursive(
        &self,
        path: &Path,
        include_handler: &mut IncludeHandler,
    ) -> Result<ParsedPlaybook, ParseError>;
    
    async fn process_include_directive(
        &self,
        directive: &IncludeDirective,
        include_handler: &mut IncludeHandler,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError>;
}

#[derive(Debug, Deserialize)]
#[serde(tag = "directive")]
enum IncludeDirective {
    #[serde(rename = "include_tasks")]
    IncludeTasks(IncludeSpec),
    
    #[serde(rename = "import_tasks")]
    ImportTasks(ImportSpec),
    
    #[serde(rename = "include_playbook")]
    IncludePlaybook(IncludeSpec),
    
    #[serde(rename = "import_playbook")]
    ImportPlaybook(ImportSpec),
    
    #[serde(rename = "include_vars")]
    IncludeVars(IncludeVarsSpec),
    
    #[serde(rename = "include_role")]
    IncludeRole(RoleIncludeSpec),
    
    #[serde(rename = "import_role")]
    ImportRole(RoleIncludeSpec),
}

#[derive(Debug, Deserialize)]
pub struct IncludeVarsSpec {
    pub file: Option<String>,
    pub dir: Option<String>,
    pub name: Option<String>,
    pub depth: Option<usize>,
    pub files_matching: Option<String>,
    pub ignore_files: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub when: Option<String>,
}
```

### Path Resolution and Security
```rust
impl IncludeHandler {
    pub fn resolve_path(&self, file_path: &str, current_file: &Path) -> Result<PathBuf, ParseError> {
        // Start with the provided path
        let path = Path::new(file_path);
        
        let resolved = if path.is_absolute() {
            // Absolute path - validate it's within allowed directories
            self.validate_absolute_path(path)?
        } else {
            // Relative path - resolve relative to current file's directory
            let current_dir = current_file.parent()
                .unwrap_or_else(|| Path::new("."));
            current_dir.join(path)
        };
        
        // Canonicalize and validate the final path
        let canonical = resolved.canonicalize()
            .map_err(|e| ParseError::FileNotFound {
                path: resolved.to_string_lossy().to_string(),
            })?;
        
        self.validate_resolved_path(&canonical)?;
        
        Ok(canonical)
    }
    
    fn validate_absolute_path(&self, path: &Path) -> Result<PathBuf, ParseError> {
        // Ensure absolute paths are within the base directory or common locations
        let allowed_prefixes = [
            &self.base_path,
            Path::new("/etc/ansible"),
            Path::new("/usr/share/ansible"),
        ];
        
        for allowed in &allowed_prefixes {
            if path.starts_with(allowed) {
                return Ok(path.to_path_buf());
            }
        }
        
        Err(ParseError::SecurityViolation {
            message: format!("Absolute path '{}' not in allowed directories", path.display()),
        })
    }
    
    fn validate_resolved_path(&self, path: &Path) -> Result<(), ParseError> {
        // Prevent directory traversal attacks
        if !path.starts_with(&self.base_path) {
            return Err(ParseError::SecurityViolation {
                message: format!("Path '{}' attempts to access files outside base directory", path.display()),
            });
        }
        
        // Check for suspicious path components
        for component in path.components() {
            if let std::path::Component::Normal(os_str) = component {
                if let Some(str_component) = os_str.to_str() {
                    if str_component.starts_with('.') && str_component.len() > 1 {
                        return Err(ParseError::SecurityViolation {
                            message: format!("Hidden file access not allowed: {}", str_component),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

## File and Package Structure

### Include/Import Module Structure
```
src/
├── parser/
│   ├── include/
│   │   ├── mod.rs                 # Include module exports
│   │   ├── handler.rs             # Main include/import handler
│   │   ├── resolver.rs            # Path resolution and validation
│   │   ├── cache.rs               # Include result caching
│   │   ├── tasks.rs               # Task include/import logic
│   │   ├── playbooks.rs           # Playbook include/import logic
│   │   ├── variables.rs           # Variable include logic
│   │   ├── roles.rs               # Role include/import logic
│   │   └── dependency.rs          # Circular dependency detection
│   ├── playbook.rs                # Enhanced with include support
│   └── error.rs                   # Add include-related errors
├── types/
│   ├── include.rs                 # Include-related types
│   └── parsed.rs                  # Enhanced with include metadata
└── ...

tests/
├── fixtures/
│   ├── includes/
│   │   ├── basic/                 # Basic include scenarios
│   │   │   ├── main.yml
│   │   │   ├── tasks/
│   │   │   │   ├── setup.yml
│   │   │   │   └── deploy.yml
│   │   │   └── vars/
│   │   │       └── main.yml
│   │   ├── nested/                # Nested include scenarios
│   │   ├── conditional/           # Conditional includes
│   │   ├── circular/              # Circular dependency tests
│   │   ├── roles/                 # Role include tests
│   │   └── edge_cases/            # Error conditions
│   └── expected/
│       └── includes/              # Expected parsing results
└── parser/
    ├── include_handler_tests.rs   # Core include handler tests
    ├── include_integration_tests.rs # Integration tests
    └── circular_dependency_tests.rs # Circular dependency tests
```

## Implementation Details

### Phase 1: Basic Include/Import Support
```rust
// src/parser/include/handler.rs
impl IncludeHandler {
    pub async fn include_tasks(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        // Resolve the file path
        let resolved_path = self.resolve_path(&include_spec.file, &context.current_file)?;
        
        // Check for circular dependencies
        self.check_circular_dependency(&resolved_path)?;
        
        // Add to include stack
        self.include_stack.push(resolved_path.clone());
        
        // Check include depth
        if context.include_depth >= self.max_include_depth {
            return Err(ParseError::MaxIncludeDepthExceeded {
                depth: self.max_include_depth,
                file: resolved_path.to_string_lossy().to_string(),
            });
        }
        
        // Load and parse the included file
        let content = self.load_file_cached(&resolved_path).await?;
        
        // Create context for included tasks
        let mut include_context = context.clone();
        include_context.current_file = resolved_path.clone();
        include_context.include_depth += 1;
        
        // Merge include variables
        if let Some(include_vars) = &include_spec.vars {
            include_context.variables.extend(include_vars.clone());
        }
        
        // Parse tasks from included file
        let raw_tasks: Vec<RawTask> = serde_yaml::from_str(&content)
            .map_err(|e| ParseError::YamlSyntax {
                line: e.location().map(|l| l.line()).unwrap_or(0),
                column: e.location().map(|l| l.column()).unwrap_or(0),
                message: e.to_string(),
            })?;
        
        let mut parsed_tasks = Vec::new();
        for (index, raw_task) in raw_tasks.into_iter().enumerate() {
            // Check for nested includes in tasks
            if self.is_include_directive(&raw_task) {
                let include_directive = self.parse_include_directive(&raw_task)?;
                let nested_tasks = self.process_include_directive(
                    &include_directive,
                    &include_context,
                ).await?;
                parsed_tasks.extend(nested_tasks);
            } else {
                let task = self.parse_task_with_context(raw_task, &include_context, index).await?;
                
                // Apply include-level properties
                let enhanced_task = self.apply_include_properties(task, include_spec)?;
                parsed_tasks.push(enhanced_task);
            }
        }
        
        // Remove from include stack
        self.include_stack.pop();
        
        Ok(parsed_tasks)
    }
    
    pub async fn import_tasks(
        &mut self,
        import_spec: &ImportSpec,
        context: &IncludeContext,
    ) -> Result<Vec<ParsedTask>, ParseError> {
        // Import is processed at parse time, similar to include_tasks
        // but with different variable scoping rules
        
        let resolved_path = self.resolve_path(&import_spec.file, &context.current_file)?;
        self.check_circular_dependency(&resolved_path)?;
        
        // For imports, we process the content immediately and merge into current context
        let content = self.load_file_cached(&resolved_path).await?;
        
        // Parse and process tasks immediately
        let raw_tasks: Vec<RawTask> = serde_yaml::from_str(&content)?;
        let mut parsed_tasks = Vec::new();
        
        for (index, raw_task) in raw_tasks.into_iter().enumerate() {
            // Apply import variables at parse time
            let processed_task = self.apply_import_variables(raw_task, import_spec)?;
            let task = self.parse_task_with_context(processed_task, context, index).await?;
            parsed_tasks.push(task);
        }
        
        Ok(parsed_tasks)
    }
    
    async fn load_file_cached(&mut self, path: &Path) -> Result<String, ParseError> {
        // Check cache first
        if let Some(cached) = self.include_cache.get(path) {
            // Check if file has been modified since cache
            if let Ok(metadata) = fs::metadata(path).await {
                if let Ok(modified) = metadata.modified() {
                    if modified <= cached.file_modified {
                        return Ok(cached.content.clone());
                    }
                }
            }
        }
        
        // Load file and update cache
        let content = fs::read_to_string(path).await
            .map_err(|e| ParseError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            })?;
        
        let file_modified = fs::metadata(path).await
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| std::time::SystemTime::now());
        
        let cached_include = CachedInclude {
            content: content.clone(),
            parsed_at: std::time::SystemTime::now(),
            file_modified,
        };
        
        self.include_cache.insert(path.to_path_buf(), cached_include);
        
        Ok(content)
    }
}
```

### Phase 2: Variable Include Support
```rust
// src/parser/include/variables.rs
impl IncludeHandler {
    pub async fn include_vars(
        &mut self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let resolved_path = self.resolve_path(&include_spec.file, &context.current_file)?;
        let content = self.load_file_cached(&resolved_path).await?;
        
        // Parse variables file
        let vars: HashMap<String, serde_json::Value> = if resolved_path.extension()
            .and_then(|ext| ext.to_str()) == Some("json") {
            // JSON variables file
            serde_json::from_str(&content)
                .map_err(|e| ParseError::Json(e))?
        } else {
            // YAML variables file
            serde_yaml::from_str(&content)
                .map_err(|e| ParseError::Yaml(e))?
        };
        
        // Process variables through template engine if needed
        let mut processed_vars = HashMap::new();
        for (key, value) in vars {
            let processed_value = self.template_engine.render_value(&value, &context.variables)?;
            processed_vars.insert(key, processed_value);
        }
        
        Ok(processed_vars)
    }
    
    pub async fn include_vars_from_dir(
        &mut self,
        vars_spec: &IncludeVarsSpec,
        context: &IncludeContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let dir_path = vars_spec.dir.as_ref()
            .ok_or_else(|| ParseError::InvalidStructure {
                message: "include_vars with dir requires 'dir' parameter".to_string(),
            })?;
        
        let resolved_dir = self.resolve_path(dir_path, &context.current_file)?;
        
        if !resolved_dir.is_dir() {
            return Err(ParseError::FileNotFound {
                path: resolved_dir.to_string_lossy().to_string(),
            });
        }
        
        let mut all_vars = HashMap::new();
        let max_depth = vars_spec.depth.unwrap_or(1);
        let extensions = vars_spec.extensions.as_ref()
            .cloned()
            .unwrap_or_else(|| vec!["yml".to_string(), "yaml".to_string(), "json".to_string()]);
        
        self.load_vars_recursive(&resolved_dir, &mut all_vars, 0, max_depth, &extensions, context).await?;
        
        Ok(all_vars)
    }
    
    async fn load_vars_recursive(
        &mut self,
        dir: &Path,
        vars: &mut HashMap<String, serde_json::Value>,
        current_depth: usize,
        max_depth: usize,
        extensions: &[String],
        context: &IncludeContext,
    ) -> Result<(), ParseError> {
        if current_depth >= max_depth {
            return Ok(());
        }
        
        let mut entries = fs::read_dir(dir).await
            .map_err(|e| ParseError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| ParseError::Io(e))? {
            let path = entry.path();
            
            if path.is_dir() && current_depth + 1 < max_depth {
                self.load_vars_recursive(&path, vars, current_depth + 1, max_depth, extensions, context).await?;
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext.to_string()) {
                        let file_vars = self.load_vars_file(&path, context).await?;
                        vars.extend(file_vars);
                    }
                }
            }
        }
        
        Ok(())
    }
}
```

### Phase 3: Role Include Support
```rust
// src/parser/include/roles.rs
impl IncludeHandler {
    pub async fn include_role(
        &mut self,
        role_spec: &RoleIncludeSpec,
        context: &IncludeContext,
    ) -> Result<ParsedRole, ParseError> {
        let role_path = self.resolve_role_path(&role_spec.name, &context.current_file)?;
        
        let mut role = ParsedRole {
            name: role_spec.name.clone(),
            src: None,
            version: None,
            vars: HashMap::new(),
            tags: Vec::new(),
        };
        
        // Load role components based on spec
        if let Some(tasks_from) = &role_spec.tasks_from {
            let tasks_path = role_path.join("tasks").join(format!("{}.yml", tasks_from));
            if tasks_path.exists() {
                let tasks = self.load_role_tasks(&tasks_path, context).await?;
                // Store tasks in role metadata (extend ParsedRole if needed)
            }
        } else {
            // Load default main.yml
            let main_tasks = role_path.join("tasks").join("main.yml");
            if main_tasks.exists() {
                let tasks = self.load_role_tasks(&main_tasks, context).await?;
            }
        }
        
        // Load role variables
        if let Some(vars_from) = &role_spec.vars_from {
            let vars_path = role_path.join("vars").join(format!("{}.yml", vars_from));
            if vars_path.exists() {
                let vars = self.load_vars_file(&vars_path, context).await?;
                role.vars.extend(vars);
            }
        }
        
        // Load role defaults
        if let Some(defaults_from) = &role_spec.defaults_from {
            let defaults_path = role_path.join("defaults").join(format!("{}.yml", defaults_from));
            if defaults_path.exists() {
                let defaults = self.load_vars_file(&defaults_path, context).await?;
                // Defaults have lower precedence than vars
                for (key, value) in defaults {
                    role.vars.entry(key).or_insert(value);
                }
            }
        }
        
        // Merge role spec variables
        if let Some(spec_vars) = &role_spec.vars {
            role.vars.extend(spec_vars.clone());
        }
        
        // Apply tags
        if let Some(tags) = &role_spec.tags {
            role.tags.extend(tags.clone());
        }
        
        Ok(role)
    }
    
    fn resolve_role_path(&self, role_name: &str, current_file: &Path) -> Result<PathBuf, ParseError> {
        // Try multiple locations for roles
        let search_paths = [
            current_file.parent().unwrap_or_else(|| Path::new(".")).join("roles"),
            current_file.parent().unwrap_or_else(|| Path::new(".")).join("..").join("roles"),
            Path::new("/etc/ansible/roles").to_path_buf(),
            Path::new("~/.ansible/roles").to_path_buf(), // TODO: Expand ~ properly
        ];
        
        for search_path in &search_paths {
            let role_path = search_path.join(role_name);
            if role_path.exists() && role_path.is_dir() {
                return Ok(role_path);
            }
        }
        
        Err(ParseError::RoleNotFound {
            role: role_name.to_string(),
            searched_paths: search_paths.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        })
    }
}
```

### Phase 4: Circular Dependency Detection
```rust
// src/parser/include/dependency.rs
impl IncludeHandler {
    pub fn check_circular_dependency(&self, file_path: &Path) -> Result<(), ParseError> {
        if self.include_stack.contains(file_path) {
            let cycle = self.build_cycle_description(file_path);
            return Err(ParseError::CircularDependency { cycle });
        }
        Ok(())
    }
    
    fn build_cycle_description(&self, file_path: &Path) -> String {
        let mut cycle_files = Vec::new();
        let mut found_start = false;
        
        for stack_file in &self.include_stack {
            if stack_file == file_path {
                found_start = true;
            }
            if found_start {
                cycle_files.push(stack_file.to_string_lossy().to_string());
            }
        }
        
        cycle_files.push(file_path.to_string_lossy().to_string());
        cycle_files.join(" -> ")
    }
    
    pub fn analyze_include_dependencies(&self, playbook: &ParsedPlaybook) -> IncludeDependencyGraph {
        let mut graph = IncludeDependencyGraph::new();
        
        for play in &playbook.plays {
            for task in &play.tasks {
                if let Some(include_info) = &task.include_info {
                    graph.add_dependency(
                        playbook.metadata.file_path.clone(),
                        include_info.included_file.clone(),
                        include_info.include_type.clone(),
                    );
                }
            }
        }
        
        graph
    }
}

#[derive(Debug)]
pub struct IncludeDependencyGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<IncludeDependency>>,
}

#[derive(Debug, Clone)]
pub struct IncludeDependency {
    pub target_file: String,
    pub include_type: IncludeType,
    pub conditional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncludeType {
    IncludeTasks,
    ImportTasks,
    IncludePlaybook,
    ImportPlaybook,
    IncludeVars,
    IncludeRole,
}
```

## Testing Strategy

### Unit Testing Requirements
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_basic_include_tasks() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create main playbook
        let main_content = r#"
---
- hosts: all
  tasks:
    - include_tasks: tasks/setup.yml
"#;
        
        // Create included tasks
        let setup_content = r#"
- name: Install package
  package:
    name: git
    state: present

- name: Create user
  user:
    name: deploy
    state: present
"#;
        
        fs::create_dir_all(temp_dir.path().join("tasks")).await.unwrap();
        fs::write(temp_dir.path().join("main.yml"), main_content).await.unwrap();
        fs::write(temp_dir.path().join("tasks/setup.yml"), setup_content).await.unwrap();
        
        let mut include_handler = IncludeHandler::new(
            temp_dir.path().to_path_buf(),
            TemplateEngine::new(),
        );
        
        let include_spec = IncludeSpec {
            file: "tasks/setup.yml".to_string(),
            vars: None,
            when: None,
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
        
        let tasks = include_handler.include_tasks(&include_spec, &context).await.unwrap();
        
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].name, "Install package");
        assert_eq!(tasks[1].name, "Create user");
    }
    
    #[tokio::test]
    async fn test_include_with_variables() {
        let temp_dir = TempDir::new().unwrap();
        
        let main_content = r#"
---
- hosts: all
  tasks:
    - include_tasks: tasks/parameterized.yml
      vars:
        package_name: nginx
        service_state: started
"#;
        
        let parameterized_content = r#"
- name: Install {{ package_name }}
  package:
    name: "{{ package_name }}"
    state: present

- name: Ensure service is {{ service_state }}
  service:
    name: "{{ package_name }}"
    state: "{{ service_state }}"
"#;
        
        fs::create_dir_all(temp_dir.path().join("tasks")).await.unwrap();
        fs::write(temp_dir.path().join("main.yml"), main_content).await.unwrap();
        fs::write(temp_dir.path().join("tasks/parameterized.yml"), parameterized_content).await.unwrap();
        
        // Test with variables
        let mut include_handler = IncludeHandler::new(
            temp_dir.path().to_path_buf(),
            TemplateEngine::new(),
        );
        
        let include_spec = IncludeSpec {
            file: "tasks/parameterized.yml".to_string(),
            vars: Some({
                let mut vars = HashMap::new();
                vars.insert("package_name".to_string(), serde_json::Value::String("nginx".to_string()));
                vars.insert("service_state".to_string(), serde_json::Value::String("started".to_string()));
                vars
            }),
            when: None,
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
        
        let tasks = include_handler.include_tasks(&include_spec, &context).await.unwrap();
        
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].name, "Install nginx");
        assert_eq!(tasks[1].name, "Ensure service is started");
    }
    
    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let temp_dir = TempDir::new().unwrap();
        
        let file_a = r#"
- include_tasks: b.yml
"#;
        
        let file_b = r#"
- include_tasks: a.yml
"#;
        
        fs::write(temp_dir.path().join("a.yml"), file_a).await.unwrap();
        fs::write(temp_dir.path().join("b.yml"), file_b).await.unwrap();
        
        let mut include_handler = IncludeHandler::new(
            temp_dir.path().to_path_buf(),
            TemplateEngine::new(),
        );
        
        let include_spec = IncludeSpec {
            file: "b.yml".to_string(),
            vars: None,
            when: None,
            tags: None,
            apply: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
        };
        
        let context = IncludeContext {
            variables: HashMap::new(),
            current_file: temp_dir.path().join("a.yml"),
            include_depth: 0,
            tags: Vec::new(),
            when_condition: None,
        };
        
        // First include should work
        include_handler.include_stack.push(temp_dir.path().join("a.yml"));
        
        // Second include should detect circular dependency
        let result = include_handler.include_tasks(&include_spec, &context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::CircularDependency { .. }));
    }
}
```

### Integration Testing
```rust
// tests/parser/include_integration_tests.rs
#[tokio::test]
async fn test_complex_nested_includes() {
    let temp_dir = setup_complex_include_structure().await;
    
    let parser = PlaybookParser::new(&TemplateEngine::new(), &HashMap::new());
    let playbook = parser.parse_with_includes(&temp_dir.path().join("site.yml")).await.unwrap();
    
    // Verify the entire include hierarchy was processed correctly
    assert!(playbook.plays.len() > 0);
    
    let mut all_tasks = Vec::new();
    for play in &playbook.plays {
        all_tasks.extend(&play.tasks);
    }
    
    // Should have tasks from all included files
    assert!(all_tasks.len() >= 10); // Expected minimum based on test structure
    
    // Verify specific tasks from different include levels
    assert!(all_tasks.iter().any(|t| t.name.contains("Install base packages")));
    assert!(all_tasks.iter().any(|t| t.name.contains("Configure nginx")));
    assert!(all_tasks.iter().any(|t| t.name.contains("Deploy application")));
}

async fn setup_complex_include_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    
    // Create directory structure
    fs::create_dir_all(temp_dir.path().join("roles/common/tasks")).await.unwrap();
    fs::create_dir_all(temp_dir.path().join("roles/webserver/tasks")).await.unwrap();
    fs::create_dir_all(temp_dir.path().join("playbooks")).await.unwrap();
    fs::create_dir_all(temp_dir.path().join("group_vars")).await.unwrap();
    
    // Main site playbook
    let site_content = r#"
---
- import_playbook: playbooks/base.yml
- import_playbook: playbooks/webservers.yml
"#;
    
    // Base playbook
    let base_content = r#"
---
- hosts: all
  roles:
    - common
"#;
    
    // Webservers playbook
    let webservers_content = r#"
---
- hosts: webservers
  tasks:
    - include_role:
        name: webserver
    - include_tasks: tasks/deploy.yml
      vars:
        app_name: myapp
"#;
    
    // Write all files
    fs::write(temp_dir.path().join("site.yml"), site_content).await.unwrap();
    fs::write(temp_dir.path().join("playbooks/base.yml"), base_content).await.unwrap();
    fs::write(temp_dir.path().join("playbooks/webservers.yml"), webservers_content).await.unwrap();
    
    // Add role files, task files, etc.
    // ... (additional file setup)
    
    temp_dir
}
```

## Edge Cases & Error Handling

### Include-Specific Error Types
```rust
#[derive(Debug, Error)]
pub enum ParseError {
    // ... existing errors ...
    
    #[error("Circular dependency detected in includes: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Maximum include depth exceeded: {depth} levels deep in file '{file}'")]
    MaxIncludeDepthExceeded { depth: usize, file: String },
    
    #[error("Role '{role}' not found. Searched paths: {searched_paths:?}")]
    RoleNotFound { role: String, searched_paths: Vec<String> },
    
    #[error("Security violation: {message}")]
    SecurityViolation { message: String },
    
    #[error("Include file '{file}' not found or not accessible")]
    IncludeFileNotFound { file: String },
    
    #[error("Invalid include directive: {message}")]
    InvalidIncludeDirective { message: String },
    
    #[error("Include variable resolution failed: {variable} in {file}")]
    IncludeVariableResolution { variable: String, file: String },
}
```

### Conditional Include Processing
```rust
impl IncludeHandler {
    async fn should_process_include(
        &self,
        include_spec: &IncludeSpec,
        context: &IncludeContext,
    ) -> Result<bool, ParseError> {
        if let Some(when_condition) = &include_spec.when {
            // Evaluate the when condition using template engine
            let result = self.template_engine.render_string(
                &format!("{{{{ {} }}}}", when_condition),
                &context.variables,
            )?;
            
            // Parse result as boolean
            match result.trim().to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(true),
                "false" | "no" | "0" => Ok(false),
                _ => {
                    // Try to parse as boolean-ish value
                    Ok(!result.trim().is_empty())
                }
            }
        } else {
            Ok(true)
        }
    }
    
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
        if let Some(include_when) = &include_spec.when {
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
            
            if let Some(apply_when) = &apply_spec.when {
                if let Some(existing_when) = &task.when {
                    task.when = Some(format!("({}) and ({})", existing_when, apply_when));
                } else {
                    task.when = Some(apply_when.clone());
                }
            }
        }
        
        Ok(task)
    }
}
```

## Dependencies

### No New Dependencies Required
The include/import functionality can be implemented using existing dependencies:
- `serde_yaml` for parsing included YAML files
- `tokio::fs` for async file I/O
- `std::path` for path resolution
- Existing template engine for variable processing

### Configuration Enhancements
```rust
#[derive(Debug, Clone)]
pub struct IncludeConfig {
    pub max_include_depth: usize,
    pub enable_include_cache: bool,
    pub cache_ttl: Duration,
    pub strict_file_permissions: bool,
    pub allow_absolute_paths: bool,
    pub role_search_paths: Vec<PathBuf>,
}

impl Default for IncludeConfig {
    fn default() -> Self {
        Self {
            max_include_depth: 100,
            enable_include_cache: true,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            strict_file_permissions: true,
            allow_absolute_paths: false,
            role_search_paths: vec![
                PathBuf::from("roles"),
                PathBuf::from("../roles"),
                PathBuf::from("/etc/ansible/roles"),
            ],
        }
    }
}
```

## Performance Considerations

### Caching Strategy
- Cache parsed include files to avoid re-parsing
- Track file modification times for cache invalidation
- Use LRU cache with configurable size limits
- Cache template compilation results

### Memory Management
```rust
impl IncludeHandler {
    pub fn optimize_memory_usage(&mut self) {
        // Clear old cache entries
        let now = std::time::SystemTime::now();
        self.include_cache.retain(|_, cached| {
            now.duration_since(cached.parsed_at)
                .map(|duration| duration < self.cache_ttl)
                .unwrap_or(false)
        });
        
        // Limit cache size
        if self.include_cache.len() > self.max_cache_size {
            // Remove oldest entries
            let mut entries: Vec<_> = self.include_cache.iter().collect();
            entries.sort_by_key(|(_, cached)| cached.parsed_at);
            
            let to_remove = entries.len() - self.max_cache_size;
            for (path, _) in entries.iter().take(to_remove) {
                self.include_cache.remove(*path);
            }
        }
    }
}
```

## Implementation Phases

### Phase 1: Basic Include Support (Week 1-2)
- [ ] Implement basic include_tasks and import_tasks
- [ ] Add path resolution and validation
- [ ] Basic error handling and circular dependency detection
- [ ] Simple variable merging
- [ ] Unit tests for core functionality

### Phase 2: Advanced Include Features (Week 3)
- [ ] Add include_playbook and import_playbook support
- [ ] Implement include_vars functionality
- [ ] Add conditional include processing
- [ ] Enhanced variable scoping
- [ ] Include result caching

### Phase 3: Role Integration (Week 4)
- [ ] Implement include_role and import_role
- [ ] Add role path resolution
- [ ] Support role component loading (tasks, vars, defaults, handlers)
- [ ] Role dependency handling
- [ ] Integration tests with roles

### Phase 4: Production Readiness (Week 5)
- [ ] Performance optimization and memory management
- [ ] Security hardening and path validation
- [ ] Comprehensive error handling
- [ ] Real-world include scenario testing
- [ ] Documentation and examples

## Success Metrics

### Functional Metrics
- All include/import directives work correctly
- Circular dependency detection prevents infinite loops
- Variable scoping matches Ansible behavior exactly
- Complex nested includes parse successfully

### Performance Metrics
- Include processing <100ms for typical scenarios
- Cache hit ratio >80% for repeated includes
- Memory usage scales linearly with include depth
- No memory leaks in long-running processes

### Compatibility Metrics
- 100% compatibility with Ansible include/import syntax
- Real-world playbooks with includes work unchanged
- Error behavior matches Ansible exactly
- All Ansible include test cases pass