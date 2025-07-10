# Spec 050: Enhanced Template Engine

## Feature Summary

Expand the template engine to support the complete set of Ansible filters, functions, and template features. This includes adding missing filters, improving Jinja2 compatibility, supporting complex expressions, and enhancing template context management for full Ansible template compatibility.

**Problem it solves**: The current template engine only supports basic filters (default, mandatory, regex_replace, etc.). Real Ansible playbooks use dozens of additional filters for data manipulation, formatting, encoding, and complex transformations that are currently unsupported.

**High-level approach**: Extend the minijinja-based template engine with comprehensive Ansible filter implementations, add template context management, improve error handling with line numbers, and ensure 100% compatibility with Ansible's template syntax.

## Goals & Requirements

### Functional Requirements
- Implement all standard Ansible filters (50+ filters)
- Support complex template expressions and nested filters
- Add template context management with proper variable scoping
- Implement Ansible-specific template functions and tests
- Support template inheritance and macros
- Handle template errors with accurate line/column information
- Support conditional expressions and loops in templates
- Add custom filter registration for extensibility

### Non-functional Requirements
- **Performance**: Template rendering <10ms for typical templates
- **Memory**: Efficient memory usage for large template contexts
- **Compatibility**: 100% compatible with Ansible template syntax
- **Error Handling**: Clear error messages with context
- **Security**: Safe template execution with no code injection

### Success Criteria
- All Ansible filters implemented and tested
- Complex real-world templates render correctly
- Template errors provide actionable information
- Performance meets requirements for large playbooks
- Full compatibility with Ansible template test suite

## API/Interface Design

### Enhanced Template Engine
```rust
use minijinja::{Environment, Value, Error as TemplateError};
use std::collections::HashMap;

pub struct TemplateEngine {
    env: Environment<'static>,
    global_context: HashMap<String, Value>,
    filter_registry: FilterRegistry,
    test_registry: TestRegistry,
}

impl TemplateEngine {
    /// Create new template engine with all Ansible filters
    pub fn new() -> Self;
    
    /// Create template engine with custom configuration
    pub fn with_config(config: TemplateConfig) -> Self;
    
    /// Add global variables available to all templates
    pub fn add_globals(&mut self, globals: HashMap<String, Value>);
    
    /// Register custom filter
    pub fn register_filter<F>(&mut self, name: &str, filter: F) 
    where F: Fn(Value, Vec<Value>) -> Result<Value, TemplateError> + 'static;
    
    /// Register custom test function
    pub fn register_test<T>(&mut self, name: &str, test: T)
    where T: Fn(Value, Vec<Value>) -> Result<bool, TemplateError> + 'static;
    
    /// Render template string with context and error location tracking
    pub fn render_string_with_context(
        &self,
        template_str: &str,
        context: &TemplateContext,
    ) -> Result<String, TemplateError>;
    
    /// Render template from file with include support
    pub fn render_file_with_context(
        &self,
        template_path: &Path,
        context: &TemplateContext,
    ) -> Result<String, TemplateError>;
    
    /// Render value with recursive template processing
    pub fn render_value_recursive(
        &self,
        value: &serde_json::Value,
        context: &TemplateContext,
    ) -> Result<serde_json::Value, ParseError>;
    
    /// Check if string contains template expressions
    pub fn has_template_expressions(&self, text: &str) -> bool;
    
    /// Extract template variables from string
    pub fn extract_template_variables(&self, text: &str) -> Result<Vec<String>, TemplateError>;
}

/// Template rendering context with variable scoping
#[derive(Debug, Clone)]
pub struct TemplateContext {
    variables: HashMap<String, Value>,
    scopes: Vec<HashMap<String, Value>>,
    loop_context: Option<LoopContext>,
    error_context: ErrorContext,
}

impl TemplateContext {
    pub fn new(variables: HashMap<String, Value>) -> Self;
    pub fn push_scope(&mut self, scope_vars: HashMap<String, Value>);
    pub fn pop_scope(&mut self);
    pub fn get_variable(&self, name: &str) -> Option<&Value>;
    pub fn set_variable(&mut self, name: String, value: Value);
    pub fn with_loop_context(&mut self, loop_ctx: LoopContext);
}

#[derive(Debug, Clone)]
pub struct LoopContext {
    pub index: usize,
    pub index0: usize,
    pub revindex: usize,
    pub revindex0: usize,
    pub first: bool,
    pub last: bool,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub file_path: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}
```

### Filter Registry System
```rust
pub struct FilterRegistry {
    filters: HashMap<String, Box<dyn FilterFunction>>,
}

pub trait FilterFunction: Send + Sync {
    fn call(&self, args: Vec<Value>) -> Result<Value, TemplateError>;
    fn min_args(&self) -> usize { 1 }
    fn max_args(&self) -> Option<usize> { None }
    fn name(&self) -> &str;
}

impl FilterRegistry {
    pub fn new() -> Self;
    pub fn with_ansible_filters() -> Self;
    pub fn register<F>(&mut self, name: &str, filter: F)
    where F: FilterFunction + 'static;
    pub fn get(&self, name: &str) -> Option<&dyn FilterFunction>;
}

/// Macro for easy filter registration
macro_rules! register_filter {
    ($registry:expr, $name:expr, $fn:expr) => {
        $registry.register($name, SimpleFilter::new($name, $fn));
    };
}
```

### Comprehensive Filter Implementations
```rust
pub mod filters {
    use super::*;
    
    // String filters
    pub fn upper_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn lower_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn title_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn capitalize_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn trim_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn strip_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn reverse_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn length_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn wordcount_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn truncate_filter(value: Value, length: Value) -> Result<Value, TemplateError>;
    pub fn replace_filter(value: Value, old: Value, new: Value) -> Result<Value, TemplateError>;
    pub fn indent_filter(value: Value, width: Value) -> Result<Value, TemplateError>;
    pub fn center_filter(value: Value, width: Value) -> Result<Value, TemplateError>;
    
    // List filters
    pub fn first_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn last_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn min_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn max_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn sum_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn sort_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn reverse_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn unique_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn flatten_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn join_filter(value: Value, separator: Value) -> Result<Value, TemplateError>;
    pub fn select_filter(value: Value, test: Value) -> Result<Value, TemplateError>;
    pub fn reject_filter(value: Value, test: Value) -> Result<Value, TemplateError>;
    pub fn map_filter(value: Value, attribute: Value) -> Result<Value, TemplateError>;
    pub fn selectattr_filter(value: Value, attr: Value, test: Value) -> Result<Value, TemplateError>;
    pub fn rejectattr_filter(value: Value, attr: Value, test: Value) -> Result<Value, TemplateError>;
    pub fn groupby_filter(value: Value, attribute: Value) -> Result<Value, TemplateError>;
    
    // Data type filters
    pub fn int_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn float_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn string_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn bool_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn list_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Date/time filters
    pub fn strftime_filter(value: Value, format: Value) -> Result<Value, TemplateError>;
    pub fn to_datetime_filter(value: Value, format: Value) -> Result<Value, TemplateError>;
    
    // Encoding filters
    pub fn b64encode_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn b64decode_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn urlencode_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn urldecode_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Hashing filters
    pub fn hash_filter(value: Value, algorithm: Value) -> Result<Value, TemplateError>;
    pub fn password_hash_filter(value: Value, scheme: Value) -> Result<Value, TemplateError>;
    pub fn checksum_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Format filters
    pub fn to_json_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn from_json_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn to_yaml_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn from_yaml_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn to_nice_json_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn to_nice_yaml_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Network filters
    pub fn ipaddr_filter(value: Value, query: Value) -> Result<Value, TemplateError>;
    pub fn ipv4_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn ipv6_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Math filters
    pub fn abs_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn round_filter(value: Value, precision: Value) -> Result<Value, TemplateError>;
    pub fn random_filter(value: Value) -> Result<Value, TemplateError>;
    
    // Ansible-specific filters
    pub fn combine_filter(value: Value, other: Value) -> Result<Value, TemplateError>;
    pub fn extract_filter(value: Value, indices: Value) -> Result<Value, TemplateError>;
    pub fn zip_filter(value: Value, other: Value) -> Result<Value, TemplateError>;
    pub fn zip_longest_filter(value: Value, other: Value) -> Result<Value, TemplateError>;
    pub fn subelements_filter(value: Value, subelement: Value) -> Result<Value, TemplateError>;
    pub fn dict2items_filter(value: Value) -> Result<Value, TemplateError>;
    pub fn items2dict_filter(value: Value) -> Result<Value, TemplateError>;
}
```

### Template Tests
```rust
pub mod tests {
    use super::*;
    
    // Type tests
    pub fn defined_test(value: Value) -> bool;
    pub fn undefined_test(value: Value) -> bool;
    pub fn none_test(value: Value) -> bool;
    pub fn string_test(value: Value) -> bool;
    pub fn number_test(value: Value) -> bool;
    pub fn sequence_test(value: Value) -> bool;
    pub fn mapping_test(value: Value) -> bool;
    
    // Comparison tests
    pub fn equalto_test(value: Value, other: Value) -> bool;
    pub fn greaterthan_test(value: Value, other: Value) -> bool;
    pub fn lessthan_test(value: Value, other: Value) -> bool;
    
    // String tests
    pub fn match_test(value: Value, pattern: Value) -> bool;
    pub fn search_test(value: Value, pattern: Value) -> bool;
    
    // Ansible-specific tests
    pub fn version_test(value: Value, version: Value, operator: Value) -> bool;
    pub fn file_test(value: Value) -> bool;
    pub fn directory_test(value: Value) -> bool;
    pub fn link_test(value: Value) -> bool;
    pub fn exists_test(value: Value) -> bool;
}
```

## File and Package Structure

### Enhanced Template Module Structure
```
src/
├── parser/
│   ├── template/
│   │   ├── mod.rs                 # Template module exports
│   │   ├── engine.rs              # Enhanced template engine
│   │   ├── context.rs             # Template context management
│   │   ├── filters/               # Filter implementations
│   │   │   ├── mod.rs            # Filter module exports
│   │   │   ├── string.rs         # String manipulation filters
│   │   │   ├── list.rs           # List/array filters
│   │   │   ├── dict.rs           # Dictionary filters
│   │   │   ├── format.rs         # Data format filters (JSON/YAML)
│   │   │   ├── encoding.rs       # Encoding/decoding filters
│   │   │   ├── math.rs           # Mathematical filters
│   │   │   ├── date.rs           # Date/time filters
│   │   │   ├── hash.rs           # Hashing and crypto filters
│   │   │   ├── network.rs        # Network/IP filters
│   │   │   └── ansible.rs        # Ansible-specific filters
│   │   ├── tests/                 # Template test functions
│   │   │   ├── mod.rs            # Test module exports
│   │   │   ├── type_tests.rs     # Type checking tests
│   │   │   ├── comparison.rs     # Comparison tests
│   │   │   ├── string_tests.rs   # String pattern tests
│   │   │   └── ansible_tests.rs  # Ansible-specific tests
│   │   ├── error.rs              # Template error handling
│   │   └── registry.rs           # Filter and test registration
│   └── template.rs               # Main template interface (enhanced)
├── types/
│   └── template.rs               # Template-related types
└── ...

tests/
├── fixtures/
│   ├── templates/
│   │   ├── basic/                # Basic template examples
│   │   ├── filters/              # Filter-specific tests
│   │   ├── complex/              # Complex template scenarios
│   │   ├── ansible/              # Real Ansible template examples
│   │   └── edge_cases/           # Edge cases and error conditions
│   └── expected/
│       └── templates/            # Expected template outputs
└── parser/
    ├── template_engine_tests.rs  # Core engine tests
    ├── filter_tests.rs           # Comprehensive filter tests
    ├── context_tests.rs          # Context management tests
    └── integration_tests.rs      # Template integration tests
```

## Implementation Details

### Phase 1: String Manipulation Filters
```rust
// src/parser/template/filters/string.rs
use minijinja::{Error, ErrorKind, Value};
use regex::Regex;

pub fn upper_filter(value: Value) -> Result<Value, Error> {
    let string = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "upper filter requires string input")
    })?;
    Ok(Value::from(string.to_uppercase()))
}

pub fn truncate_filter(value: Value, length: Value) -> Result<Value, Error> {
    let string = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "truncate filter requires string input")
    })?;
    
    let length = length.as_usize().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "truncate length must be a number")
    })?;
    
    if string.len() <= length {
        Ok(value)
    } else {
        let truncated = &string[..length];
        Ok(Value::from(format!("{}...", truncated)))
    }
}

pub fn indent_filter(value: Value, width: Value, indent_blank_lines: Option<Value>) -> Result<Value, Error> {
    let string = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "indent filter requires string input")
    })?;
    
    let width = width.as_usize().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "indent width must be a number")
    })?;
    
    let indent_blank = indent_blank_lines
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let indent_str = " ".repeat(width);
    let lines: Vec<String> = string
        .lines()
        .map(|line| {
            if line.trim().is_empty() && !indent_blank {
                line.to_string()
            } else {
                format!("{}{}", indent_str, line)
            }
        })
        .collect();
    
    Ok(Value::from(lines.join("\n")))
}

pub fn regex_replace_filter(
    value: Value,
    pattern: Value,
    replacement: Value,
    count: Option<Value>,
) -> Result<Value, Error> {
    let string = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "regex_replace requires string input")
    })?;
    
    let pattern_str = pattern.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "regex_replace pattern must be string")
    })?;
    
    let replacement_str = replacement.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "regex_replace replacement must be string")
    })?;
    
    let regex = Regex::new(pattern_str)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("Invalid regex: {}", e)))?;
    
    let result = if let Some(count_val) = count {
        let count_num = count_val.as_usize().ok_or_else(|| {
            Error::new(ErrorKind::InvalidOperation, "regex_replace count must be number")
        })?;
        regex.replacen(string, count_num, replacement_str)
    } else {
        regex.replace_all(string, replacement_str)
    };
    
    Ok(Value::from(result.to_string()))
}
```

### Phase 2: Data Format Filters
```rust
// src/parser/template/filters/format.rs
use serde_json;
use serde_yaml;

pub fn to_json_filter(value: Value, indent: Option<Value>) -> Result<Value, Error> {
    let json_value = value_to_serde_json(&value);
    
    let json_string = if let Some(indent_val) = indent {
        let indent_size = indent_val.as_usize().unwrap_or(2);
        serde_json::to_string_pretty(&json_value)
            .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("JSON serialization failed: {}", e)))?
    } else {
        serde_json::to_string(&json_value)
            .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("JSON serialization failed: {}", e)))?
    };
    
    Ok(Value::from(json_string))
}

pub fn from_json_filter(value: Value) -> Result<Value, Error> {
    let json_str = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "from_json requires string input")
    })?;
    
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("JSON parsing failed: {}", e)))?;
    
    Ok(serde_json_to_minijinja_value(&parsed))
}

pub fn to_yaml_filter(value: Value, indent: Option<Value>) -> Result<Value, Error> {
    let json_value = value_to_serde_json(&value);
    
    let yaml_string = serde_yaml::to_string(&json_value)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("YAML serialization failed: {}", e)))?;
    
    Ok(Value::from(yaml_string))
}

pub fn from_yaml_filter(value: Value) -> Result<Value, Error> {
    let yaml_str = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "from_yaml requires string input")
    })?;
    
    let parsed: serde_yaml::Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("YAML parsing failed: {}", e)))?;
    
    // Convert serde_yaml::Value to serde_json::Value then to minijinja::Value
    let json_value: serde_json::Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("YAML parsing failed: {}", e)))?;
    
    Ok(serde_json_to_minijinja_value(&json_value))
}

fn value_to_serde_json(value: &Value) -> serde_json::Value {
    match value.kind() {
        minijinja::ValueKind::Undefined | minijinja::ValueKind::None => serde_json::Value::Null,
        minijinja::ValueKind::Bool => serde_json::Value::Bool(value.is_true()),
        minijinja::ValueKind::Number => {
            if let Some(i) = value.as_i64() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Some(f) = value.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        minijinja::ValueKind::String => serde_json::Value::String(value.to_string()),
        minijinja::ValueKind::Seq => {
            let array: Vec<serde_json::Value> = value
                .try_iter()
                .unwrap()
                .map(|v| value_to_serde_json(&v))
                .collect();
            serde_json::Value::Array(array)
        }
        minijinja::ValueKind::Map => {
            let mut map = serde_json::Map::new();
            if let Ok(object) = value.try_iter() {
                for key in object {
                    if let Some(val) = value.get_item(&key) {
                        map.insert(key.to_string(), value_to_serde_json(&val));
                    }
                }
            }
            serde_json::Value::Object(map)
        }
    }
}
```

### Phase 3: List Processing Filters
```rust
// src/parser/template/filters/list.rs
pub fn select_filter(value: Value, test_name: Value) -> Result<Value, Error> {
    let test_str = test_name.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "select test name must be string")
    })?;
    
    let items: Vec<Value> = value.try_iter()
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "select requires iterable input"))?
        .collect();
    
    let filtered: Vec<Value> = items
        .into_iter()
        .filter(|item| {
            // Apply the test function
            match test_str {
                "defined" => !item.is_undefined() && !item.is_none(),
                "undefined" => item.is_undefined(),
                "none" => item.is_none(),
                "string" => item.is_string(),
                "number" => item.is_number(),
                _ => false, // Unknown test, filter out
            }
        })
        .collect();
    
    Ok(Value::from(filtered))
}

pub fn map_filter(value: Value, attribute: Value) -> Result<Value, Error> {
    let attr_name = attribute.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "map attribute must be string")
    })?;
    
    let items: Vec<Value> = value.try_iter()
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "map requires iterable input"))?
        .collect();
    
    let mapped: Vec<Value> = items
        .into_iter()
        .map(|item| {
            item.get_attr(attr_name)
                .unwrap_or(Value::UNDEFINED)
        })
        .collect();
    
    Ok(Value::from(mapped))
}

pub fn groupby_filter(value: Value, attribute: Value) -> Result<Value, Error> {
    let attr_name = attribute.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "groupby attribute must be string")
    })?;
    
    let items: Vec<Value> = value.try_iter()
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "groupby requires iterable input"))?
        .collect();
    
    let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
    
    for item in items {
        let group_key = item.get_attr(attr_name)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "".to_string());
        
        groups.entry(group_key).or_insert_with(Vec::new).push(item);
    }
    
    let result: Vec<Value> = groups
        .into_iter()
        .map(|(key, group_items)| {
            Value::from(vec![
                Value::from(key),
                Value::from(group_items),
            ])
        })
        .collect();
    
    Ok(Value::from(result))
}

pub fn combine_filter(value: Value, other: Value, recursive: Option<Value>) -> Result<Value, Error> {
    if !value.is_object() || !other.is_object() {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "combine filter requires object inputs"
        ));
    }
    
    let recursive_merge = recursive.and_then(|v| v.as_bool()).unwrap_or(false);
    
    let mut result = value.clone();
    
    if let Ok(other_iter) = other.try_iter() {
        for key in other_iter {
            if let Some(other_value) = other.get_item(&key) {
                if recursive_merge && result.get_item(&key).is_some() {
                    let existing = result.get_item(&key).unwrap();
                    if existing.is_object() && other_value.is_object() {
                        // Recursive merge for nested objects
                        let merged = combine_filter(existing, other_value, Some(Value::from(true)))?;
                        // Note: In real implementation, we'd need to update the result
                        // This is simplified for demonstration
                    }
                }
                // Note: In real implementation, we'd need to set the item
                // result.set_item(&key, other_value);
            }
        }
    }
    
    Ok(result)
}
```

### Phase 4: Enhanced Template Context
```rust
// src/parser/template/context.rs
impl TemplateContext {
    pub fn new(variables: HashMap<String, Value>) -> Self {
        Self {
            variables,
            scopes: Vec::new(),
            loop_context: None,
            error_context: ErrorContext {
                file_path: None,
                line: None,
                column: None,
            },
        }
    }
    
    pub fn push_scope(&mut self, scope_vars: HashMap<String, Value>) {
        self.scopes.push(scope_vars);
    }
    
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        // Search in reverse order: scopes (newest first), then global variables
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        
        self.variables.get(name)
    }
    
    pub fn set_variable(&mut self, name: String, value: Value) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert(name, value);
        } else {
            self.variables.insert(name, value);
        }
    }
    
    pub fn with_loop_context(&mut self, loop_ctx: LoopContext) {
        self.loop_context = Some(loop_ctx);
        
        // Add loop variables to current scope
        let mut loop_vars = HashMap::new();
        loop_vars.insert("loop".to_string(), Value::from_object({
            let mut obj = HashMap::new();
            obj.insert("index".to_string(), Value::from(loop_ctx.index));
            obj.insert("index0".to_string(), Value::from(loop_ctx.index0));
            obj.insert("revindex".to_string(), Value::from(loop_ctx.revindex));
            obj.insert("revindex0".to_string(), Value::from(loop_ctx.revindex0));
            obj.insert("first".to_string(), Value::from(loop_ctx.first));
            obj.insert("last".to_string(), Value::from(loop_ctx.last));
            obj.insert("length".to_string(), Value::from(loop_ctx.length));
            obj
        }));
        
        self.push_scope(loop_vars);
    }
    
    pub fn to_minijinja_context(&self) -> HashMap<String, Value> {
        let mut context = self.variables.clone();
        
        // Add all scope variables (later scopes override earlier ones)
        for scope in &self.scopes {
            context.extend(scope.clone());
        }
        
        context
    }
}
```

## Testing Strategy

### Comprehensive Filter Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_filters() {
        let engine = TemplateEngine::new();
        
        // Test upper filter
        let result = engine.render_string("{{ 'hello' | upper }}", &HashMap::new()).unwrap();
        assert_eq!(result, "HELLO");
        
        // Test truncate filter
        let result = engine.render_string("{{ 'hello world' | truncate(5) }}", &HashMap::new()).unwrap();
        assert_eq!(result, "hello...");
        
        // Test indent filter
        let result = engine.render_string("{{ 'line1\\nline2' | indent(2) }}", &HashMap::new()).unwrap();
        assert_eq!(result, "  line1\n  line2");
    }
    
    #[test]
    fn test_list_filters() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("items".to_string(), Value::from(vec![1, 2, 3, 4, 5]));
        
        // Test first/last
        let result = engine.render_string("{{ items | first }}", &context).unwrap();
        assert_eq!(result, "1");
        
        let result = engine.render_string("{{ items | last }}", &context).unwrap();
        assert_eq!(result, "5");
        
        // Test min/max
        let result = engine.render_string("{{ items | min }}", &context).unwrap();
        assert_eq!(result, "1");
        
        let result = engine.render_string("{{ items | max }}", &context).unwrap();
        assert_eq!(result, "5");
        
        // Test join
        let result = engine.render_string("{{ items | join(', ') }}", &context).unwrap();
        assert_eq!(result, "1, 2, 3, 4, 5");
    }
    
    #[test]
    fn test_format_filters() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        
        let data = serde_json::json!({
            "name": "test",
            "value": 42
        });
        context.insert("data".to_string(), serde_json_to_minijinja_value(&data));
        
        // Test to_json
        let result = engine.render_string("{{ data | to_json }}", &context).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["value"], 42);
        
        // Test to_yaml
        let result = engine.render_string("{{ data | to_yaml }}", &context).unwrap();
        assert!(result.contains("name: test"));
        assert!(result.contains("value: 42"));
    }
    
    #[test]
    fn test_encoding_filters() {
        let engine = TemplateEngine::new();
        
        // Test base64 encoding/decoding
        let result = engine.render_string("{{ 'hello' | b64encode }}", &HashMap::new()).unwrap();
        assert_eq!(result, "aGVsbG8=");
        
        let result = engine.render_string("{{ 'aGVsbG8=' | b64decode }}", &HashMap::new()).unwrap();
        assert_eq!(result, "hello");
        
        // Test URL encoding
        let result = engine.render_string("{{ 'hello world' | urlencode }}", &HashMap::new()).unwrap();
        assert_eq!(result, "hello%20world");
    }
    
    #[test]
    fn test_conditional_expressions() {
        let engine = TemplateEngine::new();
        let mut context = HashMap::new();
        context.insert("enabled".to_string(), Value::from(true));
        context.insert("name".to_string(), Value::from("test"));
        
        // Test ternary operator
        let result = engine.render_string(
            "{{ 'yes' if enabled else 'no' }}", 
            &context
        ).unwrap();
        assert_eq!(result, "yes");
        
        // Test with filters
        let result = engine.render_string(
            "{{ name | upper if enabled else name | lower }}", 
            &context
        ).unwrap();
        assert_eq!(result, "TEST");
    }
}
```

### Integration Testing with Real Templates
```rust
// tests/parser/template_integration_tests.rs
#[tokio::test]
async fn test_real_ansible_template() {
    let template_content = r#"
# Generated configuration for {{ inventory_hostname }}
server {
    listen {{ http_port | default(80) }};
    server_name {{ server_name | default(inventory_hostname) }};
    
    {% if ssl_enabled | default(false) %}
    listen {{ https_port | default(443) }} ssl;
    ssl_certificate {{ ssl_cert_path }};
    ssl_certificate_key {{ ssl_key_path }};
    {% endif %}
    
    location / {
        {% for backend in backends %}
        proxy_pass http://{{ backend.host }}:{{ backend.port }};
        {% endfor %}
    }
    
    {% if custom_headers is defined %}
    {% for header in custom_headers %}
    add_header {{ header.name }} "{{ header.value }}";
    {% endfor %}
    {% endif %}
}
"#;
    
    let mut context = HashMap::new();
    context.insert("inventory_hostname".to_string(), Value::from("web01.example.com"));
    context.insert("http_port".to_string(), Value::from(8080));
    context.insert("ssl_enabled".to_string(), Value::from(true));
    context.insert("https_port".to_string(), Value::from(8443));
    context.insert("ssl_cert_path".to_string(), Value::from("/etc/ssl/cert.pem"));
    context.insert("ssl_key_path".to_string(), Value::from("/etc/ssl/key.pem"));
    context.insert("backends".to_string(), Value::from(vec![
        Value::from_object({
            let mut obj = HashMap::new();
            obj.insert("host".to_string(), Value::from("backend1"));
            obj.insert("port".to_string(), Value::from(3000));
            obj
        }),
        Value::from_object({
            let mut obj = HashMap::new();
            obj.insert("host".to_string(), Value::from("backend2"));
            obj.insert("port".to_string(), Value::from(3000));
            obj
        }),
    ]));
    
    let engine = TemplateEngine::new();
    let template_context = TemplateContext::new(context);
    let result = engine.render_string_with_context(template_content, &template_context).unwrap();
    
    // Verify the template rendered correctly
    assert!(result.contains("listen 8080;"));
    assert!(result.contains("server_name web01.example.com;"));
    assert!(result.contains("listen 8443 ssl;"));
    assert!(result.contains("proxy_pass http://backend1:3000;"));
    assert!(result.contains("proxy_pass http://backend2:3000;"));
}
```

## Edge Cases & Error Handling

### Template Error Context
```rust
// src/parser/template/error.rs
use minijinja::{Error as TemplateError, ErrorKind};

pub fn enhance_template_error(
    error: TemplateError,
    context: &ErrorContext,
) -> ParseError {
    let message = if let Some(file_path) = &context.file_path {
        format!(
            "Template error in {} at line {}: {}",
            file_path,
            context.line.unwrap_or(0),
            error
        )
    } else {
        format!("Template error: {}", error)
    };
    
    ParseError::Template {
        file: context.file_path.clone().unwrap_or_else(|| "inline".to_string()),
        line: context.line.unwrap_or(0),
        message,
    }
}

pub fn validate_filter_arguments(
    filter_name: &str,
    args: &[Value],
    min_args: usize,
    max_args: Option<usize>,
) -> Result<(), TemplateError> {
    if args.len() < min_args {
        return Err(TemplateError::new(
            ErrorKind::InvalidOperation,
            format!("{} filter requires at least {} arguments, got {}", 
                   filter_name, min_args, args.len())
        ));
    }
    
    if let Some(max) = max_args {
        if args.len() > max {
            return Err(TemplateError::new(
                ErrorKind::InvalidOperation,
                format!("{} filter accepts at most {} arguments, got {}", 
                       filter_name, max, args.len())
            ));
        }
    }
    
    Ok(())
}
```

### Security and Performance Safeguards
```rust
impl TemplateEngine {
    const MAX_TEMPLATE_SIZE: usize = 1024 * 1024; // 1MB
    const MAX_RECURSION_DEPTH: usize = 100;
    const MAX_LOOP_ITERATIONS: usize = 10000;
    
    pub fn render_string_safe(&self, template_str: &str, context: &TemplateContext) -> Result<String, ParseError> {
        // Check template size
        if template_str.len() > Self::MAX_TEMPLATE_SIZE {
            return Err(ParseError::Template {
                file: "inline".to_string(),
                line: 0,
                message: "Template too large".to_string(),
            });
        }
        
        // Set up safe environment with limits
        let mut env = self.env.clone();
        env.set_max_recursion_depth(Self::MAX_RECURSION_DEPTH);
        // Note: In real implementation, we'd configure additional safety limits
        
        self.render_string_with_context(template_str, context)
    }
}
```

## Dependencies

### Additional Template Dependencies
```toml
[dependencies]
# Enhanced template engine (already present)
minijinja = { version = "2", features = ["debug", "loader"] }

# String processing
regex = "1.11"         # Already present
unicode-segmentation = "1.12"  # Proper string segmentation

# Data processing
itertools = "0.13"     # Enhanced iterator utilities
indexmap = "2.6"       # Ordered maps

# Date/time handling (already present)
chrono = { version = "0.4", features = ["serde"] }

# Encoding and hashing (already present)
base64 = "0.22"
sha2 = "0.10"
url = "2.5"            # URL encoding/decoding

# Network utilities for IP filters
ipnet = "2.10"         # IP address handling

# Password hashing
argon2 = "0.5"         # Modern password hashing
bcrypt = "0.15"        # Legacy password hashing support
```

## Configuration

### Template Engine Configuration
```rust
#[derive(Debug, Clone)]
pub struct TemplateConfig {
    pub strict_undefined: bool,         // Error on undefined variables
    pub auto_escape: bool,              // Automatic HTML escaping
    pub trim_blocks: bool,              // Trim newlines after blocks
    pub lstrip_blocks: bool,            // Strip leading whitespace
    pub keep_trailing_newline: bool,    // Preserve trailing newlines
    pub max_recursion_depth: usize,     // Template recursion limit
    pub enable_custom_filters: bool,    // Allow custom filter registration
    pub ansible_compatibility: bool,   // Enable Ansible-specific features
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            strict_undefined: false,  // Ansible default
            auto_escape: false,       // Ansible doesn't auto-escape
            trim_blocks: true,        // Ansible default
            lstrip_blocks: true,      // Ansible default
            keep_trailing_newline: true,  // Ansible default
            max_recursion_depth: 100,
            enable_custom_filters: true,
            ansible_compatibility: true,
        }
    }
}
```

## Performance Considerations

### Template Compilation Caching
```rust
use lru::LruCache;

pub struct TemplateCache {
    compiled_templates: LruCache<String, minijinja::Template<'static>>,
    template_sources: LruCache<String, String>,
}

impl TemplateCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            compiled_templates: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
            template_sources: LruCache::new(NonZeroUsize::new(capacity * 2).unwrap()),
        }
    }
    
    pub fn get_or_compile(&mut self, template_str: &str, env: &Environment) -> Result<&minijinja::Template, TemplateError> {
        let cache_key = self.generate_cache_key(template_str);
        
        if !self.compiled_templates.contains(&cache_key) {
            let template = env.template_from_str(template_str)?;
            self.compiled_templates.put(cache_key.clone(), template);
            self.template_sources.put(cache_key.clone(), template_str.to_string());
        }
        
        Ok(self.compiled_templates.get(&cache_key).unwrap())
    }
}
```

### Optimized Filter Implementation
```rust
// Use lazy static for compiled regexes
use once_cell::sync::Lazy;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
});

// Efficient string operations
pub fn wordwrap_filter(value: Value, width: Value, break_long_words: Option<Value>) -> Result<Value, Error> {
    let text = value.as_str().ok_or_else(|| {
        Error::new(ErrorKind::InvalidOperation, "wordwrap requires string input")
    })?;
    
    let width = width.as_usize().unwrap_or(79);
    let break_long = break_long_words.and_then(|v| v.as_bool()).unwrap_or(true);
    
    // Use efficient string building
    let mut result = String::with_capacity(text.len() * 2);
    let mut current_line_len = 0;
    
    for word in text.split_whitespace() {
        if current_line_len + word.len() + 1 > width {
            if !result.is_empty() {
                result.push('\n');
            }
            
            if break_long && word.len() > width {
                // Break long words
                let mut remaining = word;
                while remaining.len() > width {
                    result.push_str(&remaining[..width]);
                    result.push('\n');
                    remaining = &remaining[width..];
                }
                result.push_str(remaining);
                current_line_len = remaining.len();
            } else {
                result.push_str(word);
                current_line_len = word.len();
            }
        } else {
            if current_line_len > 0 {
                result.push(' ');
                current_line_len += 1;
            }
            result.push_str(word);
            current_line_len += word.len();
        }
    }
    
    Ok(Value::from(result))
}
```

## Implementation Phases

### Phase 1: Core Filter Implementation (Week 1-2)
- [ ] Implement string manipulation filters (20+ filters)
- [ ] Implement list processing filters (15+ filters)
- [ ] Implement data type conversion filters
- [ ] Basic filter registration and testing
- [ ] Error handling for filter operations

### Phase 2: Advanced Filters (Week 3)
- [ ] Implement encoding/decoding filters
- [ ] Implement hashing and crypto filters
- [ ] Implement date/time filters
- [ ] Implement network/IP filters
- [ ] Format conversion filters (JSON/YAML)

### Phase 3: Template Features (Week 4)
- [ ] Enhanced template context management
- [ ] Template test functions implementation
- [ ] Loop context and conditional expressions
- [ ] Template error handling with line numbers
- [ ] Template compilation caching

### Phase 4: Integration and Optimization (Week 5)
- [ ] Integration with existing parser components
- [ ] Performance optimization and benchmarking
- [ ] Comprehensive testing with real templates
- [ ] Documentation and examples
- [ ] Security review and hardening

## Success Metrics

### Functional Metrics
- All 50+ Ansible filters implemented and tested
- Complex real-world templates render correctly
- Template expressions evaluate with proper precedence
- Error messages provide actionable information

### Performance Metrics
- Template rendering <10ms for typical templates
- Filter operations <1ms for standard inputs
- Memory usage proportional to template complexity
- Template compilation caching >90% hit rate

### Compatibility Metrics
- 100% compatibility with Ansible filter syntax
- All Ansible template test cases pass
- Real-world playbook templates work unchanged
- Error behavior matches Ansible exactly