use crate::parser::error::ParseError;
use crate::parser::inventory::ini::{IniInventoryParser, InventoryParserConfig};
use crate::parser::inventory::validation::InventoryValidator;
use crate::parser::inventory::variables::VariableInheritanceResolver;
use crate::parser::template::TemplateEngine;
use crate::types::parsed::*;
use regex;
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

    /// Filter inventory based on a limit pattern (similar to Ansible's --limit)
    pub fn filter_inventory(
        &self,
        inventory: &mut ParsedInventory,
        limit_pattern: &str,
    ) -> Result<(), ParseError> {
        // Parse comma-separated patterns
        let patterns: Vec<&str> = limit_pattern.split(',').map(|s| s.trim()).collect();

        let mut matching_hosts = std::collections::HashSet::new();

        for pattern in patterns {
            // Check if pattern is a group name (prefixed with ':' or just a group name)
            let is_group_pattern = pattern.starts_with(':');
            let pattern_name = if is_group_pattern {
                &pattern[1..]
            } else {
                pattern
            };

            if is_group_pattern || inventory.groups.contains_key(pattern_name) {
                // It's a group pattern - add all hosts from the group
                if let Some(group) = inventory.groups.get(pattern_name) {
                    for host in &group.hosts {
                        matching_hosts.insert(host.clone());
                    }
                    // Also add hosts from child groups
                    Self::collect_hosts_from_children(
                        &inventory.groups,
                        pattern_name,
                        &mut matching_hosts,
                    );
                }
            } else if pattern.contains('[') && pattern.contains(']') {
                // It's a host pattern with ranges - expand it
                match self.expand_host_pattern(pattern) {
                    Ok(expanded_hosts) => {
                        for host in expanded_hosts {
                            if inventory.hosts.contains_key(&host) {
                                matching_hosts.insert(host);
                            }
                        }
                    }
                    Err(_) => {
                        // If pattern expansion fails, treat as literal
                        if inventory.hosts.contains_key(pattern) {
                            matching_hosts.insert(pattern.to_string());
                        }
                    }
                }
            } else {
                // It's a literal host name or glob pattern
                if pattern.contains('*') || pattern.contains('?') {
                    // Simple glob pattern matching
                    let regex_pattern = pattern.replace("*", ".*").replace("?", ".");
                    if let Ok(re) = regex::Regex::new(&format!("^{regex_pattern}$")) {
                        for host_name in inventory.hosts.keys() {
                            if re.is_match(host_name) {
                                matching_hosts.insert(host_name.clone());
                            }
                        }
                    }
                } else {
                    // Literal host name
                    if inventory.hosts.contains_key(pattern) {
                        matching_hosts.insert(pattern.to_string());
                    }
                }
            }
        }

        // Filter hosts to only include matching ones
        inventory
            .hosts
            .retain(|host_name, _| matching_hosts.contains(host_name));

        // Update groups to only include hosts that still exist
        for group in inventory.groups.values_mut() {
            group.hosts.retain(|host| matching_hosts.contains(host));
        }

        // Remove empty groups (except 'all')
        inventory.groups.retain(|name, group| {
            name == "all" || !group.hosts.is_empty() || !group.children.is_empty()
        });

        // Update the 'all' group to reflect the filtered hosts
        if let Some(all_group) = inventory.groups.get_mut("all") {
            all_group.hosts = matching_hosts.into_iter().collect();
            all_group.hosts.sort();
        }

        Ok(())
    }

    /// Recursively collect hosts from child groups
    fn collect_hosts_from_children(
        groups: &HashMap<String, ParsedGroup>,
        group_name: &str,
        matching_hosts: &mut std::collections::HashSet<String>,
    ) {
        if let Some(group) = groups.get(group_name) {
            for child_name in &group.children {
                if let Some(child_group) = groups.get(child_name) {
                    for host in &child_group.hosts {
                        matching_hosts.insert(host.clone());
                    }
                    // Recurse into child's children
                    Self::collect_hosts_from_children(groups, child_name, matching_hosts);
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

        // Ensure 'all' group exists and contains all hosts
        Self::ensure_all_group(&mut inventory);

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
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(content)?;

        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();
        let mut variables = self.extra_vars.clone();

        // Handle both formats:
        // 1. Standard Ansible inventory with groups at top level
        // 2. Inventory with 'all' group containing everything

        if let serde_yaml::Value::Mapping(root) = yaml_value {
            for (key, value) in root {
                if let Some(group_name) = key.as_str() {
                    if group_name == "all" {
                        // Handle 'all' group specially for global variables
                        if let serde_yaml::Value::Mapping(ref all_data) = value {
                            if let Some(vars_value) =
                                all_data.get(serde_yaml::Value::String("vars".to_string()))
                            {
                                if let Ok(vars) =
                                    serde_yaml::from_value::<HashMap<String, serde_json::Value>>(
                                        vars_value.clone(),
                                    )
                                {
                                    variables.extend(vars);
                                }
                            }
                            // Also process 'all' as a regular group if it has hosts or children
                            if all_data.contains_key(serde_yaml::Value::String("hosts".to_string()))
                                || all_data
                                    .contains_key(serde_yaml::Value::String("children".to_string()))
                            {
                                self.process_group(group_name, &value, &mut hosts, &mut groups)?;
                            }
                        }
                    } else {
                        // Process regular groups
                        self.process_group(group_name, &value, &mut hosts, &mut groups)?;
                    }
                }
            }
        }

        let mut inventory = ParsedInventory {
            hosts,
            groups,
            variables,
        };

        // Ensure 'all' group exists and contains all hosts
        Self::ensure_all_group(&mut inventory);

        // Resolve variable inheritance
        VariableInheritanceResolver::resolve_group_inheritance(&mut inventory)?;

        // Validate the final inventory
        InventoryValidator::validate_inventory(&inventory)?;

        Ok(inventory)
    }

    fn process_group(
        &self,
        group_name: &str,
        group_value: &serde_yaml::Value,
        hosts: &mut HashMap<String, ParsedHost>,
        groups: &mut HashMap<String, ParsedGroup>,
    ) -> Result<(), ParseError> {
        if let serde_yaml::Value::Mapping(group_data) = group_value {
            let mut parsed_hosts = Vec::new();
            let mut children = Vec::new();
            let mut group_vars = HashMap::new();

            // Process hosts in this group
            if let Some(serde_yaml::Value::Mapping(hosts_data)) =
                group_data.get(serde_yaml::Value::String("hosts".to_string()))
            {
                for (hostname_val, host_data_val) in hosts_data {
                    if let Some(hostname) = hostname_val.as_str() {
                        let host_vars = match host_data_val {
                            serde_yaml::Value::Mapping(_) => {
                                serde_yaml::from_value::<HashMap<String, serde_json::Value>>(
                                    host_data_val.clone(),
                                )
                                .unwrap_or_default()
                            }
                            serde_yaml::Value::Null => HashMap::new(),
                            _ => HashMap::new(),
                        };

                        let conn = self.extract_connection_info(&host_vars);

                        let host = ParsedHost {
                            name: hostname.to_string(),
                            address: conn.address,
                            port: conn.port,
                            user: conn.user,
                            vars: host_vars,
                            groups: vec![group_name.to_string()],
                            connection: conn.connection,
                            ssh_private_key_file: conn.ssh_private_key_file,
                            ssh_common_args: conn.ssh_common_args,
                            ssh_extra_args: conn.ssh_extra_args,
                            ssh_pipelining: conn.ssh_pipelining,
                            connection_timeout: conn.connection_timeout,
                            ansible_become: conn.ansible_become,
                            become_method: conn.become_method,
                            become_user: conn.become_user,
                            become_flags: conn.become_flags,
                        };

                        hosts.insert(hostname.to_string(), host);
                        parsed_hosts.push(hostname.to_string());
                    }
                }
            }

            // Process children groups
            if let Some(serde_yaml::Value::Mapping(children_data)) =
                group_data.get(serde_yaml::Value::String("children".to_string()))
            {
                for (child_name_val, child_data_val) in children_data {
                    if let Some(child_name) = child_name_val.as_str() {
                        children.push(child_name.to_string());
                        // Recursively process child group
                        self.process_group(child_name, child_data_val, hosts, groups)?;
                    }
                }
            }

            // Process group variables
            if let Some(vars_value) = group_data.get(serde_yaml::Value::String("vars".to_string()))
            {
                group_vars = serde_yaml::from_value::<HashMap<String, serde_json::Value>>(
                    vars_value.clone(),
                )
                .unwrap_or_default();
            }

            groups.insert(
                group_name.to_string(),
                ParsedGroup {
                    name: group_name.to_string(),
                    hosts: parsed_hosts,
                    children,
                    vars: group_vars,
                },
            );
        }

        Ok(())
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
                                    let conn = self.extract_connection_info(&vars_map);

                                    let host = ParsedHost {
                                        name: hostname.clone(),
                                        address: conn.address,
                                        port: conn.port,
                                        user: conn.user,
                                        vars: vars_map,
                                        groups: Vec::new(), // Will be filled when processing groups
                                        connection: conn.connection,
                                        ssh_private_key_file: conn.ssh_private_key_file,
                                        ssh_common_args: conn.ssh_common_args,
                                        ssh_extra_args: conn.ssh_extra_args,
                                        ssh_pipelining: conn.ssh_pipelining,
                                        connection_timeout: conn.connection_timeout,
                                        ansible_become: conn.ansible_become,
                                        become_method: conn.become_method,
                                        become_user: conn.become_user,
                                        become_flags: conn.become_flags,
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

                        // Update host group memberships and create hosts if they don't exist
                        for hostname in group_hosts {
                            if let Some(host) = hosts.get_mut(&hostname) {
                                host.groups.push(key.clone());
                            } else {
                                // Create host if it doesn't exist
                                let host = ParsedHost {
                                    name: hostname.clone(),
                                    address: None,
                                    port: None,
                                    user: None,
                                    vars: HashMap::new(),
                                    groups: vec![key.clone()],
                                    connection: None,
                                    ssh_private_key_file: None,
                                    ssh_common_args: None,
                                    ssh_extra_args: None,
                                    ssh_pipelining: None,
                                    connection_timeout: None,
                                    ansible_become: None,
                                    become_method: None,
                                    become_user: None,
                                    become_flags: None,
                                };
                                hosts.insert(hostname.clone(), host);
                            }
                        }
                    }
                }
            }
        }

        let mut inventory = ParsedInventory {
            hosts,
            groups,
            variables,
        };

        // Ensure 'all' group exists and contains all hosts
        Self::ensure_all_group(&mut inventory);

        // Resolve variable inheritance
        VariableInheritanceResolver::resolve_group_inheritance(&mut inventory)?;

        // Validate the final inventory
        InventoryValidator::validate_inventory(&inventory)?;

        Ok(inventory)
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

    /// Ensure the 'all' group exists and contains all hosts
    fn ensure_all_group(inventory: &mut ParsedInventory) {
        let all_host_names: Vec<String> = inventory.hosts.keys().cloned().collect();

        // Add all hosts to the 'all' group membership
        for host in inventory.hosts.values_mut() {
            if !host.groups.contains(&"all".to_string()) {
                host.groups.push("all".to_string());
            }
        }

        // Create or update the 'all' group
        if let Some(all_group) = inventory.groups.get_mut("all") {
            // Update existing 'all' group to ensure it contains all hosts
            for host_name in &all_host_names {
                if !all_group.hosts.contains(host_name) {
                    all_group.hosts.push(host_name.clone());
                }
            }
        } else {
            // Create the 'all' group
            use crate::types::parsed::ParsedGroup;
            inventory.groups.insert(
                "all".to_string(),
                ParsedGroup {
                    name: "all".to_string(),
                    hosts: all_host_names,
                    children: Vec::new(),
                    vars: HashMap::new(),
                },
            );
        }
    }

    fn extract_connection_info(
        &self,
        vars: &HashMap<String, serde_json::Value>,
    ) -> ParsedHostConnection {
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
            .or_else(|| vars.get("ansible_ssh_user_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Connection settings
        let connection = vars
            .get("ansible_connection")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssh_private_key_file = vars
            .get("ansible_ssh_private_key_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssh_common_args = vars
            .get("ansible_ssh_common_args")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssh_extra_args = vars
            .get("ansible_ssh_extra_args")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let ssh_pipelining = vars.get("ansible_ssh_pipelining").and_then(|v| v.as_bool());

        let connection_timeout = vars
            .get("ansible_timeout")
            .or_else(|| vars.get("ansible_connection_timeout"))
            .and_then(|v| v.as_u64())
            .map(|t| t as u32);

        // Privilege escalation
        let ansible_become = vars.get("ansible_become").and_then(|v| v.as_bool());

        let become_method = vars
            .get("ansible_become_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let become_user = vars
            .get("ansible_become_user")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let become_flags = vars
            .get("ansible_become_flags")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        ParsedHostConnection {
            address,
            port,
            user,
            connection,
            ssh_private_key_file,
            ssh_common_args,
            ssh_extra_args,
            ssh_pipelining,
            connection_timeout,
            ansible_become,
            become_method,
            become_user,
            become_flags,
        }
    }
}

// Raw data structures for YAML inventory parsing
#[derive(Debug, Deserialize)]
struct RawYamlInventory {
    #[allow(dead_code)]
    all: Option<RawGroup>,
    #[serde(flatten)]
    #[allow(dead_code)]
    groups: Option<HashMap<String, RawGroup>>,
}

#[derive(Debug, Deserialize)]
struct RawGroup {
    #[allow(dead_code)]
    hosts: Option<HashMap<String, Option<HashMap<String, serde_json::Value>>>>,
    #[allow(dead_code)]
    children: Option<HashMap<String, RawGroup>>,
    #[allow(dead_code)]
    vars: Option<HashMap<String, serde_json::Value>>,
}
