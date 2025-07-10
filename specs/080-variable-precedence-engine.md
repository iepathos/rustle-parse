# Spec 080: Variable Precedence Engine

## Feature Summary

Implement Ansible's complex variable precedence system that determines which variable values take priority when the same variable is defined in multiple places. This includes handling precedence between command line vars, playbook vars, host vars, group vars, role defaults, and other variable sources according to Ansible's documented precedence rules.

**Problem it solves**: The current implementation has basic variable merging without proper precedence handling. Real Ansible projects rely on the complex variable precedence system to override defaults with specific configurations, and incorrect precedence can lead to unexpected behavior.

**High-level approach**: Create a comprehensive variable resolution engine that implements Ansible's full precedence hierarchy, handles variable scoping, manages variable inheritance, and provides clear variable source tracking for debugging.

## Goals & Requirements

### Functional Requirements
- Implement complete Ansible variable precedence hierarchy (22 levels)
- Handle variable inheritance from groups to hosts
- Support role variable precedence (defaults < vars < params)
- Manage playbook-level variable scoping
- Handle command-line variable overrides
- Support conditional variable definitions
- Implement variable source tracking for debugging
- Handle special variables (hostvars, groups, etc.)
- Support fact variable integration
- Manage include/import variable scoping

### Non-functional Requirements
- **Performance**: Variable resolution <10ms for typical scenarios
- **Memory**: Efficient memory usage for large variable sets
- **Compatibility**: 100% compatible with Ansible precedence rules
- **Debugging**: Clear variable source information for troubleshooting
- **Maintainability**: Clean separation of precedence logic

### Success Criteria
- All 22 precedence levels implemented correctly
- Variable resolution matches Ansible behavior exactly
- Complex real-world scenarios work correctly
- Variable debugging information is accurate
- Performance acceptable for large inventories

## API/Interface Design

### Variable Precedence Engine
```rust
use std::collections::{HashMap, BTreeMap};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariablePrecedence {
    // Lowest precedence (1)
    RoleDefaults = 1,
    
    // Inventory variables (2-5)
    InventoryFileGroupVars = 2,
    InventoryGroupVarsAll = 3,
    PlaybookGroupVarsAll = 4,
    PlaybookGroupVars = 5,
    
    // Inventory host variables (6-9)
    InventoryFileHostVars = 6,
    InventoryHostVarsAll = 7,
    PlaybookHostVarsAll = 8,
    PlaybookHostVars = 9,
    
    // Host facts and registered variables (10-11)
    HostFacts = 10,
    CachedSetFacts = 11,
    
    // Play variables (12-14)
    PlayVars = 12,
    PlayVarsPrompt = 13,
    PlayVarsFiles = 14,
    
    // Role and task variables (15-17)
    RoleVars = 15,
    IncludeVars = 16,
    SetFactsRegistered = 17,
    
    // Task variables (18-19)
    RoleTaskVars = 18,
    TaskVars = 19,
    
    // Include parameters and block vars (20-21)
    IncludeParams = 20,
    BlockVars = 21,
    
    // Highest precedence (22)
    ExtraVars = 22,
}

pub struct VariablePrecedenceEngine {
    variables: BTreeMap<VariablePrecedence, HashMap<String, VariableValue>>,
    host_variables: HashMap<String, BTreeMap<VariablePrecedence, HashMap<String, VariableValue>>>,
    group_variables: HashMap<String, BTreeMap<VariablePrecedence, HashMap<String, VariableValue>>>,
    variable_sources: HashMap<String, VariableSource>,
    special_variables: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct VariableValue {
    pub value: Value,
    pub source: VariableSource,
    pub conditional: Option<String>,
    pub precedence: VariablePrecedence,
}

#[derive(Debug, Clone)]
pub struct VariableSource {
    pub source_type: VariableSourceType,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
    pub role_name: Option<String>,
    pub host_name: Option<String>,
    pub group_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableSourceType {
    RoleDefaults,
    RoleVars,
    GroupVars,
    HostVars,
    PlayVars,
    TaskVars,
    IncludeVars,
    IncludeParams,
    BlockVars,
    ExtraVars,
    Facts,
    SetFacts,
    RegisteredVars,
    VarsPrompt,
    VarsFiles,
    CommandLine,
}

impl VariablePrecedenceEngine {
    pub fn new() -> Self;
    
    /// Add variables at specific precedence level
    pub fn add_variables(
        &mut self,
        precedence: VariablePrecedence,
        variables: HashMap<String, Value>,
        source: VariableSource,
    );
    
    /// Add host-specific variables
    pub fn add_host_variables(
        &mut self,
        host: &str,
        precedence: VariablePrecedence,
        variables: HashMap<String, Value>,
        source: VariableSource,
    );
    
    /// Add group-specific variables
    pub fn add_group_variables(
        &mut self,
        group: &str,
        precedence: VariablePrecedence,
        variables: HashMap<String, Value>,
        source: VariableSource,
    );
    
    /// Resolve variables for a specific host
    pub fn resolve_host_variables(
        &self,
        host: &str,
        host_groups: &[String],
    ) -> VariableResolutionResult;
    
    /// Resolve variables for a play context
    pub fn resolve_play_variables(
        &self,
        hosts: &[String],
        groups: &[String],
    ) -> HashMap<String, VariableValue>;
    
    /// Resolve variables for a task context
    pub fn resolve_task_variables(
        &self,
        host: &str,
        host_groups: &[String],
        task_vars: &HashMap<String, Value>,
        block_vars: &HashMap<String, Value>,
    ) -> VariableResolutionResult;
    
    /// Get variable source information for debugging
    pub fn get_variable_source(&self, variable_name: &str, host: Option<&str>) -> Option<&VariableSource>;
    
    /// Get all variables that would affect a specific variable
    pub fn debug_variable_precedence(
        &self,
        variable_name: &str,
        host: &str,
        host_groups: &[String],
    ) -> VariableDebugInfo;
    
    /// Handle special variables (hostvars, groups, etc.)
    pub fn resolve_special_variables(&self, inventory: &ParsedInventory) -> HashMap<String, Value>;
}

#[derive(Debug, Clone)]
pub struct VariableResolutionResult {
    pub variables: HashMap<String, Value>,
    pub variable_sources: HashMap<String, VariableSource>,
    pub resolution_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VariableDebugInfo {
    pub variable_name: String,
    pub final_value: Option<Value>,
    pub final_source: Option<VariableSource>,
    pub all_definitions: Vec<VariableDefinition>,
    pub resolution_path: Vec<ResolutionStep>,
}

#[derive(Debug, Clone)]
pub struct VariableDefinition {
    pub value: Value,
    pub source: VariableSource,
    pub precedence: VariablePrecedence,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ResolutionStep {
    pub step: String,
    pub variables_added: usize,
    pub variables_overridden: usize,
    pub precedence_level: VariablePrecedence,
}
```

### Group Variable Inheritance
```rust
pub struct GroupInheritanceEngine {
    group_hierarchy: HashMap<String, Vec<String>>, // group -> parent groups
    group_variables: HashMap<String, HashMap<VariablePrecedence, HashMap<String, VariableValue>>>,
}

impl GroupInheritanceEngine {
    pub fn new() -> Self;
    
    /// Build group hierarchy from inventory
    pub fn build_hierarchy(&mut self, inventory: &ParsedInventory);
    
    /// Resolve group inheritance for a host
    pub fn resolve_group_inheritance(
        &self,
        host_groups: &[String],
    ) -> HashMap<String, VariableValue>;
    
    /// Get all groups that affect a host (including parent groups)
    pub fn get_effective_groups(&self, host_groups: &[String]) -> Vec<String>;
    
    /// Validate group hierarchy for cycles
    pub fn validate_hierarchy(&self) -> Result<(), VariableError>;
}
```

### Template Integration
```rust
impl TemplateEngine {
    /// Render templates with proper variable precedence
    pub fn render_with_precedence(
        &self,
        template: &str,
        engine: &VariablePrecedenceEngine,
        host: &str,
        host_groups: &[String],
    ) -> Result<String, ParseError>;
    
    /// Resolve template variables using precedence engine
    pub fn resolve_template_variables(
        &self,
        value: &Value,
        engine: &VariablePrecedenceEngine,
        host: &str,
        host_groups: &[String],
    ) -> Result<Value, ParseError>;
}
```

## File and Package Structure

### Variable Precedence Module Structure
```
src/
├── parser/
│   ├── variables/
│   │   ├── mod.rs                 # Variables module exports
│   │   ├── precedence.rs          # Main precedence engine
│   │   ├── inheritance.rs         # Group inheritance logic
│   │   ├── resolution.rs          # Variable resolution algorithms
│   │   ├── sources.rs             # Variable source tracking
│   │   ├── special.rs             # Special variables (hostvars, etc.)
│   │   ├── debugging.rs           # Variable debugging utilities
│   │   └── validation.rs          # Variable validation
│   ├── playbook.rs                # Enhanced with precedence
│   ├── inventory.rs               # Enhanced with precedence
│   └── template.rs                # Enhanced with precedence
├── types/
│   ├── variables.rs               # Variable-related types
│   └── parsed.rs                  # Enhanced with variable metadata
└── ...

tests/
├── fixtures/
│   ├── variables/
│   │   ├── precedence/            # Precedence test scenarios
│   │   ├── inheritance/           # Group inheritance tests
│   │   ├── sources/               # Variable source tests
│   │   ├── special/               # Special variable tests
│   │   └── debugging/             # Variable debugging tests
│   └── expected/
│       └── variables/             # Expected resolution results
└── parser/
    ├── precedence_engine_tests.rs # Core precedence tests
    ├── inheritance_tests.rs       # Group inheritance tests
    ├── resolution_tests.rs        # Variable resolution tests
    └── debugging_tests.rs         # Debugging feature tests
```

## Implementation Details

### Phase 1: Core Precedence Engine
```rust
// src/parser/variables/precedence.rs
impl VariablePrecedenceEngine {
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
            host_variables: HashMap::new(),
            group_variables: HashMap::new(),
            variable_sources: HashMap::new(),
            special_variables: HashMap::new(),
        }
    }
    
    pub fn add_variables(
        &mut self,
        precedence: VariablePrecedence,
        variables: HashMap<String, Value>,
        source: VariableSource,
    ) {
        let precedence_vars = self.variables.entry(precedence).or_insert_with(HashMap::new);
        
        for (key, value) in variables {
            let variable_value = VariableValue {
                value: value.clone(),
                source: source.clone(),
                conditional: None,
                precedence,
            };
            
            precedence_vars.insert(key.clone(), variable_value.clone());
            self.variable_sources.insert(
                self.make_variable_key(&key, None, precedence),
                source.clone(),
            );
        }
    }
    
    pub fn resolve_host_variables(
        &self,
        host: &str,
        host_groups: &[String],
    ) -> VariableResolutionResult {
        let mut resolved_variables = HashMap::new();
        let mut variable_sources = HashMap::new();
        let mut resolution_order = Vec::new();
        
        // Process variables in precedence order (lowest to highest)
        for (precedence, vars) in &self.variables {
            for (key, variable_value) in vars {
                resolved_variables.insert(key.clone(), variable_value.value.clone());
                variable_sources.insert(key.clone(), variable_value.source.clone());
                resolution_order.push(format!("{}:{}", precedence as u8, key));
            }
        }
        
        // Process group variables for this host
        for group_name in host_groups {
            if let Some(group_vars) = self.group_variables.get(group_name) {
                for (precedence, vars) in group_vars {
                    for (key, variable_value) in vars {
                        resolved_variables.insert(key.clone(), variable_value.value.clone());
                        variable_sources.insert(key.clone(), variable_value.source.clone());
                        resolution_order.push(format!("{}:{}:{}", precedence as u8, group_name, key));
                    }
                }
            }
        }
        
        // Process host-specific variables
        if let Some(host_vars) = self.host_variables.get(host) {
            for (precedence, vars) in host_vars {
                for (key, variable_value) in vars {
                    resolved_variables.insert(key.clone(), variable_value.value.clone());
                    variable_sources.insert(key.clone(), variable_value.source.clone());
                    resolution_order.push(format!("{}:{}:{}", precedence as u8, host, key));
                }
            }
        }
        
        VariableResolutionResult {
            variables: resolved_variables,
            variable_sources,
            resolution_order,
        }
    }
    
    fn make_variable_key(
        &self,
        var_name: &str,
        context: Option<&str>,
        precedence: VariablePrecedence,
    ) -> String {
        match context {
            Some(ctx) => format!("{}:{}:{}", precedence as u8, ctx, var_name),
            None => format!("{}:global:{}", precedence as u8, var_name),
        }
    }
}
```

### Phase 2: Group Inheritance Engine
```rust
// src/parser/variables/inheritance.rs
impl GroupInheritanceEngine {
    pub fn build_hierarchy(&mut self, inventory: &ParsedInventory) {
        // Build parent-child relationships
        for (group_name, group) in &inventory.groups {
            let mut parents = Vec::new();
            
            // Find parent groups (groups that have this group as a child)
            for (parent_name, parent_group) in &inventory.groups {
                if parent_group.children.contains(group_name) {
                    parents.push(parent_name.clone());
                }
            }
            
            self.group_hierarchy.insert(group_name.clone(), parents);
        }
        
        // Validate for cycles
        self.validate_hierarchy().expect("Group hierarchy contains cycles");
    }
    
    pub fn resolve_group_inheritance(
        &self,
        host_groups: &[String],
    ) -> HashMap<String, VariableValue> {
        let mut resolved_vars = HashMap::new();
        let all_groups = self.get_effective_groups(host_groups);
        
        // Process groups in dependency order (parents before children)
        let ordered_groups = self.topological_sort(&all_groups);
        
        for group_name in ordered_groups {
            if let Some(group_vars) = self.group_variables.get(&group_name) {
                for (precedence, vars) in group_vars {
                    for (key, variable_value) in vars {
                        // Higher precedence groups override lower precedence
                        if let Some(existing) = resolved_vars.get(key) {
                            if variable_value.precedence >= existing.precedence {
                                resolved_vars.insert(key.clone(), variable_value.clone());
                            }
                        } else {
                            resolved_vars.insert(key.clone(), variable_value.clone());
                        }
                    }
                }
            }
        }
        
        resolved_vars
    }
    
    pub fn get_effective_groups(&self, host_groups: &[String]) -> Vec<String> {
        let mut all_groups = HashSet::new();
        let mut to_process = VecDeque::from(host_groups.to_vec());
        
        while let Some(group) = to_process.pop_front() {
            if all_groups.insert(group.clone()) {
                // Add parent groups to processing queue
                if let Some(parents) = self.group_hierarchy.get(&group) {
                    for parent in parents {
                        to_process.push_back(parent.clone());
                    }
                }
            }
        }
        
        all_groups.into_iter().collect()
    }
    
    fn topological_sort(&self, groups: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        
        for group in groups {
            if !visited.contains(group) {
                self.topological_visit(group, &mut visited, &mut temp_visited, &mut result);
            }
        }
        
        result.reverse(); // We want parents before children
        result
    }
    
    fn topological_visit(
        &self,
        group: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        if temp_visited.contains(group) {
            panic!("Circular dependency detected in group hierarchy");
        }
        
        if visited.contains(group) {
            return;
        }
        
        temp_visited.insert(group.to_string());
        
        // Visit parent groups first
        if let Some(parents) = self.group_hierarchy.get(group) {
            for parent in parents {
                self.topological_visit(parent, visited, temp_visited, result);
            }
        }
        
        temp_visited.remove(group);
        visited.insert(group.to_string());
        result.push(group.to_string());
    }
    
    pub fn validate_hierarchy(&self) -> Result<(), VariableError> {
        // Check for circular dependencies using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for group in self.group_hierarchy.keys() {
            if !visited.contains(group) {
                if self.has_cycle(group, &mut visited, &mut rec_stack)? {
                    return Err(VariableError::CircularGroupDependency {
                        group: group.clone(),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    fn has_cycle(
        &self,
        group: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<bool, VariableError> {
        visited.insert(group.to_string());
        rec_stack.insert(group.to_string());
        
        if let Some(parents) = self.group_hierarchy.get(group) {
            for parent in parents {
                if !visited.contains(parent) {
                    if self.has_cycle(parent, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(parent) {
                    return Ok(true);
                }
            }
        }
        
        rec_stack.remove(group);
        Ok(false)
    }
}
```

### Phase 3: Special Variables Implementation
```rust
// src/parser/variables/special.rs
impl VariablePrecedenceEngine {
    pub fn resolve_special_variables(&self, inventory: &ParsedInventory) -> HashMap<String, Value> {
        let mut special_vars = HashMap::new();
        
        // hostvars - variables for all hosts
        let hostvars = self.build_hostvars(inventory);
        special_vars.insert("hostvars".to_string(), Value::Object(hostvars));
        
        // groups - all groups and their hosts
        let groups = self.build_groups_var(inventory);
        special_vars.insert("groups".to_string(), Value::Object(groups));
        
        // group_names - list of all group names
        let group_names: Vec<Value> = inventory.groups.keys()
            .map(|name| Value::String(name.clone()))
            .collect();
        special_vars.insert("group_names".to_string(), Value::Array(group_names));
        
        // play_hosts - hosts in current play (context-dependent)
        // This will be set during play execution
        
        // inventory_hostname - current host (context-dependent)
        // This will be set during host execution
        
        special_vars
    }
    
    fn build_hostvars(&self, inventory: &ParsedInventory) -> serde_json::Map<String, Value> {
        let mut hostvars = serde_json::Map::new();
        
        for (hostname, host) in &inventory.hosts {
            // Resolve all variables for this host
            let host_groups: Vec<String> = host.groups.clone();
            let resolved = self.resolve_host_variables(hostname, &host_groups);
            
            // Convert to JSON object
            let host_vars: serde_json::Map<String, Value> = resolved.variables
                .into_iter()
                .collect();
            
            hostvars.insert(hostname.clone(), Value::Object(host_vars));
        }
        
        hostvars
    }
    
    fn build_groups_var(&self, inventory: &ParsedInventory) -> serde_json::Map<String, Value> {
        let mut groups = serde_json::Map::new();
        
        for (group_name, group) in &inventory.groups {
            let host_list: Vec<Value> = group.hosts.iter()
                .map(|hostname| Value::String(hostname.clone()))
                .collect();
            
            groups.insert(group_name.clone(), Value::Array(host_list));
        }
        
        // Add 'all' group if not present
        if !groups.contains_key("all") {
            let all_hosts: Vec<Value> = inventory.hosts.keys()
                .map(|hostname| Value::String(hostname.clone()))
                .collect();
            groups.insert("all".to_string(), Value::Array(all_hosts));
        }
        
        // Add 'ungrouped' for hosts not in any explicit group
        let mut ungrouped_hosts = Vec::new();
        for (hostname, host) in &inventory.hosts {
            if host.groups.is_empty() || host.groups == vec!["all"] {
                ungrouped_hosts.push(Value::String(hostname.clone()));
            }
        }
        if !ungrouped_hosts.is_empty() {
            groups.insert("ungrouped".to_string(), Value::Array(ungrouped_hosts));
        }
        
        groups
    }
}
```

### Phase 4: Variable Debugging System
```rust
// src/parser/variables/debugging.rs
impl VariablePrecedenceEngine {
    pub fn debug_variable_precedence(
        &self,
        variable_name: &str,
        host: &str,
        host_groups: &[String],
    ) -> VariableDebugInfo {
        let mut all_definitions = Vec::new();
        let mut resolution_path = Vec::new();
        
        // Collect all definitions of this variable
        self.collect_variable_definitions(variable_name, host, host_groups, &mut all_definitions);
        
        // Sort by precedence
        all_definitions.sort_by_key(|def| def.precedence);
        
        // Build resolution path
        self.build_resolution_path(variable_name, &all_definitions, &mut resolution_path);
        
        // Determine final value and source
        let (final_value, final_source) = all_definitions
            .iter()
            .rev() // Highest precedence first
            .find(|def| def.active)
            .map(|def| (Some(def.value.clone()), Some(def.source.clone())))
            .unwrap_or((None, None));
        
        VariableDebugInfo {
            variable_name: variable_name.to_string(),
            final_value,
            final_source,
            all_definitions,
            resolution_path,
        }
    }
    
    fn collect_variable_definitions(
        &self,
        variable_name: &str,
        host: &str,
        host_groups: &[String],
        definitions: &mut Vec<VariableDefinition>,
    ) {
        // Global variables
        for (precedence, vars) in &self.variables {
            if let Some(variable_value) = vars.get(variable_name) {
                definitions.push(VariableDefinition {
                    value: variable_value.value.clone(),
                    source: variable_value.source.clone(),
                    precedence: *precedence,
                    active: true,
                });
            }
        }
        
        // Group variables
        for group_name in host_groups {
            if let Some(group_vars) = self.group_variables.get(group_name) {
                for (precedence, vars) in group_vars {
                    if let Some(variable_value) = vars.get(variable_name) {
                        definitions.push(VariableDefinition {
                            value: variable_value.value.clone(),
                            source: variable_value.source.clone(),
                            precedence: *precedence,
                            active: true,
                        });
                    }
                }
            }
        }
        
        // Host variables
        if let Some(host_vars) = self.host_variables.get(host) {
            for (precedence, vars) in host_vars {
                if let Some(variable_value) = vars.get(variable_name) {
                    definitions.push(VariableDefinition {
                        value: variable_value.value.clone(),
                        source: variable_value.source.clone(),
                        precedence: *precedence,
                        active: true,
                    });
                }
            }
        }
    }
    
    fn build_resolution_path(
        &self,
        variable_name: &str,
        definitions: &[VariableDefinition],
        path: &mut Vec<ResolutionStep>,
    ) {
        let mut current_precedence = None;
        let mut vars_at_level = 0;
        let mut overrides_at_level = 0;
        
        for definition in definitions {
            if current_precedence != Some(definition.precedence) {
                // Finalize previous level
                if let Some(prev_precedence) = current_precedence {
                    path.push(ResolutionStep {
                        step: format!("Processed {} at precedence level {}", 
                                     variable_name, prev_precedence as u8),
                        variables_added: vars_at_level,
                        variables_overridden: overrides_at_level,
                        precedence_level: prev_precedence,
                    });
                }
                
                // Start new level
                current_precedence = Some(definition.precedence);
                vars_at_level = 1;
                overrides_at_level = if vars_at_level > 1 { 1 } else { 0 };
            } else {
                vars_at_level += 1;
                overrides_at_level += 1;
            }
        }
        
        // Finalize last level
        if let Some(precedence) = current_precedence {
            path.push(ResolutionStep {
                step: format!("Final resolution for {} at precedence level {}", 
                             variable_name, precedence as u8),
                variables_added: vars_at_level,
                variables_overridden: overrides_at_level,
                precedence_level: precedence,
            });
        }
    }
}
```

## Testing Strategy

### Unit Testing Requirements
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_precedence_order() {
        let mut engine = VariablePrecedenceEngine::new();
        
        // Add variables at different precedence levels
        let mut low_vars = HashMap::new();
        low_vars.insert("test_var".to_string(), Value::String("low".to_string()));
        engine.add_variables(
            VariablePrecedence::RoleDefaults,
            low_vars,
            VariableSource {
                source_type: VariableSourceType::RoleDefaults,
                file_path: Some("roles/test/defaults/main.yml".to_string()),
                line_number: Some(1),
                role_name: Some("test".to_string()),
                host_name: None,
                group_name: None,
            },
        );
        
        let mut high_vars = HashMap::new();
        high_vars.insert("test_var".to_string(), Value::String("high".to_string()));
        engine.add_variables(
            VariablePrecedence::ExtraVars,
            high_vars,
            VariableSource {
                source_type: VariableSourceType::ExtraVars,
                file_path: None,
                line_number: None,
                role_name: None,
                host_name: None,
                group_name: None,
            },
        );
        
        let result = engine.resolve_host_variables("test_host", &[]);
        
        assert_eq!(
            result.variables.get("test_var").unwrap().as_str().unwrap(),
            "high"
        );
    }
    
    #[test]
    fn test_group_inheritance() {
        let mut inheritance_engine = GroupInheritanceEngine::new();
        
        // Create test inventory with group hierarchy
        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: HashMap::new(),
        };
        
        // Create groups: all -> production -> webservers
        inventory.groups.insert("all".to_string(), ParsedGroup {
            name: "all".to_string(),
            hosts: vec!["web1".to_string()],
            children: vec!["production".to_string()],
            vars: {
                let mut vars = HashMap::new();
                vars.insert("env".to_string(), Value::String("default".to_string()));
                vars
            },
        });
        
        inventory.groups.insert("production".to_string(), ParsedGroup {
            name: "production".to_string(),
            hosts: vec!["web1".to_string()],
            children: vec!["webservers".to_string()],
            vars: {
                let mut vars = HashMap::new();
                vars.insert("env".to_string(), Value::String("production".to_string()));
                vars.insert("deploy_user".to_string(), Value::String("deploy".to_string()));
                vars
            },
        });
        
        inventory.groups.insert("webservers".to_string(), ParsedGroup {
            name: "webservers".to_string(),
            hosts: vec!["web1".to_string()],
            children: vec![],
            vars: {
                let mut vars = HashMap::new();
                vars.insert("http_port".to_string(), Value::Number(serde_json::Number::from(80)));
                vars
            },
        });
        
        inheritance_engine.build_hierarchy(&inventory);
        
        let effective_groups = inheritance_engine.get_effective_groups(&["webservers".to_string()]);
        
        assert!(effective_groups.contains(&"all".to_string()));
        assert!(effective_groups.contains(&"production".to_string()));
        assert!(effective_groups.contains(&"webservers".to_string()));
    }
    
    #[test]
    fn test_variable_debugging() {
        let mut engine = VariablePrecedenceEngine::new();
        
        // Add the same variable at multiple precedence levels
        let precedence_levels = [
            (VariablePrecedence::RoleDefaults, "role_default"),
            (VariablePrecedence::GroupVars, "group_vars"),
            (VariablePrecedence::HostVars, "host_vars"),
            (VariablePrecedence::TaskVars, "task_vars"),
            (VariablePrecedence::ExtraVars, "extra_vars"),
        ];
        
        for (precedence, value) in &precedence_levels {
            let mut vars = HashMap::new();
            vars.insert("debug_var".to_string(), Value::String(value.to_string()));
            
            engine.add_variables(
                *precedence,
                vars,
                VariableSource {
                    source_type: match precedence {
                        VariablePrecedence::RoleDefaults => VariableSourceType::RoleDefaults,
                        VariablePrecedence::GroupVars => VariableSourceType::GroupVars,
                        VariablePrecedence::HostVars => VariableSourceType::HostVars,
                        VariablePrecedence::TaskVars => VariableSourceType::TaskVars,
                        VariablePrecedence::ExtraVars => VariableSourceType::ExtraVars,
                        _ => VariableSourceType::RoleDefaults,
                    },
                    file_path: Some(format!("test_{}.yml", value)),
                    line_number: Some(1),
                    role_name: None,
                    host_name: None,
                    group_name: None,
                },
            );
        }
        
        let debug_info = engine.debug_variable_precedence(
            "debug_var",
            "test_host",
            &["test_group".to_string()],
        );
        
        assert_eq!(debug_info.all_definitions.len(), 5);
        assert_eq!(
            debug_info.final_value.unwrap().as_str().unwrap(),
            "extra_vars"
        );
        assert_eq!(debug_info.final_source.unwrap().source_type, VariableSourceType::ExtraVars);
    }
    
    #[test]
    fn test_special_variables() {
        let mut engine = VariablePrecedenceEngine::new();
        
        // Create test inventory
        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: HashMap::new(),
        };
        
        // Add hosts
        inventory.hosts.insert("web1".to_string(), ParsedHost {
            name: "web1".to_string(),
            address: Some("192.168.1.10".to_string()),
            port: None,
            user: None,
            vars: {
                let mut vars = HashMap::new();
                vars.insert("role".to_string(), Value::String("webserver".to_string()));
                vars
            },
            groups: vec!["webservers".to_string()],
        });
        
        inventory.hosts.insert("db1".to_string(), ParsedHost {
            name: "db1".to_string(),
            address: Some("192.168.1.20".to_string()),
            port: None,
            user: None,
            vars: {
                let mut vars = HashMap::new();
                vars.insert("role".to_string(), Value::String("database".to_string()));
                vars
            },
            groups: vec!["databases".to_string()],
        });
        
        // Add groups
        inventory.groups.insert("webservers".to_string(), ParsedGroup {
            name: "webservers".to_string(),
            hosts: vec!["web1".to_string()],
            children: vec![],
            vars: HashMap::new(),
        });
        
        inventory.groups.insert("databases".to_string(), ParsedGroup {
            name: "databases".to_string(),
            hosts: vec!["db1".to_string()],
            children: vec![],
            vars: HashMap::new(),
        });
        
        let special_vars = engine.resolve_special_variables(&inventory);
        
        // Test hostvars
        let hostvars = special_vars.get("hostvars").unwrap().as_object().unwrap();
        assert!(hostvars.contains_key("web1"));
        assert!(hostvars.contains_key("db1"));
        
        // Test groups
        let groups = special_vars.get("groups").unwrap().as_object().unwrap();
        assert!(groups.contains_key("webservers"));
        assert!(groups.contains_key("databases"));
        
        let webservers_hosts = groups.get("webservers").unwrap().as_array().unwrap();
        assert_eq!(webservers_hosts.len(), 1);
        assert_eq!(webservers_hosts[0].as_str().unwrap(), "web1");
        
        // Test group_names
        let group_names = special_vars.get("group_names").unwrap().as_array().unwrap();
        assert!(group_names.iter().any(|v| v.as_str() == Some("webservers")));
        assert!(group_names.iter().any(|v| v.as_str() == Some("databases")));
    }
}
```

### Integration Testing
```rust
// tests/parser/precedence_integration_tests.rs
#[tokio::test]
async fn test_full_precedence_scenario() {
    let temp_dir = setup_complex_precedence_structure().await;
    
    let mut parser = Parser::new();
    parser = parser.with_extra_vars({
        let mut extra = HashMap::new();
        extra.insert("override_var".to_string(), serde_json::Value::String("from_extra".to_string()));
        extra
    });
    
    let playbook = parser.parse_playbook(&temp_dir.path().join("site.yml")).await.unwrap();
    let inventory = parser.parse_inventory(&temp_dir.path().join("inventory.yml")).await.unwrap();
    
    // Test variable resolution for specific host
    let precedence_engine = VariablePrecedenceEngine::new();
    // ... populate engine with all variables from parsing ...
    
    let web1_vars = precedence_engine.resolve_host_variables(
        "web1",
        &["webservers".to_string(), "production".to_string()],
    );
    
    // Verify precedence is respected
    assert_eq!(web1_vars.variables.get("override_var").unwrap().as_str().unwrap(), "from_extra");
    assert_eq!(web1_vars.variables.get("env").unwrap().as_str().unwrap(), "production");
    
    // Verify variable sources are tracked
    let override_source = web1_vars.variable_sources.get("override_var").unwrap();
    assert_eq!(override_source.source_type, VariableSourceType::ExtraVars);
}

async fn setup_complex_precedence_structure() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    
    // Create complex directory structure with variables at all levels
    create_role_with_defaults(&temp_dir, "webserver").await;
    create_group_vars(&temp_dir).await;
    create_host_vars(&temp_dir).await;
    create_playbook_with_vars(&temp_dir).await;
    create_inventory_with_vars(&temp_dir).await;
    
    temp_dir
}
```

## Edge Cases & Error Handling

### Variable-Specific Error Types
```rust
#[derive(Debug, Error)]
pub enum VariableError {
    #[error("Circular group dependency detected starting from group '{group}'")]
    CircularGroupDependency { group: String },
    
    #[error("Variable '{variable}' has conflicting definitions")]
    ConflictingVariableDefinitions { variable: String },
    
    #[error("Invalid variable precedence level: {level}")]
    InvalidPrecedenceLevel { level: u8 },
    
    #[error("Variable resolution failed for host '{host}': {message}")]
    ResolutionFailed { host: String, message: String },
    
    #[error("Special variable '{variable}' cannot be overridden")]
    SpecialVariableOverride { variable: String },
}
```

### Edge Case Handling
```rust
impl VariablePrecedenceEngine {
    /// Handle undefined variable references
    pub fn handle_undefined_variable(
        &self,
        variable_name: &str,
        context: &str,
    ) -> Result<Value, VariableError> {
        // Check if it's a special variable that should be defined
        if self.is_special_variable(variable_name) {
            return Ok(self.get_special_variable_default(variable_name));
        }
        
        // Return undefined or error based on configuration
        Err(VariableError::ResolutionFailed {
            host: context.to_string(),
            message: format!("Variable '{}' is not defined", variable_name),
        })
    }
    
    /// Handle variable type conflicts
    pub fn validate_variable_types(&self) -> Result<(), VariableError> {
        // Check for variables that change type across precedence levels
        // This could indicate configuration errors
        
        for (var_name, definitions) in self.get_all_variable_definitions() {
            let mut seen_types = HashSet::new();
            
            for definition in definitions {
                let value_type = self.get_value_type(&definition.value);
                seen_types.insert(value_type);
            }
            
            if seen_types.len() > 1 {
                // Multiple types for same variable - potential issue
                tracing::warn!(
                    "Variable '{}' has multiple types across definitions: {:?}",
                    var_name,
                    seen_types
                );
            }
        }
        
        Ok(())
    }
    
    fn is_special_variable(&self, name: &str) -> bool {
        matches!(name, 
            "hostvars" | "groups" | "group_names" | "inventory_hostname" |
            "play_hosts" | "ansible_version" | "ansible_facts"
        )
    }
}
```

## Dependencies

### No New Major Dependencies
The variable precedence engine can be implemented using existing dependencies:
- `std::collections::HashMap` and `BTreeMap` for variable storage
- `serde_json::Value` for variable values
- Existing error handling infrastructure

## Performance Considerations

### Variable Resolution Optimization
```rust
impl VariablePrecedenceEngine {
    /// Cache resolved variables for performance
    fn cache_resolution(&mut self, cache_key: &str, result: &VariableResolutionResult) {
        if self.resolution_cache.len() >= self.max_cache_size {
            self.resolution_cache.clear(); // Simple cache eviction
        }
        self.resolution_cache.insert(cache_key.to_string(), result.clone());
    }
    
    /// Optimized variable lookup for common patterns
    pub fn fast_variable_lookup(
        &self,
        variable_name: &str,
        host: &str,
    ) -> Option<&VariableValue> {
        // Fast path for simple variable lookups
        
        // Check host variables first (common case)
        if let Some(host_vars) = self.host_variables.get(host) {
            if let Some(precedence_vars) = host_vars.last_key_value() {
                if let Some(var) = precedence_vars.1.get(variable_name) {
                    return Some(var);
                }
            }
        }
        
        // Check global variables
        if let Some(precedence_vars) = self.variables.last_key_value() {
            if let Some(var) = precedence_vars.1.get(variable_name) {
                return Some(var);
            }
        }
        
        None
    }
}
```

### Memory Optimization
- Use string interning for variable names
- Implement copy-on-write for large variable sets
- Use efficient data structures for precedence ordering

## Implementation Phases

### Phase 1: Core Precedence System (Week 1-2)
- [ ] Implement basic precedence enumeration and ordering
- [ ] Create variable storage with precedence levels
- [ ] Basic variable resolution algorithm
- [ ] Simple precedence testing

### Phase 2: Group Inheritance (Week 3)
- [ ] Implement group hierarchy building
- [ ] Add group variable inheritance logic
- [ ] Circular dependency detection
- [ ] Group inheritance testing

### Phase 3: Special Variables (Week 4)
- [ ] Implement hostvars, groups, group_names
- [ ] Add inventory_hostname and play_hosts context
- [ ] Special variable resolution
- [ ] Integration with template engine

### Phase 4: Debugging and Optimization (Week 5)
- [ ] Variable debugging and source tracking
- [ ] Performance optimization and caching
- [ ] Comprehensive integration testing
- [ ] Documentation and examples

## Success Metrics

### Functional Metrics
- All 22 precedence levels implemented correctly
- Group inheritance matches Ansible behavior exactly
- Special variables work in all contexts
- Variable debugging provides accurate information

### Performance Metrics
- Variable resolution <10ms for typical scenarios
- Group inheritance processing <5ms
- Memory usage scales linearly with variable count
- Resolution caching improves performance >50%

### Compatibility Metrics
- 100% compatibility with Ansible precedence rules
- All Ansible variable test cases pass
- Real-world variable scenarios work correctly
- Variable debugging matches Ansible's ansible-inventory output