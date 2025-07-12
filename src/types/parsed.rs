use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Value = serde_json::Value;

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
    pub changed_when: Option<String>,
    pub failed_when: Option<String>,
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
