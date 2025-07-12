use crate::parser::error::ParseError;
use crate::parser::inventory::ini::{IniInventoryParser, InventoryParserConfig};
use crate::parser::inventory::validation::InventoryValidator;
use crate::parser::inventory::variables::VariableInheritanceResolver;
use crate::parser::template::TemplateEngine;
use crate::types::parsed::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub struct InventoryParser<'a> {
    template_engine: &'a TemplateEngine,
    extra_vars: &'a HashMap<String, serde_json::Value>,
    config: InventoryParserConfig,
}

impl<'a> InventoryParser<'a> {
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

    pub async fn parse(&self, path: &Path) -> Result<ParsedInventory, ParseError> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ParseError::FileNotFound {
                    path: path.to_string_lossy().to_string(),
                }
            } else {
                ParseError::Io(e)
            }
        })?;

        // Detect format based on file extension and content
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension {
            "ini" => self.parse_ini_inventory(&content).await,
            "yml" | "yaml" => self.parse_yaml_inventory(&content).await,
            "json" => self.parse_json_inventory(&content).await,
            _ => {
                // Try to auto-detect format
                if content.trim_start().starts_with('{') {
                    self.parse_json_inventory(&content).await
                } else if content.contains("---") || content.trim_start().starts_with("all:") {
                    self.parse_yaml_inventory(&content).await
                } else if content.contains('[') && content.contains(']') {
                    // INI format sections like [webservers]
                    self.parse_ini_inventory(&content).await
                } else {
                    // Default to INI for simple host lists
                    self.parse_ini_inventory(&content).await
                }
            }
        }
    }

    async fn parse_ini_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError> {
        // Use the new comprehensive INI parser
        let ini_parser = IniInventoryParser::with_config(
            self.template_engine,
            self.extra_vars,
            self.config.clone(),
        );

        let mut inventory = ini_parser.parse_ini_inventory(content).await?;

        // Resolve variable inheritance
        VariableInheritanceResolver::resolve_group_inheritance(&mut inventory)?;

        // Validate the final inventory
        InventoryValidator::validate_inventory(&inventory)?;

        Ok(inventory)
    }

    /// Parse host patterns like web[01:05] into individual hosts
    pub fn expand_host_pattern(&self, pattern: &str) -> Result<Vec<String>, ParseError> {
        let ini_parser = IniInventoryParser::with_config(
            self.template_engine,
            self.extra_vars,
            self.config.clone(),
        );
        ini_parser.expand_host_pattern(pattern)
    }

    /// Parse inline host variables from inventory line
    pub fn parse_host_variables(
        &self,
        vars_str: &str,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError> {
        let ini_parser = IniInventoryParser::with_config(
            self.template_engine,
            self.extra_vars,
            self.config.clone(),
        );
        let raw_vars = ini_parser.parse_host_variables(vars_str)?;
        Ok(raw_vars
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect())
    }

    /// Resolve group inheritance and variable precedence
    pub fn resolve_group_inheritance(
        &self,
        inventory: &mut ParsedInventory,
    ) -> Result<(), ParseError> {
        VariableInheritanceResolver::resolve_group_inheritance(inventory)
    }

    /// Validate inventory structure and relationships
    pub fn validate_inventory(&self, inventory: &ParsedInventory) -> Result<(), ParseError> {
        InventoryValidator::validate_inventory(inventory)
    }

    async fn parse_yaml_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError> {
        let raw_inventory: RawYamlInventory = serde_yaml::from_str(content)?;

        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();
        let mut variables = self.extra_vars.clone();

        // Add global variables if present
        if let Some(all_group) = raw_inventory.all {
            if let Some(vars) = all_group.vars {
                variables.extend(vars);
            }
        }

        // Process each group
        for (group_name, group_data) in raw_inventory.groups.unwrap_or_default() {
            let mut parsed_hosts = Vec::new();
            let mut children = Vec::new();
            let group_vars = group_data.vars.unwrap_or_default();

            // Process hosts in this group
            if let Some(hosts_data) = group_data.hosts {
                for (hostname, host_data) in hosts_data {
                    let host_vars = host_data.unwrap_or_default();
                    let (address, port, user) = self.extract_connection_info(&host_vars);

                    let host = ParsedHost {
                        name: hostname.clone(),
                        address,
                        port,
                        user,
                        vars: host_vars,
                        groups: vec![group_name.clone()],
                    };

                    hosts.insert(hostname.clone(), host);
                    parsed_hosts.push(hostname);
                }
            }

            // Process children groups
            if let Some(children_data) = group_data.children {
                for child_name in children_data.keys() {
                    children.push(child_name.clone());
                }
            }

            groups.insert(
                group_name.clone(),
                ParsedGroup {
                    name: group_name,
                    hosts: parsed_hosts,
                    children,
                    vars: group_vars,
                },
            );
        }

        Ok(ParsedInventory {
            hosts,
            groups,
            variables,
        })
    }

    async fn parse_json_inventory(&self, content: &str) -> Result<ParsedInventory, ParseError> {
        let raw_inventory: serde_json::Value = serde_json::from_str(content)?;

        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();
        let variables = self.extra_vars.clone();

        if let serde_json::Value::Object(root) = raw_inventory {
            for (key, value) in root {
                if key == "_meta" {
                    // Handle meta section with hostvars
                    if let serde_json::Value::Object(meta) = value {
                        if let Some(serde_json::Value::Object(hostvars)) = meta.get("hostvars") {
                            for (hostname, host_vars) in hostvars {
                                if let serde_json::Value::Object(vars_obj) = host_vars {
                                    let vars_map: HashMap<String, serde_json::Value> = vars_obj
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect();
                                    let (address, port, user) =
                                        self.extract_connection_info(&vars_map);

                                    let host = ParsedHost {
                                        name: hostname.clone(),
                                        address,
                                        port,
                                        user,
                                        vars: vars_map,
                                        groups: Vec::new(), // Will be filled when processing groups
                                    };

                                    hosts.insert(hostname.clone(), host);
                                }
                            }
                        }
                    }
                } else {
                    // Handle group sections
                    if let serde_json::Value::Object(group_data) = value {
                        let group_hosts: Vec<String> = group_data
                            .get("hosts")
                            .and_then(|h| h.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let children: Vec<String> = group_data
                            .get("children")
                            .and_then(|c| c.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let group_vars: HashMap<String, serde_json::Value> = group_data
                            .get("vars")
                            .and_then(|v| v.as_object())
                            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                            .unwrap_or_default();

                        groups.insert(
                            key.clone(),
                            ParsedGroup {
                                name: key.clone(),
                                hosts: group_hosts.clone(),
                                children,
                                vars: group_vars,
                            },
                        );

                        // Update host group memberships
                        for hostname in group_hosts {
                            if let Some(host) = hosts.get_mut(&hostname) {
                                host.groups.push(key.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(ParsedInventory {
            hosts,
            groups,
            variables,
        })
    }

    #[allow(dead_code)]
    fn parse_ini_value(&self, value: &str) -> serde_json::Value {
        // Try to parse as different types
        if value.is_empty() {
            return serde_json::Value::String(value.to_string());
        }

        // Boolean
        match value.to_lowercase().as_str() {
            "true" | "yes" | "on" => return serde_json::Value::Bool(true),
            "false" | "no" | "off" => return serde_json::Value::Bool(false),
            _ => {}
        }

        // Number
        if let Ok(int_val) = value.parse::<i64>() {
            return serde_json::Value::Number(serde_json::Number::from(int_val));
        }
        if let Ok(float_val) = value.parse::<f64>() {
            return serde_json::Value::Number(serde_json::Number::from_f64(float_val).unwrap());
        }

        // String (default)
        serde_json::Value::String(value.to_string())
    }

    #[allow(dead_code)]
    fn parse_host_variables_internal(&self, vars_str: &str) -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();

        // Parse key=value pairs
        for pair in vars_str.split_whitespace() {
            if let Some((key, value)) = pair.split_once('=') {
                let parsed_value = self.parse_ini_value(value);
                vars.insert(key.to_string(), parsed_value);
            }
        }

        vars
    }

    fn extract_connection_info(
        &self,
        vars: &HashMap<String, serde_json::Value>,
    ) -> (Option<String>, Option<u16>, Option<String>) {
        let address = vars
            .get("ansible_host")
            .or_else(|| vars.get("ansible_ssh_host"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let port = vars
            .get("ansible_port")
            .or_else(|| vars.get("ansible_ssh_port"))
            .and_then(|v| v.as_u64())
            .map(|p| p as u16);

        let user = vars
            .get("ansible_user")
            .or_else(|| vars.get("ansible_ssh_user"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        (address, port, user)
    }
}

// Raw data structures for YAML inventory parsing
#[derive(Debug, Deserialize)]
struct RawYamlInventory {
    all: Option<RawGroup>,
    #[serde(flatten)]
    groups: Option<HashMap<String, RawGroup>>,
}

#[derive(Debug, Deserialize)]
struct RawGroup {
    hosts: Option<HashMap<String, Option<HashMap<String, serde_json::Value>>>>,
    children: Option<HashMap<String, RawGroup>>,
    vars: Option<HashMap<String, serde_json::Value>>,
}
