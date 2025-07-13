# Spec 130: Boolean Value Parsing Fix

## Feature Summary

Fix YAML parsing error where boolean values (like `false`) are incorrectly expected as strings in the `changed_when` field and similar boolean-or-string fields in Ansible playbooks. The error occurs because the current parser expects all conditional fields to be strings, but Ansible allows boolean literals (`true`/`false`) in addition to conditional expressions.

This issue prevents rustle-parse from processing valid Ansible playbooks that use boolean literals in fields like `changed_when`, `failed_when`, and similar conditional directives.

## Goals & Requirements

### Functional Requirements
- Parse boolean literals (`true`, `false`) in conditional fields like `changed_when` and `failed_when`
- Maintain backward compatibility with string-based conditional expressions
- Support YAML boolean representations: `true`, `false`, `yes`, `no`, `on`, `off`
- Preserve existing functionality for template expressions and variables in these fields

### Non-functional Requirements
- Zero performance impact on existing string parsing
- Maintain type safety with Rust's type system
- Follow Ansible's YAML parsing semantics exactly
- Comprehensive error messages for invalid values

### Success Criteria
- `tests/fixtures/playbooks/package_management.yml` parses successfully
- All existing tests continue to pass
- New tests cover boolean literal scenarios
- Parser handles mixed boolean/string usage within same playbook

## API/Interface Design

### Enhanced Data Types

```rust
// New enum to represent boolean-or-string values
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BooleanOrString {
    Boolean(bool),
    String(String),
}

impl From<bool> for BooleanOrString {
    fn from(value: bool) -> Self {
        BooleanOrString::Boolean(value)
    }
}

impl From<String> for BooleanOrString {
    fn from(value: String) -> Self {
        BooleanOrString::String(value)
    }
}

impl BooleanOrString {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            BooleanOrString::Boolean(b) => Some(*b),
            BooleanOrString::String(_) => None,
        }
    }
    
    pub fn as_string(&self) -> Option<&str> {
        match self {
            BooleanOrString::Boolean(_) => None,
            BooleanOrString::String(s) => Some(s.as_str()),
        }
    }
}
```

### Deserializer Function

```rust
fn deserialize_boolean_or_string<'de, D>(deserializer: D) -> Result<Option<BooleanOrString>, D::Error>
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
        Some(other) => Err(D::Error::invalid_type(
            serde::de::Unexpected::Other(&format!("{:?}", other)),
            &"boolean or string"
        )),
    }
}
```

### Updated Struct Definitions

```rust
// In RawTask struct
#[derive(Debug, Deserialize)]
struct RawTask {
    // ... existing fields ...
    #[serde(deserialize_with = "deserialize_boolean_or_string", default)]
    changed_when: Option<BooleanOrString>,
    #[serde(deserialize_with = "deserialize_boolean_or_string", default)]
    failed_when: Option<BooleanOrString>,
    // ... other fields ...
}

// In ParsedTask struct  
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParsedTask {
    // ... existing fields ...
    pub changed_when: Option<BooleanOrString>,
    pub failed_when: Option<BooleanOrString>,
    // ... other fields ...
}
```

## File and Package Structure

### Files to Modify
- `src/parser/playbook.rs` - Update RawTask struct and add deserializer
- `src/parser/include/handler.rs` - Update RawIncludeTask struct  
- `src/types/parsed.rs` - Update ParsedTask struct definition
- `src/types/mod.rs` - Export new BooleanOrString type

### New Files
- `src/types/boolean_or_string.rs` - New module for BooleanOrString type (optional)

### Import Structure
```rust
// In parser modules
use crate::types::BooleanOrString;

// In types/mod.rs
pub mod boolean_or_string;
pub use boolean_or_string::BooleanOrString;
```

## Implementation Details

### Step 1: Create BooleanOrString Type
1. Add `BooleanOrString` enum to `src/types/parsed.rs` or new module
2. Implement serialization/deserialization traits
3. Add helper methods for type checking and conversion
4. Export from `src/types/mod.rs`

### Step 2: Update Deserializer Function
1. Replace existing `deserialize_yaml_bool` uses with new function
2. Handle all YAML boolean representations: `true`, `false`, `yes`, `no`, `on`, `off`
3. Preserve string values that aren't boolean representations
4. Add comprehensive error handling with descriptive messages

### Step 3: Update Struct Definitions
1. Change `changed_when: Option<String>` to `changed_when: Option<BooleanOrString>`
2. Change `failed_when: Option<String>` to `failed_when: Option<BooleanOrString>`
3. Update both `RawTask` and `ParsedTask` structs
4. Update conversion logic between raw and parsed types

### Step 4: Handle Template Conversion
```rust
// In playbook.rs conversion logic
fn convert_boolean_or_string_field(
    field: Option<BooleanOrString>,
    template_engine: &TemplateEngine,
    context: &Context,
) -> Result<Option<BooleanOrString>, Error> {
    match field {
        None => Ok(None),
        Some(BooleanOrString::Boolean(b)) => Ok(Some(BooleanOrString::Boolean(b))),
        Some(BooleanOrString::String(s)) => {
            let resolved = template_engine.render(&s, context)?;
            // Try to parse resolved template as boolean
            match resolved.to_lowercase().as_str() {
                "true" | "yes" | "on" => Ok(Some(BooleanOrString::Boolean(true))),
                "false" | "no" | "off" => Ok(Some(BooleanOrString::Boolean(false))),
                _ => Ok(Some(BooleanOrString::String(resolved))),
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_boolean_literal_parsing() {
        let yaml = r#"
changed_when: false
failed_when: true
"#;
        let task: RawTask = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(task.changed_when, Some(BooleanOrString::Boolean(false)));
        assert_eq!(task.failed_when, Some(BooleanOrString::Boolean(true)));
    }
    
    #[test]
    fn test_string_boolean_parsing() {
        let yaml = r#"
changed_when: "false"
failed_when: "yes"
"#;
        let task: RawTask = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(task.changed_when, Some(BooleanOrString::Boolean(false)));
        assert_eq!(task.failed_when, Some(BooleanOrString::Boolean(true)));
    }
    
    #[test]
    fn test_conditional_expression_parsing() {
        let yaml = r#"
changed_when: "result.rc != 0"
failed_when: "ansible_hostname == 'test'"
"#;
        let task: RawTask = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(task.changed_when, Some(BooleanOrString::String(_))));
        assert!(matches!(task.failed_when, Some(BooleanOrString::String(_))));
    }
}
```

### Integration Tests
- Add test in `tests/parser/` directory
- Test parsing of `package_management.yml` fixture
- Test mixed boolean/string usage in same playbook
- Test all YAML boolean representations

### Test Files
- `tests/parser/test_boolean_parsing.rs` - New test file
- Update existing playbook parser tests
- Add fixtures with various boolean representations

## Edge Cases & Error Handling

### Edge Cases
1. **Mixed usage**: Same playbook with both boolean literals and string expressions
2. **Template resolution**: String templates that resolve to boolean values
3. **YAML variations**: Different YAML boolean representations (`yes`/`no`, `on`/`off`)
4. **Case sensitivity**: Handle `True`, `FALSE`, `YES`, etc.
5. **Numeric values**: Reject numeric values that aren't valid booleans

### Error Handling
```rust
// Enhanced error messages
match value {
    Some(serde_yaml::Value::Number(_)) => Err(D::Error::custom(
        "numeric values not supported for boolean/string fields, use 'true'/'false' or a string expression"
    )),
    Some(serde_yaml::Value::Sequence(_)) => Err(D::Error::custom(
        "arrays not supported for boolean/string fields"
    )),
    Some(other) => Err(D::Error::invalid_type(
        serde::de::Unexpected::Other(&format!("{:?}", other)),
        &"boolean literal (true/false) or string expression"
    )),
}
```

### Recovery Strategies
- Provide clear error messages with suggestions
- Include line/column information where possible
- Suggest correct syntax in error messages

## Dependencies

### External Dependencies
- No new external dependencies required
- Uses existing `serde`, `serde_yaml`, and `serde_json` crates

### Internal Dependencies
- `src/types/` module for new type definitions
- `src/parser/template.rs` for template resolution
- `src/parser/error.rs` for error handling

## Configuration

No new configuration options required. This is a parser compatibility fix that should work transparently.

## Documentation

### Rustdoc Updates
```rust
/// Represents a field that can contain either a boolean literal or a string expression.
/// 
/// Ansible allows conditional fields like `changed_when` and `failed_when` to contain:
/// - Boolean literals: `true`, `false`, `yes`, `no`, `on`, `off` 
/// - String expressions: `"result.rc != 0"`, `"{{ some_var }}"`
/// 
/// # Examples
/// 
/// ```yaml
/// # Boolean literal
/// changed_when: false
/// 
/// # String expression  
/// changed_when: "result.rc != 0"
/// 
/// # Template variable
/// failed_when: "{{ custom_condition }}"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BooleanOrString {
    /// A boolean literal value
    Boolean(bool),
    /// A string expression or template
    String(String),
}
```

### README Updates
- Document the fix for boolean literal support
- Add examples of supported boolean syntax
- Note Ansible compatibility improvements

## Implementation Phases

### Phase 1: Core Type Implementation
1. Create `BooleanOrString` enum with traits
2. Add deserializer function
3. Write comprehensive unit tests

### Phase 2: Parser Integration  
1. Update `RawTask` and `ParsedTask` structs
2. Modify field parsing for `changed_when` and `failed_when`
3. Update template resolution logic

### Phase 3: Testing & Validation
1. Add integration tests for package_management.yml
2. Test all boolean representations
3. Verify backward compatibility

### Phase 4: Documentation & Cleanup
1. Add rustdoc documentation
2. Update README with compatibility notes
3. Clean up any duplicate code

## Success Metrics

- `./target/release/rustle-parse tests/fixtures/playbooks/package_management.yml` executes without errors
- All existing tests pass without modification
- New test coverage for boolean literal scenarios achieves 100%
- No performance regression in parsing benchmarks
- Error messages are clear and actionable for invalid syntax