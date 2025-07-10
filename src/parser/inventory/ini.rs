use crate::parser::error::ParseError;
use crate::parser::inventory::patterns::HostPattern;
use crate::parser::template::TemplateEngine;
use crate::types::parsed::*;
use configparser::ini::Ini;
use std::collections::{HashMap, HashSet};

/// Internal structure for parsing INI sections
#[derive(Debug)]
struct IniSection {
    name: String,
    section_type: SectionType,
    entries: Vec<IniEntry>,
}

#[derive(Debug, PartialEq)]
enum SectionType {
    Hosts,         // [groupname]
    GroupVars,     // [groupname:vars]
    GroupChildren, // [groupname:children]
}

#[derive(Debug)]
struct IniEntry {
    key: String,
    value: Option<String>,
    variables: HashMap<String, String>,
}

/// Configuration for INI inventory parsing
#[derive(Debug, Clone)]
pub struct InventoryParserConfig {
    pub strict_mode: bool,            // Fail on warnings
    pub expand_patterns: bool,        // Enable host pattern expansion
    pub max_pattern_expansion: usize, // Limit pattern expansion size
    pub validate_hosts: bool,         // Validate host connectivity
    pub resolve_dns: bool,            // Resolve hostnames to IPs
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

/// Extended inventory parser with complete INI support
pub struct IniInventoryParser<'a> {
    #[allow(dead_code)]
    template_engine: &'a TemplateEngine,
    extra_vars: &'a HashMap<String, serde_json::Value>,
    config: InventoryParserConfig,
}

impl<'a> IniInventoryParser<'a> {
    pub fn new(
        template_engine: &'a TemplateEngine,
        extra_vars: &'a HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            template_engine,
            extra_vars,
            config: InventoryParserConfig::default(),
        }
    }

    pub fn with_config(
        template_engine: &'a TemplateEngine,
        extra_vars: &'a HashMap<String, serde_json::Value>,
        config: InventoryParserConfig,
    ) -> Self {
        Self {
            template_engine,
            extra_vars,
            config,
        }
    }

    /// Parse INI inventory with complete feature support
    pub async fn parse_ini_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError> {
        let mut config = Ini::new();
        config
            .read(content.to_string())
            .map_err(|e| ParseError::IniParsing {
                message: format!("Failed to parse INI: {e}"),
            })?;

        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: self.extra_vars.clone(),
        };

        // Parse all sections first
        let sections = self.parse_ini_sections(&config)?;

        // Process each section type in order
        for section in &sections {
            match &section.section_type {
                SectionType::Hosts => {
                    self.process_hosts_section(&mut inventory, section)?;
                }
                SectionType::GroupVars => {
                    self.process_group_vars_section(&mut inventory, section)?;
                }
                SectionType::GroupChildren => {
                    self.process_group_children_section(&mut inventory, section)?;
                }
            }
        }

        // Ensure the "all" group exists and contains all hosts
        self.ensure_all_group(&mut inventory);

        // Validate final inventory structure
        self.validate_inventory_structure(&inventory)?;

        Ok(inventory)
    }

    /// Parse all INI sections into structured data
    fn parse_ini_sections(&self, config: &Ini) -> Result<Vec<IniSection>, ParseError> {
        let mut sections = Vec::new();

        for section_name in config.sections() {
            let section_type = self.determine_section_type(&section_name);
            let mut entries = Vec::new();

            if let Some(section_map) = config.get_map_ref().get(&section_name) {
                for (key, value) in section_map {
                    let variables = if section_type == SectionType::Hosts {
                        self.parse_host_variables(value.as_deref().unwrap_or(""))?
                    } else {
                        HashMap::new()
                    };

                    entries.push(IniEntry {
                        key: key.clone(),
                        value: value.clone(),
                        variables,
                    });
                }
            }

            sections.push(IniSection {
                name: section_name,
                section_type,
                entries,
            });
        }

        Ok(sections)
    }

    /// Determine the type of INI section
    fn determine_section_type(&self, section_name: &str) -> SectionType {
        if section_name.ends_with(":vars") {
            SectionType::GroupVars
        } else if section_name.ends_with(":children") {
            SectionType::GroupChildren
        } else {
            SectionType::Hosts
        }
    }

    /// Process a hosts section
    fn process_hosts_section(
        &self,
        inventory: &mut ParsedInventory,
        section: &IniSection,
    ) -> Result<(), ParseError> {
        let group_name = section.name.clone();
        let mut group_hosts = Vec::new();

        for entry in &section.entries {
            let hosts = if self.config.expand_patterns {
                self.expand_host_pattern(&entry.key)?
            } else {
                vec![entry.key.clone()]
            };

            for hostname in hosts {
                // Check for duplicate hosts
                if inventory.hosts.contains_key(&hostname) && self.config.strict_mode {
                    return Err(ParseError::DuplicateHost { host: hostname });
                }

                let (address, port, user) = self.extract_connection_info(&entry.variables);

                let host = ParsedHost {
                    name: hostname.clone(),
                    address,
                    port,
                    user,
                    vars: entry
                        .variables
                        .iter()
                        .map(|(k, v)| (k.clone(), self.parse_ini_value(v)))
                        .collect(),
                    groups: vec![group_name.clone()],
                };

                inventory.hosts.insert(hostname.clone(), host);
                group_hosts.push(hostname);
            }
        }

        // Create or update the group
        let group = ParsedGroup {
            name: group_name.clone(),
            hosts: group_hosts,
            children: Vec::new(),
            vars: HashMap::new(),
        };

        inventory.groups.insert(group_name, group);
        Ok(())
    }

    /// Process a group variables section
    fn process_group_vars_section(
        &self,
        inventory: &mut ParsedInventory,
        section: &IniSection,
    ) -> Result<(), ParseError> {
        let group_name = section.name.trim_end_matches(":vars");

        // Ensure the group exists
        if !inventory.groups.contains_key(group_name) {
            inventory.groups.insert(
                group_name.to_string(),
                ParsedGroup {
                    name: group_name.to_string(),
                    hosts: Vec::new(),
                    children: Vec::new(),
                    vars: HashMap::new(),
                },
            );
        }

        if let Some(group) = inventory.groups.get_mut(group_name) {
            for entry in &section.entries {
                let value = entry.value.as_deref().unwrap_or("");
                group
                    .vars
                    .insert(entry.key.clone(), self.parse_ini_value(value));
            }
        }

        Ok(())
    }

    /// Process a group children section
    fn process_group_children_section(
        &self,
        inventory: &mut ParsedInventory,
        section: &IniSection,
    ) -> Result<(), ParseError> {
        let group_name = section.name.trim_end_matches(":children");

        // Ensure the parent group exists
        if !inventory.groups.contains_key(group_name) {
            inventory.groups.insert(
                group_name.to_string(),
                ParsedGroup {
                    name: group_name.to_string(),
                    hosts: Vec::new(),
                    children: Vec::new(),
                    vars: HashMap::new(),
                },
            );
        }

        let children: Vec<String> = section
            .entries
            .iter()
            .map(|entry| entry.key.clone())
            .collect();

        // Validate that all child groups exist
        for child_name in &children {
            if !inventory.groups.contains_key(child_name) && self.config.strict_mode {
                return Err(ParseError::UnknownGroup {
                    group: child_name.clone(),
                });
            }
        }

        if let Some(group) = inventory.groups.get_mut(group_name) {
            group.children = children;
        }

        Ok(())
    }

    /// Parse host patterns like web[01:05] into individual hosts
    pub fn expand_host_pattern(&self, pattern: &str) -> Result<Vec<String>, ParseError> {
        let host_pattern = HostPattern::new(pattern)?;
        if host_pattern.expanded.len() > self.config.max_pattern_expansion {
            return Err(ParseError::InvalidHostPattern {
                pattern: pattern.to_string(),
                line: 0,
                message: format!(
                    "Pattern expansion exceeds maximum limit of {} hosts",
                    self.config.max_pattern_expansion
                ),
            });
        }
        Ok(host_pattern.expanded)
    }

    /// Parse inline host variables from inventory line
    pub fn parse_host_variables(
        &self,
        vars_str: &str,
    ) -> Result<HashMap<String, String>, ParseError> {
        let mut vars = HashMap::new();

        if vars_str.is_empty() {
            return Ok(vars);
        }

        // Parse shell-style key=value pairs with proper quoting
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
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
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                }
                '=' if !in_quotes && in_key => {
                    in_key = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !in_key && !current_key.is_empty() {
                        // End of key=value pair
                        vars.insert(
                            current_key.trim().to_string(),
                            current_value.trim().to_string(),
                        );
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
            vars.insert(
                current_key.trim().to_string(),
                current_value.trim().to_string(),
            );
        }

        Ok(vars)
    }

    /// Parse INI value and convert to JSON value
    fn parse_ini_value(&self, value: &str) -> serde_json::Value {
        let value = value.trim();

        if value.is_empty() {
            return serde_json::Value::String(value.to_string());
        }

        // Remove quotes if present
        let unquoted = if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };

        // Boolean
        match unquoted.to_lowercase().as_str() {
            "true" | "yes" | "on" => return serde_json::Value::Bool(true),
            "false" | "no" | "off" => return serde_json::Value::Bool(false),
            _ => {}
        }

        // Number
        if let Ok(int_val) = unquoted.parse::<i64>() {
            return serde_json::Value::Number(serde_json::Number::from(int_val));
        }
        if let Ok(float_val) = unquoted.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                return serde_json::Value::Number(num);
            }
        }

        // String (default)
        serde_json::Value::String(unquoted.to_string())
    }

    /// Extract connection information from variables
    fn extract_connection_info(
        &self,
        vars: &HashMap<String, String>,
    ) -> (Option<String>, Option<u16>, Option<String>) {
        let address = vars
            .get("ansible_host")
            .or_else(|| vars.get("ansible_ssh_host")).cloned();

        let port = vars
            .get("ansible_port")
            .or_else(|| vars.get("ansible_ssh_port"))
            .and_then(|s| s.parse::<u16>().ok());

        let user = vars
            .get("ansible_user")
            .or_else(|| vars.get("ansible_ssh_user"))
            .or_else(|| vars.get("ansible_ssh_user_name")).cloned();

        (address, port, user)
    }

    /// Ensure the "all" group exists and contains all hosts
    fn ensure_all_group(&self, inventory: &mut ParsedInventory) {
        let all_hosts: Vec<String> = inventory.hosts.keys().cloned().collect();

        // Update existing hosts to include "all" group membership
        for host in inventory.hosts.values_mut() {
            if !host.groups.contains(&"all".to_string()) {
                host.groups.push("all".to_string());
            }
        }

        // Create or update the "all" group
        let all_group = ParsedGroup {
            name: "all".to_string(),
            hosts: all_hosts,
            children: Vec::new(),
            vars: HashMap::new(),
        };

        inventory.groups.insert("all".to_string(), all_group);
    }

    /// Validate inventory structure
    fn validate_inventory_structure(&self, inventory: &ParsedInventory) -> Result<(), ParseError> {
        // Check for circular dependencies in group children
        self.check_circular_dependencies(inventory)?;

        // Validate that all referenced hosts exist
        for (group_name, group) in &inventory.groups {
            for host_name in &group.hosts {
                if !inventory.hosts.contains_key(host_name) && self.config.strict_mode {
                    return Err(ParseError::InvalidStructure {
                        message: format!(
                            "Group '{group_name}' references non-existent host '{host_name}'"
                        ),
                    });
                }
            }

            for child_name in &group.children {
                if !inventory.groups.contains_key(child_name) && self.config.strict_mode {
                    return Err(ParseError::UnknownGroup {
                        group: child_name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check for circular dependencies in group hierarchy
    fn check_circular_dependencies(&self, inventory: &ParsedInventory) -> Result<(), ParseError> {
        for group_name in inventory.groups.keys() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if Self::has_circular_dependency(inventory, group_name, &mut visited, &mut path)? {
                return Err(ParseError::CircularGroupDependency {
                    cycle: path.join(" -> "),
                });
            }
        }
        Ok(())
    }

    /// Recursively check for circular dependencies
    fn has_circular_dependency(
        inventory: &ParsedInventory,
        group_name: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<bool, ParseError> {
        if path.contains(&group_name.to_string()) {
            path.push(group_name.to_string());
            return Ok(true);
        }

        if visited.contains(group_name) {
            return Ok(false);
        }

        visited.insert(group_name.to_string());
        path.push(group_name.to_string());

        if let Some(group) = inventory.groups.get(group_name) {
            for child_name in &group.children {
                if Self::has_circular_dependency(inventory, child_name, visited, path)? {
                    return Ok(true);
                }
            }
        }

        path.pop();
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::template::TemplateEngine;

    fn create_test_parser() -> IniInventoryParser<'static> {
        use once_cell::sync::Lazy;
        static TEMPLATE_ENGINE: Lazy<TemplateEngine> = Lazy::new(TemplateEngine::new);
        static EXTRA_VARS: Lazy<HashMap<String, serde_json::Value>> = Lazy::new(HashMap::new);
        IniInventoryParser::new(&TEMPLATE_ENGINE, &EXTRA_VARS)
    }

    #[tokio::test]
    async fn test_simple_ini_parsing() {
        let ini_content = r#"
[webservers]
web1 ansible_host=192.168.1.10
web2 ansible_host=192.168.1.11

[webservers:vars]
http_port=80

[all]
web1
web2
"#;

        let parser = create_test_parser();
        let inventory = parser.parse_ini_inventory(ini_content).await.unwrap();

        assert_eq!(inventory.hosts.len(), 2);
        assert!(inventory.hosts.contains_key("web1"));
        assert!(inventory.hosts.contains_key("web2"));

        let webservers_group = inventory.groups.get("webservers").unwrap();
        assert_eq!(webservers_group.hosts.len(), 2);
        assert_eq!(
            webservers_group
                .vars
                .get("http_port")
                .unwrap()
                .as_u64()
                .unwrap(),
            80
        );
    }

    #[tokio::test]
    async fn test_host_pattern_expansion() {
        let ini_content = r#"
[webservers]
web[01:03] ansible_user=deploy

[all]
web01
web02
web03
"#;

        let parser = create_test_parser();
        let inventory = parser.parse_ini_inventory(ini_content).await.unwrap();

        assert_eq!(inventory.hosts.len(), 3);
        assert!(inventory.hosts.contains_key("web01"));
        assert!(inventory.hosts.contains_key("web02"));
        assert!(inventory.hosts.contains_key("web03"));
    }

    #[tokio::test]
    async fn test_group_children() {
        let ini_content = r#"
[webservers]
web1

[databases]
db1

[production:children]
webservers
databases

[production:vars]
env=production
"#;

        let parser = create_test_parser();
        let inventory = parser.parse_ini_inventory(ini_content).await.unwrap();

        let production_group = inventory.groups.get("production").unwrap();
        assert_eq!(production_group.children.len(), 2);
        assert!(production_group
            .children
            .contains(&"webservers".to_string()));
        assert!(production_group.children.contains(&"databases".to_string()));
        assert_eq!(
            production_group.vars.get("env").unwrap().as_str().unwrap(),
            "production"
        );
    }

    #[test]
    fn test_host_variable_parsing() {
        let parser = create_test_parser();
        let vars = parser
            .parse_host_variables(
                "ansible_host=192.168.1.10 ansible_port=22 custom_var='hello world'",
            )
            .unwrap();

        assert_eq!(vars.get("ansible_host").unwrap(), "192.168.1.10");
        assert_eq!(vars.get("ansible_port").unwrap(), "22");
        assert_eq!(vars.get("custom_var").unwrap(), "hello world");
    }

    #[test]
    fn test_ini_value_parsing() {
        let parser = create_test_parser();

        assert_eq!(
            parser.parse_ini_value("true"),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parser.parse_ini_value("false"),
            serde_json::Value::Bool(false)
        );
        assert_eq!(
            parser.parse_ini_value("42"),
            serde_json::Value::Number(serde_json::Number::from(42))
        );
        assert_eq!(
            parser.parse_ini_value("3.14"),
            serde_json::Value::Number(serde_json::Number::from_f64(3.14).unwrap())
        );
        assert_eq!(
            parser.parse_ini_value("\"hello\""),
            serde_json::Value::String("hello".to_string())
        );
    }
}
