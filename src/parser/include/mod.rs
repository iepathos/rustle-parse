pub mod cache;
pub mod dependency;
pub mod handler;
pub mod resolver;
pub mod roles;
pub mod tasks;
pub mod variables;

pub use cache::CachedInclude;
pub use dependency::IncludeDependencyGraph;
pub use handler::IncludeHandler;
pub use resolver::PathResolver;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for include/import processing
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

/// Specification for include directives
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncludeSpec {
    pub file: String,
    #[serde(default)]
    pub vars: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "when")]
    pub when_condition: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub apply: Option<ApplySpec>,
    pub delegate_to: Option<String>,
    pub delegate_facts: Option<bool>,
    pub run_once: Option<bool>,
}

/// Specification for import directives (subset of include features)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportSpec {
    pub file: String,
    #[serde(default)]
    pub vars: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "when")]
    pub when_condition: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Specification for role includes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoleIncludeSpec {
    pub name: String,
    pub tasks_from: Option<String>,
    pub vars_from: Option<String>,
    pub defaults_from: Option<String>,
    pub handlers_from: Option<String>,
    #[serde(default)]
    pub vars: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "when")]
    pub when_condition: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub apply: Option<ApplySpec>,
}

/// Apply block specification for includes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApplySpec {
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "when")]
    pub when_condition: Option<String>,
    pub r#become: Option<bool>,
    pub become_user: Option<String>,
}

/// Context for include processing
#[derive(Debug, Clone)]
pub struct IncludeContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub current_file: PathBuf,
    pub include_depth: usize,
    pub tags: Vec<String>,
    pub when_condition: Option<String>,
}

/// Specification for include_vars directive
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncludeVarsSpec {
    pub file: Option<String>,
    pub dir: Option<String>,
    pub name: Option<String>,
    pub depth: Option<usize>,
    pub files_matching: Option<String>,
    pub ignore_files: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    #[serde(rename = "when")]
    pub when_condition: Option<String>,
}

/// Enumeration of all include/import directive types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "directive")]
pub enum IncludeDirective {
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

/// Type of include/import operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IncludeType {
    IncludeTasks,
    ImportTasks,
    IncludePlaybook,
    ImportPlaybook,
    IncludeVars,
    IncludeRole,
    ImportRole,
}

impl std::fmt::Display for IncludeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludeType::IncludeTasks => write!(f, "include_tasks"),
            IncludeType::ImportTasks => write!(f, "import_tasks"),
            IncludeType::IncludePlaybook => write!(f, "include_playbook"),
            IncludeType::ImportPlaybook => write!(f, "import_playbook"),
            IncludeType::IncludeVars => write!(f, "include_vars"),
            IncludeType::IncludeRole => write!(f, "include_role"),
            IncludeType::ImportRole => write!(f, "import_role"),
        }
    }
}
