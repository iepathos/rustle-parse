pub mod cache;
pub mod dependency;
pub mod error;
pub mod inventory;
pub mod playbook;
pub mod template;
pub mod validator;
pub mod vault;

pub use error::ParseError;
pub use inventory::InventoryParser;
pub use playbook::PlaybookParser;
pub use template::TemplateEngine;

use crate::types::parsed::{ParsedInventory, ParsedPlaybook};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Parser {
    vault_password: Option<String>,
    extra_vars: HashMap<String, serde_json::Value>,
    template_engine: TemplateEngine,
    cache: Option<cache::ParseCache>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            vault_password: None,
            extra_vars: HashMap::new(),
            template_engine: TemplateEngine::new(),
            cache: None,
        }
    }

    pub fn with_vault_password(mut self, password: String) -> Self {
        self.vault_password = Some(password);
        self
    }

    pub fn with_extra_vars(mut self, vars: HashMap<String, serde_json::Value>) -> Self {
        self.extra_vars = vars;
        self
    }

    pub fn with_cache(mut self, cache_dir: PathBuf) -> Self {
        self.cache = Some(cache::ParseCache::new(cache_dir));
        self
    }

    pub async fn parse_playbook(&self, path: &Path) -> Result<ParsedPlaybook, ParseError> {
        let parser = PlaybookParser::new(&self.template_engine, &self.extra_vars);
        parser.parse(path).await
    }

    pub async fn parse_inventory(&self, path: &Path) -> Result<ParsedInventory, ParseError> {
        let parser = InventoryParser::new(&self.template_engine, &self.extra_vars);
        parser.parse(path).await
    }

    pub async fn validate_syntax(&self, path: &Path) -> Result<(), ParseError> {
        validator::validate_playbook_syntax(path).await
    }

    pub fn resolve_dependencies(&self, playbook: &ParsedPlaybook) -> Vec<String> {
        dependency::resolve_task_dependencies(&playbook.plays)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
