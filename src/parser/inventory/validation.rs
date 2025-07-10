use crate::parser::error::ParseError;
use crate::parser::inventory::variables::VariableInheritanceResolver;
use crate::types::parsed::{ParsedHost, ParsedInventory};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Comprehensive inventory validation
pub struct InventoryValidator;

impl InventoryValidator {
    /// Validate entire inventory structure
    pub fn validate_inventory(inventory: &ParsedInventory) -> Result<(), ParseError> {
        Self::validate_basic_structure(inventory)?;
        Self::validate_host_references(inventory)?;
        Self::validate_group_references(inventory)?;
        Self::validate_variable_names(inventory)?;
        Self::validate_connection_parameters(inventory)?;
        VariableInheritanceResolver::validate_variable_inheritance(inventory)?;
        Ok(())
    }

    /// Validate basic inventory structure and consistency
    fn validate_basic_structure(inventory: &ParsedInventory) -> Result<(), ParseError> {
        // Ensure all groups exist
        if inventory.groups.is_empty() {
            return Err(ParseError::InvalidStructure {
                message: "Inventory must contain at least one group".to_string(),
            });
        }

        // Ensure 'all' group exists
        if !inventory.groups.contains_key("all") {
            return Err(ParseError::InvalidStructure {
                message: "Inventory must contain an 'all' group".to_string(),
            });
        }

        // Validate that each host belongs to at least one group
        for (host_name, host) in &inventory.hosts {
            if host.groups.is_empty() {
                return Err(ParseError::InvalidStructure {
                    message: format!("Host '{}' does not belong to any groups", host_name),
                });
            }

            // All hosts must be in the 'all' group
            if !host.groups.contains(&"all".to_string()) {
                return Err(ParseError::InvalidStructure {
                    message: format!("Host '{}' is not in the 'all' group", host_name),
                });
            }
        }

        // Validate group name uniqueness
        let mut seen_groups = HashSet::new();
        for group_name in inventory.groups.keys() {
            if !seen_groups.insert(group_name.clone()) {
                return Err(ParseError::InvalidStructure {
                    message: format!("Duplicate group name: '{}'", group_name),
                });
            }
        }

        // Validate host name uniqueness
        let mut seen_hosts = HashSet::new();
        for host_name in inventory.hosts.keys() {
            if !seen_hosts.insert(host_name.clone()) {
                return Err(ParseError::DuplicateHost {
                    host: host_name.clone(),
                });
            }
        }

        Ok(())
    }

    /// Validate that all host references in groups are valid
    fn validate_host_references(inventory: &ParsedInventory) -> Result<(), ParseError> {
        for (group_name, group) in &inventory.groups {
            for host_name in &group.hosts {
                if !inventory.hosts.contains_key(host_name) {
                    return Err(ParseError::InvalidStructure {
                        message: format!(
                            "Group '{}' references non-existent host '{}'",
                            group_name, host_name
                        ),
                    });
                }
            }
        }

        // Validate that each host's group membership is consistent
        for (host_name, host) in &inventory.hosts {
            for group_name in &host.groups {
                if let Some(group) = inventory.groups.get(group_name) {
                    if !group.hosts.contains(host_name) {
                        return Err(ParseError::InvalidStructure {
                            message: format!(
                                "Host '{}' claims membership in group '{}' but group doesn't list the host", 
                                host_name, group_name
                            ),
                        });
                    }
                } else {
                    return Err(ParseError::UnknownGroup {
                        group: group_name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate that all group references (children) are valid
    fn validate_group_references(inventory: &ParsedInventory) -> Result<(), ParseError> {
        for (group_name, group) in &inventory.groups {
            for child_name in &group.children {
                if !inventory.groups.contains_key(child_name) {
                    return Err(ParseError::UnknownGroup {
                        group: child_name.clone(),
                    });
                }

                // Prevent self-reference
                if child_name == group_name {
                    return Err(ParseError::CircularGroupDependency {
                        cycle: format!("{} -> {}", group_name, child_name),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate variable names and values
    fn validate_variable_names(inventory: &ParsedInventory) -> Result<(), ParseError> {
        // Validate global variables
        Self::validate_variable_dict(&inventory.variables, "global")?;

        // Validate group variables
        for (group_name, group) in &inventory.groups {
            Self::validate_variable_dict(&group.vars, &format!("group '{}'", group_name))?;
        }

        // Validate host variables
        for (host_name, host) in &inventory.hosts {
            Self::validate_variable_dict(&host.vars, &format!("host '{}'", host_name))?;
        }

        Ok(())
    }

    /// Validate a dictionary of variables
    fn validate_variable_dict(
        vars: &HashMap<String, serde_json::Value>,
        context: &str,
    ) -> Result<(), ParseError> {
        for (var_name, var_value) in vars {
            Self::validate_variable_name(var_name, context)?;
            Self::validate_variable_value(var_value, var_name, context)?;
        }
        Ok(())
    }

    /// Validate a single variable name
    fn validate_variable_name(name: &str, context: &str) -> Result<(), ParseError> {
        // Check length limits
        const MAX_VARIABLE_NAME_LENGTH: usize = 256;
        if name.len() > MAX_VARIABLE_NAME_LENGTH {
            return Err(ParseError::InvalidVariableSyntax {
                line: 0,
                message: format!(
                    "Variable name '{}' in {} exceeds maximum length of {} characters",
                    name, context, MAX_VARIABLE_NAME_LENGTH
                ),
            });
        }

        // Check for valid variable name pattern
        if !VALID_VARIABLE_NAME.is_match(name) {
            return Err(ParseError::InvalidVariableSyntax {
                line: 0,
                message: format!(
                    "Invalid variable name '{}' in {}. Variable names must start with a letter or underscore and contain only letters, numbers, and underscores",
                    name, context
                ),
            });
        }

        // Check for reserved variable names
        if RESERVED_VARIABLE_NAMES.contains(&name) {
            return Err(ParseError::InvalidVariableSyntax {
                line: 0,
                message: format!(
                    "Variable name '{}' in {} is reserved and cannot be used",
                    name, context
                ),
            });
        }

        // Warn about potentially problematic variable names
        if PROBLEMATIC_VARIABLE_NAMES.contains(&name) {
            // For now, we'll allow these but could add warnings in the future
            // In strict mode, we might want to reject these
        }

        Ok(())
    }

    /// Validate a single variable value
    fn validate_variable_value(
        value: &serde_json::Value,
        var_name: &str,
        context: &str,
    ) -> Result<(), ParseError> {
        // Check value size limits
        const MAX_VARIABLE_VALUE_LENGTH: usize = 4096;

        let value_string = match value {
            serde_json::Value::String(s) => s.clone(),
            _ => serde_json::to_string(value).unwrap_or_default(),
        };

        if value_string.len() > MAX_VARIABLE_VALUE_LENGTH {
            return Err(ParseError::InvalidVariableSyntax {
                line: 0,
                message: format!(
                    "Variable '{}' in {} has value exceeding maximum length of {} characters",
                    var_name, context, MAX_VARIABLE_VALUE_LENGTH
                ),
            });
        }

        // Check for potentially dangerous values (basic security check)
        if let serde_json::Value::String(s) = value {
            if Self::contains_potentially_dangerous_content(s) {
                return Err(ParseError::InvalidVariableSyntax {
                    line: 0,
                    message: format!(
                        "Variable '{}' in {} contains potentially dangerous content",
                        var_name, context
                    ),
                });
            }
        }

        Ok(())
    }

    /// Check for potentially dangerous content in variable values
    fn contains_potentially_dangerous_content(value: &str) -> bool {
        // Check for command injection patterns
        let dangerous_patterns = [
            "$(",
            "`",
            "&",
            "|",
            ";",
            "\n",
            "\r",
            "../",
            "/..",
            "file://",
            "ftp://",
            "<script",
            "javascript:",
            "data:",
        ];

        for pattern in &dangerous_patterns {
            if value.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Validate connection parameters for all hosts
    fn validate_connection_parameters(inventory: &ParsedInventory) -> Result<(), ParseError> {
        for (host_name, host) in &inventory.hosts {
            Self::validate_host_connection_parameters(host, host_name)?;
        }
        Ok(())
    }

    /// Validate connection parameters for a single host
    fn validate_host_connection_parameters(
        host: &ParsedHost,
        host_name: &str,
    ) -> Result<(), ParseError> {
        // Validate ansible_host (IP address or hostname)
        if let Some(address) = &host.address {
            if !Self::is_valid_hostname_or_ip(address) {
                return Err(ParseError::InvalidStructure {
                    message: format!(
                        "Host '{}' has invalid ansible_host address: '{}'",
                        host_name, address
                    ),
                });
            }
        }

        // Validate ansible_port
        if let Some(port) = host.port {
            #[allow(unused_comparisons)]
            if port == 0 || port > 65535 {
                return Err(ParseError::InvalidStructure {
                    message: format!("Host '{}' has invalid port number: {}", host_name, port),
                });
            }
        }

        // Validate ansible_user
        if let Some(user) = &host.user {
            if user.is_empty() || user.len() > 32 {
                return Err(ParseError::InvalidStructure {
                    message: format!("Host '{}' has invalid username: '{}'", host_name, user),
                });
            }

            if !VALID_USERNAME.is_match(user) {
                return Err(ParseError::InvalidStructure {
                    message: format!(
                        "Host '{}' has invalid username format: '{}'",
                        host_name, user
                    ),
                });
            }
        }

        Ok(())
    }

    /// Check if a string is a valid hostname or IP address
    fn is_valid_hostname_or_ip(address: &str) -> bool {
        // Check for valid IP address (basic validation)
        if VALID_IPV4.is_match(address) {
            return true;
        }

        // Check for valid hostname
        if VALID_HOSTNAME.is_match(address) && address.len() <= 253 {
            return true;
        }

        false
    }

    /// Perform additional validation checks
    pub fn validate_inventory_with_config(
        inventory: &ParsedInventory,
        strict_mode: bool,
    ) -> Result<Vec<String>, ParseError> {
        let mut warnings = Vec::new();

        // Basic validation always runs
        Self::validate_inventory(inventory)?;

        if strict_mode {
            // Additional strict mode validations
            warnings.extend(Self::check_for_warnings(inventory)?);
        }

        Ok(warnings)
    }

    /// Check for potential issues that generate warnings
    fn check_for_warnings(inventory: &ParsedInventory) -> Result<Vec<String>, ParseError> {
        let mut warnings = Vec::new();

        // Check for empty groups
        for (group_name, group) in &inventory.groups {
            if group.hosts.is_empty() && group.children.is_empty() && group_name != "all" {
                warnings.push(format!(
                    "Group '{}' is empty (no hosts or children)",
                    group_name
                ));
            }
        }

        // Check for hosts with no connection information
        for (host_name, host) in &inventory.hosts {
            if host.address.is_none() && host_name != "localhost" {
                warnings.push(format!(
                    "Host '{}' has no ansible_host address specified",
                    host_name
                ));
            }
        }

        // Check for unused variables
        let used_vars = Self::collect_used_variable_names(inventory);
        for var_name in inventory.variables.keys() {
            if !used_vars.contains(var_name) {
                warnings.push(format!(
                    "Global variable '{}' appears to be unused",
                    var_name
                ));
            }
        }

        Ok(warnings)
    }

    /// Collect all variable names that appear to be used in templates or references
    fn collect_used_variable_names(inventory: &ParsedInventory) -> HashSet<String> {
        let mut used_vars = HashSet::new();

        // This is a simplified implementation - in practice, you'd want to
        // parse template expressions to find variable references
        for host in inventory.hosts.values() {
            for value in host.vars.values() {
                if let serde_json::Value::String(s) = value {
                    Self::extract_variable_references(s, &mut used_vars);
                }
            }
        }

        for group in inventory.groups.values() {
            for value in group.vars.values() {
                if let serde_json::Value::String(s) = value {
                    Self::extract_variable_references(s, &mut used_vars);
                }
            }
        }

        used_vars
    }

    /// Extract variable references from template strings
    fn extract_variable_references(template: &str, used_vars: &mut HashSet<String>) {
        // Simple regex to find {{ variable_name }} patterns
        for captures in TEMPLATE_VAR_PATTERN.captures_iter(template) {
            if let Some(var_name) = captures.get(1) {
                used_vars.insert(var_name.as_str().to_string());
            }
        }
    }
}

// Compiled regex patterns for validation
static VALID_VARIABLE_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());

static VALID_HOSTNAME: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$").unwrap()
});

static VALID_IPV4: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$").unwrap()
});

static VALID_USERNAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-z_]([a-z0-9_-]{0,31}|[a-z0-9_-]{0,30}\$)$").unwrap());

static TEMPLATE_VAR_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap());

// Reserved variable names that cannot be used
static RESERVED_VARIABLE_NAMES: &[&str] = &[
    "inventory_hostname",
    "inventory_hostname_short",
    "inventory_file",
    "inventory_dir",
    "groups",
    "group_names",
    "hostvars",
    "playbook_dir",
    "role_path",
    "ansible_facts",
    "ansible_version",
];

// Variable names that might cause issues
static PROBLEMATIC_VARIABLE_NAMES: &[&str] = &[
    "host",
    "hostname",
    "item",
    "result",
    "changed",
    "failed",
    "skipped",
    "ok",
    "unreachable",
    "rescued",
    "ignored",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::parsed::{ParsedGroup, ParsedHost};

    fn create_valid_inventory() -> ParsedInventory {
        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();

        hosts.insert(
            "web1".to_string(),
            ParsedHost {
                name: "web1".to_string(),
                address: Some("192.168.1.10".to_string()),
                port: Some(22),
                user: Some("deploy".to_string()),
                vars: {
                    let mut vars = HashMap::new();
                    vars.insert(
                        "valid_var".to_string(),
                        serde_json::Value::String("value".to_string()),
                    );
                    vars
                },
                groups: vec!["webservers".to_string(), "all".to_string()],
            },
        );

        groups.insert(
            "webservers".to_string(),
            ParsedGroup {
                name: "webservers".to_string(),
                hosts: vec!["web1".to_string()],
                children: Vec::new(),
                vars: HashMap::new(),
            },
        );

        groups.insert(
            "all".to_string(),
            ParsedGroup {
                name: "all".to_string(),
                hosts: vec!["web1".to_string()],
                children: Vec::new(),
                vars: HashMap::new(),
            },
        );

        ParsedInventory {
            hosts,
            groups,
            variables: HashMap::new(),
        }
    }

    #[test]
    fn test_valid_inventory_passes_validation() {
        let inventory = create_valid_inventory();
        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_all_group_fails_validation() {
        let mut inventory = create_valid_inventory();
        inventory.groups.remove("all");

        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidStructure { .. }
        ));
    }

    #[test]
    fn test_invalid_variable_name_fails_validation() {
        let mut inventory = create_valid_inventory();

        // Add host with invalid variable name
        inventory.hosts.get_mut("web1").unwrap().vars.insert(
            "123invalid".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidVariableSyntax { .. }
        ));
    }

    #[test]
    fn test_circular_group_dependency_fails_validation() {
        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: HashMap::new(),
        };

        // Create circular dependency
        inventory.groups.insert(
            "group1".to_string(),
            ParsedGroup {
                name: "group1".to_string(),
                hosts: Vec::new(),
                children: vec!["group2".to_string()],
                vars: HashMap::new(),
            },
        );

        inventory.groups.insert(
            "group2".to_string(),
            ParsedGroup {
                name: "group2".to_string(),
                hosts: Vec::new(),
                children: vec!["group1".to_string()],
                vars: HashMap::new(),
            },
        );

        inventory.groups.insert(
            "all".to_string(),
            ParsedGroup {
                name: "all".to_string(),
                hosts: Vec::new(),
                children: Vec::new(),
                vars: HashMap::new(),
            },
        );

        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::CircularGroupDependency { .. }
        ));
    }

    #[test]
    fn test_invalid_hostname_fails_validation() {
        let mut inventory = create_valid_inventory();
        inventory.hosts.get_mut("web1").unwrap().address = Some("invalid..hostname..".to_string());

        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidStructure { .. }
        ));
    }

    #[test]
    fn test_invalid_port_fails_validation() {
        let mut inventory = create_valid_inventory();
        inventory.hosts.get_mut("web1").unwrap().port = Some(0);

        let result = InventoryValidator::validate_inventory(&inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidStructure { .. }
        ));
    }

    #[test]
    fn test_valid_variable_names() {
        assert!(InventoryValidator::validate_variable_name("valid_var", "test").is_ok());
        assert!(InventoryValidator::validate_variable_name("_private_var", "test").is_ok());
        assert!(InventoryValidator::validate_variable_name("var123", "test").is_ok());
    }

    #[test]
    fn test_invalid_variable_names() {
        assert!(InventoryValidator::validate_variable_name("123invalid", "test").is_err());
        assert!(InventoryValidator::validate_variable_name("var-with-dash", "test").is_err());
        assert!(InventoryValidator::validate_variable_name("var with space", "test").is_err());
        assert!(InventoryValidator::validate_variable_name("inventory_hostname", "test").is_err());
    }

    #[test]
    fn test_hostname_validation() {
        assert!(InventoryValidator::is_valid_hostname_or_ip("example.com"));
        assert!(InventoryValidator::is_valid_hostname_or_ip("192.168.1.1"));
        assert!(InventoryValidator::is_valid_hostname_or_ip(
            "web-server-01.example.com"
        ));
        assert!(!InventoryValidator::is_valid_hostname_or_ip(
            "invalid..hostname"
        ));
        assert!(!InventoryValidator::is_valid_hostname_or_ip("256.1.1.1"));
    }
}
