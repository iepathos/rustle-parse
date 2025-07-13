use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Value = serde_json::Value;

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BooleanOrString {
    /// A boolean literal value
    Boolean(bool),
    /// A string expression or template
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlaybook {
    pub metadata: PlaybookMetadata,
    pub plays: Vec<ParsedPlay>,
    pub variables: HashMap<String, Value>,
    pub facts_required: bool,
    pub vault_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookMetadata {
    pub file_path: String,
    pub version: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlay {
    pub name: String,
    pub hosts: HostPattern,
    pub vars: HashMap<String, Value>,
    pub tasks: Vec<ParsedTask>,
    pub handlers: Vec<ParsedTask>,
    pub roles: Vec<ParsedRole>,
    pub strategy: ExecutionStrategy,
    pub serial: Option<u32>,
    pub max_fail_percentage: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostPattern {
    Single(String),
    Multiple(Vec<String>),
    All,
}

impl Serialize for HostPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            HostPattern::Single(s) => s.serialize(serializer),
            HostPattern::Multiple(v) => v.serialize(serializer),
            HostPattern::All => "all".serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for HostPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(s) => {
                if s == "all" {
                    Ok(HostPattern::All)
                } else {
                    Ok(HostPattern::Single(s))
                }
            }
            serde_json::Value::Array(arr) => {
                let strings: Result<Vec<String>, _> = arr
                    .into_iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => Ok(s),
                        _ => Err(D::Error::custom("Array elements must be strings")),
                    })
                    .collect();
                Ok(HostPattern::Multiple(strings?))
            }
            _ => Err(D::Error::custom("Expected string or array of strings")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    pub id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap<String, Value>,
    pub vars: HashMap<String, Value>,
    pub when: Option<String>,
    pub loop_items: Option<Value>,
    pub tags: Vec<String>,
    pub notify: Vec<String>,
    pub changed_when: Option<BooleanOrString>,
    pub failed_when: Option<BooleanOrString>,
    pub ignore_errors: bool,
    pub delegate_to: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRole {
    pub name: String,
    pub src: Option<String>,
    pub version: Option<String>,
    pub vars: HashMap<String, Value>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStrategy {
    #[default]
    Linear,
    Free,
    Debug,
    #[serde(other)]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInventory {
    pub hosts: HashMap<String, ParsedHost>,
    pub groups: HashMap<String, ParsedGroup>,
    pub variables: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHost {
    pub name: String,
    pub address: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub vars: HashMap<String, Value>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedGroup {
    pub name: String,
    pub hosts: Vec<String>,
    pub children: Vec<String>,
    pub vars: HashMap<String, Value>,
}
