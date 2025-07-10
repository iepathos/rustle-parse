# Spec 030: Complete INI Inventory Parsing

## Feature Summary

Implement comprehensive INI format inventory parsing to fully support Ansible's INI inventory syntax including host patterns, group variables, group children, and connection parameters. This replaces the current stub implementation with a complete parser that handles all INI inventory features.

**Problem it solves**: The current INI inventory parser is a placeholder that only returns localhost. Real Ansible projects use complex INI inventories with host patterns, group hierarchies, and variable inheritance that need full parsing support.

**High-level approach**: Implement a robust INI parser using the configparser crate to handle all Ansible INI inventory features including host pattern expansion, group variable inheritance, and connection parameter parsing.

## Goals & Requirements

### Functional Requirements
- Parse complete INI inventory files with all Ansible-compatible syntax
- Support host pattern expansion (web[01:05], db-[a:c])
- Handle group variables and group children sections
- Parse connection parameters (ansible_host, ansible_port, ansible_user, etc.)
- Support inline host variables in inventory entries
- Implement proper variable inheritance from groups to hosts
- Handle special groups (all, ungrouped)
- Support inventory file includes and group_vars/host_vars directories

### Non-functional Requirements
- **Performance**: Parse large inventories (>1000 hosts) in <1 second
- **Memory**: Keep memory usage proportional to inventory size
- **Compatibility**: 100% compatible with Ansible INI inventory syntax
- **Error Handling**: Clear error messages with line numbers for syntax errors
- **Validation**: Comprehensive validation of inventory structure

### Success Criteria
- All Ansible INI inventory features are supported
- Host pattern expansion works correctly for all patterns
- Variable inheritance follows Ansible precedence rules
- Integration tests pass with real-world inventory files
- Performance meets requirements for large inventories

## API/Interface Design

### Enhanced InventoryParser Methods
```rust
impl<'a> InventoryParser<'a> {
    /// Parse INI inventory with complete feature support
    pub async fn parse_ini_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError>;
    
    /// Parse host patterns like web[01:05] into individual hosts
    pub fn expand_host_pattern(&self, pattern: &str) -> Result<Vec<String>, ParseError>;
    
    /// Parse inline host variables from inventory line
    pub fn parse_host_variables(&self, vars_str: &str) -> HashMap<String, serde_json::Value>;
    
    /// Resolve group inheritance and variable precedence
    pub fn resolve_group_inheritance(&self, inventory: &mut ParsedInventory) -> Result<(), ParseError>;
    
    /// Validate inventory structure and relationships
    pub fn validate_inventory(&self, inventory: &ParsedInventory) -> Result<(), ParseError>;
}
```

### INI Parsing Structures
```rust
/// Internal structure for parsing INI sections
#[derive(Debug)]
struct IniSection {
    name: String,
    section_type: SectionType,
    entries: Vec<IniEntry>,
}

#[derive(Debug)]
enum SectionType {
    Hosts,           // [groupname]
    GroupVars,       // [groupname:vars]
    GroupChildren,   // [groupname:children]
}

#[derive(Debug)]
struct IniEntry {
    key: String,
    value: Option<String>,
    variables: HashMap<String, String>,
}

/// Host pattern expansion utilities
pub struct HostPattern {
    pub pattern: String,
    pub expanded: Vec<String>,
}

impl HostPattern {
    pub fn new(pattern: &str) -> Result<Self, ParseError>;
    pub fn expand(&self) -> Result<Vec<String>, ParseError>;
    pub fn is_pattern(&self) -> bool;
}
```

### Error Types
```rust
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid host pattern '{pattern}' at line {line}: {message}")]
    InvalidHostPattern {
        pattern: String,
        line: usize,
        message: String,
    },
    
    #[error("Circular group dependency: {cycle}")]
    CircularGroupDependency { cycle: String },
    
    #[error("Invalid variable syntax at line {line}: {message}")]
    InvalidVariableSyntax { line: usize, message: String },
    
    #[error("Duplicate host '{host}' in inventory")]
    DuplicateHost { host: String },
    
    #[error("Unknown group '{group}' referenced in children")]
    UnknownGroup { group: String },
}
```

## File and Package Structure

### Enhanced Files
```
src/
├── parser/
│   ├── inventory/
│   │   ├── mod.rs                 # Re-export inventory parsing
│   │   ├── ini.rs                 # Complete INI parsing implementation
│   │   ├── patterns.rs            # Host pattern expansion
│   │   ├── variables.rs           # Variable parsing and inheritance
│   │   └── validation.rs          # Inventory validation
│   └── inventory.rs               # Main inventory parser (delegate to format-specific parsers)
├── types/
│   └── inventory.rs               # Enhanced inventory types
└── ...

tests/
├── fixtures/
│   ├── inventories/
│   │   ├── complex.ini            # Complex real-world inventory
│   │   ├── patterns.ini           # Host pattern examples
│   │   ├── inheritance.ini        # Variable inheritance test
│   │   └── edge_cases.ini         # Edge cases and error conditions
│   └── expected/
│       └── inventories/           # Expected parsing results
└── parser/
    ├── inventory_ini_tests.rs     # Comprehensive INI parsing tests
    └── pattern_expansion_tests.rs # Host pattern expansion tests
```

### Integration with Existing Code
- Enhance `src/parser/inventory.rs` to delegate INI parsing to new implementation
- Extend `ParsedInventory` and related types as needed
- Maintain backward compatibility with existing inventory parsing interface

## Implementation Details

### Phase 1: Host Pattern Expansion
```rust
// src/parser/inventory/patterns.rs
use regex::Regex;

impl HostPattern {
    pub fn expand(&self) -> Result<Vec<String>, ParseError> {
        if !self.is_pattern() {
            return Ok(vec![self.pattern.clone()]);
        }
        
        let mut hosts = Vec::new();
        
        // Handle numeric ranges: web[01:05]
        if let Some(captures) = NUMERIC_PATTERN.captures(&self.pattern) {
            let prefix = &captures[1];
            let start: i32 = captures[2].parse()?;
            let end: i32 = captures[3].parse()?;
            let suffix = captures.get(4).map_or("", |m| m.as_str());
            
            for i in start..=end {
                let formatted = if captures[2].starts_with('0') {
                    format!("{}{:0width$}{}", prefix, i, suffix, width = captures[2].len())
                } else {
                    format!("{}{}{}", prefix, i, suffix)
                };
                hosts.push(formatted);
            }
        }
        
        // Handle alphabetic ranges: db-[a:c]
        else if let Some(captures) = ALPHA_PATTERN.captures(&self.pattern) {
            let prefix = &captures[1];
            let start_char = captures[2].chars().next().unwrap();
            let end_char = captures[3].chars().next().unwrap();
            let suffix = captures.get(4).map_or("", |m| m.as_str());
            
            for c in start_char..=end_char {
                hosts.push(format!("{}{}{}", prefix, c, suffix));
            }
        }
        
        Ok(hosts)
    }
}

static NUMERIC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(.+)\[(\d+):(\d+)\](.*)$").unwrap()
});

static ALPHA_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(.+)\[([a-z]):([a-z])\](.*)$").unwrap()
});
```

### Phase 2: INI Section Parsing
```rust
// src/parser/inventory/ini.rs
use configparser::ini::Ini;

impl<'a> InventoryParser<'a> {
    pub async fn parse_ini_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError> {
        let mut config = Ini::new();
        config.read(content.to_string())
            .map_err(|e| ParseError::IniParsing { 
                message: format!("Failed to parse INI: {}", e) 
            })?;
        
        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: self.extra_vars.clone(),
        };
        
        // Parse all sections
        for section_name in config.sections() {
            self.parse_ini_section(&mut inventory, &config, &section_name)?;
        }
        
        // Resolve group inheritance and variable precedence
        self.resolve_group_inheritance(&mut inventory)?;
        
        // Validate final inventory structure
        self.validate_inventory(&inventory)?;
        
        Ok(inventory)
    }
    
    fn parse_ini_section(
        &self,
        inventory: &mut ParsedInventory,
        config: &Ini,
        section_name: &str,
    ) -> Result<(), ParseError> {
        let section_type = self.determine_section_type(section_name);
        
        match section_type {
            SectionType::Hosts => {
                self.parse_hosts_section(inventory, config, section_name)?;
            }
            SectionType::GroupVars => {
                self.parse_group_vars_section(inventory, config, section_name)?;
            }
            SectionType::GroupChildren => {
                self.parse_group_children_section(inventory, config, section_name)?;
            }
        }
        
        Ok(())
    }
}
```

### Phase 3: Variable Inheritance Resolution
```rust
// src/parser/inventory/variables.rs
impl<'a> InventoryParser<'a> {
    pub fn resolve_group_inheritance(&self, inventory: &mut ParsedInventory) -> Result<(), ParseError> {
        // Build group dependency graph
        let mut graph = petgraph::Graph::new();
        let mut group_indices = HashMap::new();
        
        // Add all groups as nodes
        for group_name in inventory.groups.keys() {
            let index = graph.add_node(group_name.clone());
            group_indices.insert(group_name.clone(), index);
        }
        
        // Add edges for child relationships
        for (group_name, group) in &inventory.groups {
            let group_index = group_indices[group_name];
            for child_name in &group.children {
                if let Some(&child_index) = group_indices.get(child_name) {
                    graph.add_edge(group_index, child_index, ());
                }
            }
        }
        
        // Check for circular dependencies
        if petgraph::algo::is_cyclic_directed(&graph) {
            return Err(ParseError::CircularGroupDependency { 
                cycle: self.find_cycle(&graph)? 
            });
        }
        
        // Resolve variables in topological order
        let topo_order = petgraph::algo::toposort(&graph, None)
            .map_err(|_| ParseError::CircularGroupDependency { 
                cycle: "Complex cycle detected".to_string() 
            })?;
        
        // Apply group variables to hosts in dependency order
        for node_index in topo_order {
            let group_name = &graph[node_index];
            self.apply_group_variables_to_hosts(inventory, group_name)?;
        }
        
        Ok(())
    }
    
    fn apply_group_variables_to_hosts(
        &self,
        inventory: &mut ParsedInventory,
        group_name: &str,
    ) -> Result<(), ParseError> {
        let group = inventory.groups.get(group_name).cloned();
        if let Some(group) = group {
            for host_name in &group.hosts {
                if let Some(host) = inventory.hosts.get_mut(host_name) {
                    // Group variables have lower precedence than host variables
                    for (key, value) in &group.vars {
                        host.vars.entry(key.clone()).or_insert_with(|| value.clone());
                    }
                }
            }
        }
        Ok(())
    }
}
```

### Phase 4: Connection Parameter Processing
```rust
impl<'a> InventoryParser<'a> {
    fn extract_connection_info(&self, vars: &HashMap<String, serde_json::Value>) 
        -> (Option<String>, Option<u16>, Option<String>) {
        
        let address = vars.get("ansible_host")
            .or_else(|| vars.get("ansible_ssh_host"))
            .and_then(|v| v.as_str())
            .map(String::from);
            
        let port = vars.get("ansible_port")
            .or_else(|| vars.get("ansible_ssh_port"))
            .and_then(|v| v.as_u64())
            .map(|p| p as u16);
            
        let user = vars.get("ansible_user")
            .or_else(|| vars.get("ansible_ssh_user"))
            .or_else(|| vars.get("ansible_ssh_user_name"))
            .and_then(|v| v.as_str())
            .map(String::from);
            
        (address, port, user)
    }
    
    fn parse_host_variables(&self, vars_str: &str) -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        
        // Parse shell-style key=value pairs with proper quoting
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut in_quotes = false;
        let mut in_key = true;
        let mut escape_next = false;
        
        for ch in vars_str.chars() {
            if escape_next {
                if in_key {
                    current_key.push(ch);
                } else {
                    current_value.push(ch);
                }
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' => escape_next = true,
                '"' | '\'' => in_quotes = !in_quotes,
                '=' if !in_quotes && in_key => in_key = false,
                ' ' | '\t' if !in_quotes => {
                    if !in_key && !current_key.is_empty() {
                        // End of key=value pair
                        let parsed_value = self.parse_ini_value(&current_value);
                        vars.insert(current_key.clone(), parsed_value);
                        current_key.clear();
                        current_value.clear();
                        in_key = true;
                    }
                }
                _ => {
                    if in_key {
                        current_key.push(ch);
                    } else {
                        current_value.push(ch);
                    }
                }
            }
        }
        
        // Handle last pair
        if !current_key.is_empty() {
            let parsed_value = self.parse_ini_value(&current_value);
            vars.insert(current_key, parsed_value);
        }
        
        vars
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
    fn test_host_pattern_expansion() {
        let pattern = HostPattern::new("web[01:03]").unwrap();
        let expanded = pattern.expand().unwrap();
        assert_eq!(expanded, vec!["web01", "web02", "web03"]);
        
        let alpha_pattern = HostPattern::new("db-[a:c]").unwrap();
        let alpha_expanded = alpha_pattern.expand().unwrap();
        assert_eq!(alpha_expanded, vec!["db-a", "db-b", "db-c"]);
    }
    
    #[test]
    fn test_variable_parsing() {
        let parser = InventoryParser::new(&TemplateEngine::new(), &HashMap::new());
        let vars = parser.parse_host_variables("ansible_host=192.168.1.10 ansible_port=22 custom_var='hello world'");
        
        assert_eq!(vars.get("ansible_host").unwrap().as_str().unwrap(), "192.168.1.10");
        assert_eq!(vars.get("ansible_port").unwrap().as_u64().unwrap(), 22);
        assert_eq!(vars.get("custom_var").unwrap().as_str().unwrap(), "hello world");
    }
    
    #[test]
    fn test_group_inheritance() {
        let ini_content = r#"
        [webservers]
        web1
        web2
        
        [databases]
        db1
        
        [production:children]
        webservers
        databases
        
        [production:vars]
        env=production
        
        [webservers:vars]
        http_port=80
        "#;
        
        let parser = InventoryParser::new(&TemplateEngine::new(), &HashMap::new());
        let inventory = parser.parse_ini_inventory(ini_content).await.unwrap();
        
        // Check that web1 inherited both production and webservers vars
        let web1 = inventory.hosts.get("web1").unwrap();
        assert_eq!(web1.vars.get("env").unwrap().as_str().unwrap(), "production");
        assert_eq!(web1.vars.get("http_port").unwrap().as_u64().unwrap(), 80);
    }
}
```

### Integration Testing Requirements
```rust
// tests/parser/inventory_ini_tests.rs
#[tokio::test]
async fn test_complex_inventory_parsing() {
    let complex_ini = include_str!("../fixtures/inventories/complex.ini");
    let parser = InventoryParser::new(&TemplateEngine::new(), &HashMap::new());
    
    let inventory = parser.parse_ini_inventory(complex_ini).await.unwrap();
    
    // Verify all expected hosts are present
    assert!(inventory.hosts.contains_key("web01"));
    assert!(inventory.hosts.contains_key("web02"));
    assert!(inventory.hosts.contains_key("db-a"));
    
    // Verify group structure
    assert!(inventory.groups.contains_key("webservers"));
    assert!(inventory.groups.contains_key("production"));
    
    // Verify variable inheritance
    let web01 = inventory.hosts.get("web01").unwrap();
    assert!(web01.vars.contains_key("env"));
}

#[tokio::test]
async fn test_error_handling() {
    let invalid_ini = r#"
    [webservers
    web1
    "#;
    
    let parser = InventoryParser::new(&TemplateEngine::new(), &HashMap::new());
    let result = parser.parse_ini_inventory(invalid_ini).await;
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ParseError::IniParsing { .. }));
}
```

## Edge Cases & Error Handling

### Critical Edge Cases
1. **Invalid Host Patterns**
   - Malformed ranges: `web[01:abc]`, `web[05:01]`
   - Invalid characters in patterns
   - Nested patterns: `web[01:05][a:c]`

2. **Circular Group Dependencies**
   - Direct cycles: A → B → A
   - Indirect cycles: A → B → C → A
   - Self-references: A → A

3. **Variable Conflicts**
   - Same variable defined in multiple groups
   - Host variables vs group variables
   - Reserved variable names

4. **Malformed INI Syntax**
   - Unclosed sections
   - Invalid section names
   - Malformed variable assignments

### Error Recovery Strategies
```rust
impl<'a> InventoryParser<'a> {
    fn handle_parsing_error(&self, error: &configparser::ini::Error, line: usize) -> ParseError {
        match error {
            configparser::ini::Error::InvalidLine => {
                ParseError::InvalidVariableSyntax {
                    line,
                    message: "Invalid INI line format".to_string(),
                }
            }
            _ => ParseError::IniParsing {
                message: format!("INI parsing error at line {}: {}", line, error),
            }
        }
    }
    
    fn validate_host_pattern(&self, pattern: &str) -> Result<(), ParseError> {
        // Validate pattern syntax before attempting expansion
        if pattern.matches('[').count() != pattern.matches(']').count() {
            return Err(ParseError::InvalidHostPattern {
                pattern: pattern.to_string(),
                line: 0,
                message: "Unmatched brackets in host pattern".to_string(),
            });
        }
        
        // Additional validation logic...
        Ok(())
    }
}
```

## Dependencies

### New Dependencies
```toml
[dependencies]
# INI parsing (already present)
configparser = "3.1"

# Graph algorithms for dependency resolution (already present)
petgraph = "0.8"

# Regular expressions for pattern matching (already present)
regex = "1.11"

# Lazy static for compiled regexes
once_cell = "1.21"
```

### Internal Dependencies
- Enhanced error types in `src/parser/error.rs`
- Integration with existing template engine
- Use of existing `ParsedInventory` and related types

## Configuration

### Parser Configuration Options
```rust
pub struct InventoryParserConfig {
    pub strict_mode: bool,              // Fail on warnings
    pub expand_patterns: bool,          // Enable host pattern expansion
    pub max_pattern_expansion: usize,   // Limit pattern expansion size
    pub validate_hosts: bool,           // Validate host connectivity
    pub resolve_dns: bool,              // Resolve hostnames to IPs
}

impl Default for InventoryParserConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            expand_patterns: true,
            max_pattern_expansion: 1000,
            validate_hosts: false,
            resolve_dns: false,
        }
    }
}
```

## Documentation

### Enhanced Documentation Requirements
```rust
/// Complete INI inventory parser with full Ansible compatibility.
/// 
/// This parser handles all Ansible INI inventory features including:
/// - Host pattern expansion (web[01:05], db-[a:c])
/// - Group variables and children
/// - Variable inheritance and precedence
/// - Connection parameter parsing
/// 
/// # Examples
/// 
/// ```rust
/// use rustle_parse::parser::InventoryParser;
/// 
/// let ini_content = r#"
/// [webservers]
/// web[01:03] ansible_user=deploy
/// 
/// [webservers:vars]
/// http_port=80
/// "#;
/// 
/// let parser = InventoryParser::new(&template_engine, &extra_vars);
/// let inventory = parser.parse_ini_inventory(ini_content).await?;
/// 
/// assert_eq!(inventory.hosts.len(), 3); // web01, web02, web03
/// ```
pub async fn parse_ini_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError>;
```

## Performance Considerations

### Optimization Strategies
- Use compiled regexes for pattern matching
- Implement efficient graph algorithms for dependency resolution
- Cache parsed patterns for reuse
- Stream processing for very large inventories
- Parallel pattern expansion for large ranges

### Memory Management
```rust
// Use owned strings only when necessary
struct HostEntry<'a> {
    name: &'a str,
    variables: HashMap<&'a str, &'a str>,
}

// Implement efficient pattern expansion
impl HostPattern {
    pub fn expand_lazy(&self) -> impl Iterator<Item = String> + '_ {
        // Lazy iterator to avoid allocating all hosts at once
        (self.start..=self.end).map(move |i| {
            format!("{}{:0width$}{}", self.prefix, i, self.suffix, width = self.zero_pad)
        })
    }
}
```

## Security Considerations

### Input Validation
- Sanitize all user input from INI files
- Validate host patterns to prevent ReDoS attacks
- Limit recursion depth in group inheritance
- Validate variable names against injection patterns

### Resource Limits
```rust
const MAX_HOSTS_PER_PATTERN: usize = 10000;
const MAX_INHERITANCE_DEPTH: usize = 100;
const MAX_VARIABLE_NAME_LENGTH: usize = 256;
const MAX_VARIABLE_VALUE_LENGTH: usize = 4096;
```

## Implementation Phases

### Phase 1: Host Pattern Expansion (Week 1)
- [ ] Implement `HostPattern` struct and expansion logic
- [ ] Add support for numeric ranges `[01:05]`
- [ ] Add support for alphabetic ranges `[a:c]`
- [ ] Comprehensive testing of pattern expansion
- [ ] Error handling for invalid patterns

### Phase 2: INI Section Parsing (Week 2)
- [ ] Implement complete INI parsing with configparser
- [ ] Parse host sections with variable extraction
- [ ] Parse group variables sections
- [ ] Parse group children sections
- [ ] Handle special sections and edge cases

### Phase 3: Variable Inheritance (Week 3)
- [ ] Implement group dependency graph
- [ ] Add circular dependency detection
- [ ] Implement variable precedence resolution
- [ ] Add variable validation and sanitization
- [ ] Comprehensive inheritance testing

### Phase 4: Integration and Validation (Week 4)
- [ ] Integrate with existing inventory parser
- [ ] Add comprehensive validation
- [ ] Performance testing and optimization
- [ ] Documentation and examples
- [ ] Real-world inventory testing

## Success Metrics

### Functional Metrics
- All Ansible INI inventory features supported
- Host patterns expand correctly for all valid syntax
- Variable inheritance follows Ansible precedence exactly
- Error messages are clear and actionable

### Performance Metrics
- Parse 1000-host inventory in <1 second
- Memory usage <10MB for typical inventories
- Pattern expansion efficient for large ranges
- No memory leaks in long-running processes

### Quality Metrics
- 100% test coverage for core parsing logic
- All edge cases handled gracefully
- Documentation covers all features
- Integration tests with real inventories pass