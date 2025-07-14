use crate::parser::error::ParseError;
use crate::types::parsed::{ParsedHost, ParsedInventory};
use petgraph::{Direction, Graph};
use std::collections::{HashMap, HashSet};

/// Type alias for group dependency graph structure
type GroupGraph = (
    Graph<String, ()>,
    HashMap<String, petgraph::graph::NodeIndex>,
);

/// Variable inheritance resolver for inventory
pub struct VariableInheritanceResolver;

impl VariableInheritanceResolver {
    /// Resolve group inheritance and variable precedence for the entire inventory
    pub fn resolve_group_inheritance(inventory: &mut ParsedInventory) -> Result<(), ParseError> {
        // Build group dependency graph
        let (graph, group_indices) = Self::build_group_graph(inventory)?;

        // Check for circular dependencies
        if petgraph::algo::is_cyclic_directed(&graph) {
            let cycle = Self::find_cycle_description(&graph, &group_indices)?;
            return Err(ParseError::CircularGroupDependency { cycle });
        }

        // Get topological ordering of groups
        let topo_order = petgraph::algo::toposort(&graph, None).map_err(|_| {
            ParseError::CircularGroupDependency {
                cycle: "Complex cycle detected in group hierarchy".to_string(),
            }
        })?;

        // Apply group variables to hosts in dependency order (children first, then parents)
        for node_index in topo_order.iter().rev() {
            let group_name = &graph[*node_index];
            Self::apply_group_variables_to_hosts(inventory, group_name)?;
        }

        // Apply variables from parent groups to child groups
        for node_index in topo_order.iter().rev() {
            let group_name = &graph[*node_index];
            Self::inherit_variables_from_children(inventory, group_name)?;
        }

        Ok(())
    }

    /// Build a directed graph representing group dependencies
    fn build_group_graph(inventory: &ParsedInventory) -> Result<GroupGraph, ParseError> {
        let mut graph = Graph::new();
        let mut group_indices = HashMap::new();

        // Add all groups as nodes
        for group_name in inventory.groups.keys() {
            let index = graph.add_node(group_name.clone());
            group_indices.insert(group_name.clone(), index);
        }

        // Add edges for parent -> child relationships
        for (group_name, group) in &inventory.groups {
            let parent_index = group_indices[group_name];
            for child_name in &group.children {
                if let Some(&child_index) = group_indices.get(child_name) {
                    graph.add_edge(parent_index, child_index, ());
                } else {
                    // Child group doesn't exist - this should be caught in validation
                    continue;
                }
            }
        }

        Ok((graph, group_indices))
    }

    /// Find and describe a cycle in the group dependency graph
    fn find_cycle_description(
        graph: &Graph<String, ()>,
        group_indices: &HashMap<String, petgraph::graph::NodeIndex>,
    ) -> Result<String, ParseError> {
        // Simple cycle detection - find the first cycle we encounter
        for &start_index in group_indices.values() {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            if Self::find_cycle_from_node(graph, start_index, start_index, &mut visited, &mut path)
            {
                return Ok(path.join(" -> "));
            }
        }

        Ok("Cycle detected but path unknown".to_string())
    }

    /// Recursively find a cycle starting from a specific node
    fn find_cycle_from_node(
        graph: &Graph<String, ()>,
        current: petgraph::graph::NodeIndex,
        target: petgraph::graph::NodeIndex,
        visited: &mut HashSet<petgraph::graph::NodeIndex>,
        path: &mut Vec<String>,
    ) -> bool {
        if visited.contains(&current) {
            return current == target && path.len() > 1;
        }

        visited.insert(current);
        path.push(graph[current].clone());

        for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
            if Self::find_cycle_from_node(graph, neighbor, target, visited, path) {
                return true;
            }
        }

        path.pop();
        false
    }

    /// Apply group variables to all hosts in the group
    fn apply_group_variables_to_hosts(
        inventory: &mut ParsedInventory,
        group_name: &str,
    ) -> Result<(), ParseError> {
        let group = match inventory.groups.get(group_name) {
            Some(g) => g.clone(),  // Clone to avoid borrow checker issues
            None => return Ok(()), // Group doesn't exist, skip
        };

        // Apply group variables to direct member hosts
        for host_name in &group.hosts {
            if let Some(host) = inventory.hosts.get_mut(host_name) {
                Self::apply_group_vars_to_host(host, &group.vars);
            }
        }

        // Apply group variables to hosts in child groups (recursive inheritance)
        for child_group_name in &group.children {
            Self::apply_group_variables_to_child_group_hosts(
                inventory,
                child_group_name,
                &group.vars,
            )?;
        }

        Ok(())
    }

    /// Apply group variables to a single host with proper precedence
    fn apply_group_vars_to_host(
        host: &mut ParsedHost,
        group_vars: &HashMap<String, serde_json::Value>,
    ) {
        for (key, value) in group_vars {
            // Group variables have lower precedence than host variables
            // Only set if the host doesn't already have this variable
            host.vars
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
    }

    /// Recursively apply group variables to hosts in child groups
    fn apply_group_variables_to_child_group_hosts(
        inventory: &mut ParsedInventory,
        child_group_name: &str,
        parent_vars: &HashMap<String, serde_json::Value>,
    ) -> Result<(), ParseError> {
        let child_group = match inventory.groups.get(child_group_name) {
            Some(g) => g.clone(),
            None => return Ok(()),
        };

        // Apply to direct hosts in child group
        for host_name in &child_group.hosts {
            if let Some(host) = inventory.hosts.get_mut(host_name) {
                Self::apply_group_vars_to_host(host, parent_vars);
            }
        }

        // Recursively apply to grandchild groups
        for grandchild_name in &child_group.children {
            Self::apply_group_variables_to_child_group_hosts(
                inventory,
                grandchild_name,
                parent_vars,
            )?;
        }

        Ok(())
    }

    /// Inherit variables from child groups (child group vars override parent group vars)
    fn inherit_variables_from_children(
        inventory: &mut ParsedInventory,
        group_name: &str,
    ) -> Result<(), ParseError> {
        let group = match inventory.groups.get(group_name) {
            Some(g) => g.clone(),
            None => return Ok(()),
        };

        for child_name in &group.children {
            if let Some(child_group) = inventory.groups.get(child_name) {
                let child_vars = child_group.vars.clone();

                // Apply child variables to parent group (lower precedence)
                if let Some(parent_group) = inventory.groups.get_mut(group_name) {
                    for (key, value) in child_vars {
                        parent_group.vars.entry(key).or_insert(value);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get effective variables for a specific host, considering all inheritance
    pub fn get_effective_host_variables(
        inventory: &ParsedInventory,
        host_name: &str,
    ) -> HashMap<String, serde_json::Value> {
        let mut effective_vars = HashMap::new();

        if let Some(host) = inventory.hosts.get(host_name) {
            // Start with global inventory variables (lowest precedence)
            effective_vars.extend(inventory.variables.clone());

            // Add variables from the "all" group (applies to all hosts)
            if let Some(all_group) = inventory.groups.get("all") {
                for (key, value) in &all_group.vars {
                    effective_vars.insert(key.clone(), value.clone());
                }
            }

            // Add variables from all groups this host belongs to
            for group_name in &host.groups {
                if let Some(group) = inventory.groups.get(group_name) {
                    // Add group variables (override global and "all" group variables)
                    for (key, value) in &group.vars {
                        effective_vars.insert(key.clone(), value.clone());
                    }
                }
            }

            // Add host-specific variables (highest precedence)
            for (key, value) in &host.vars {
                effective_vars.insert(key.clone(), value.clone());
            }
        }

        effective_vars
    }

    /// Get all hosts that belong to a group (including through inheritance)
    pub fn get_all_group_hosts(inventory: &ParsedInventory, group_name: &str) -> HashSet<String> {
        let mut all_hosts = HashSet::new();
        let mut visited_groups = HashSet::new();

        Self::collect_group_hosts_recursive(
            inventory,
            group_name,
            &mut all_hosts,
            &mut visited_groups,
        );

        all_hosts
    }

    /// Recursively collect all hosts from a group and its children
    fn collect_group_hosts_recursive(
        inventory: &ParsedInventory,
        group_name: &str,
        all_hosts: &mut HashSet<String>,
        visited_groups: &mut HashSet<String>,
    ) {
        if visited_groups.contains(group_name) {
            return; // Avoid infinite recursion
        }

        visited_groups.insert(group_name.to_string());

        if let Some(group) = inventory.groups.get(group_name) {
            // Add direct hosts
            for host_name in &group.hosts {
                all_hosts.insert(host_name.clone());
            }

            // Recursively add hosts from child groups
            for child_name in &group.children {
                Self::collect_group_hosts_recursive(
                    inventory,
                    child_name,
                    all_hosts,
                    visited_groups,
                );
            }
        }
    }

    /// Validate variable inheritance setup
    pub fn validate_variable_inheritance(inventory: &ParsedInventory) -> Result<(), ParseError> {
        // Check that all group references are valid
        for group in inventory.groups.values() {
            for child_name in &group.children {
                if !inventory.groups.contains_key(child_name) {
                    return Err(ParseError::UnknownGroup {
                        group: child_name.clone(),
                    });
                }
            }
        }

        // Check for circular dependencies
        let (graph, _) = Self::build_group_graph(inventory)?;
        if petgraph::algo::is_cyclic_directed(&graph) {
            return Err(ParseError::CircularGroupDependency {
                cycle: "Circular dependency detected in group hierarchy".to_string(),
            });
        }

        Ok(())
    }
}

/// Variable precedence levels in Ansible inventory (from lowest to highest)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariablePrecedence {
    GlobalInventory = 1, // inventory.variables
    GroupAll = 2,        // all group variables
    ParentGroup = 3,     // parent group variables
    ChildGroup = 4,      // child group variables
    Host = 5,            // host-specific variables
}

/// Utility functions for working with variable precedence
impl VariablePrecedence {
    /// Merge variables according to Ansible precedence rules
    pub fn merge_variables(
        global_vars: &HashMap<String, serde_json::Value>,
        group_vars: &[&HashMap<String, serde_json::Value>],
        host_vars: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut merged = HashMap::new();

        // Start with global variables
        merged.extend(global_vars.clone());

        // Apply group variables in order (later groups override earlier ones)
        for vars in group_vars {
            merged.extend((*vars).clone());
        }

        // Apply host variables (highest precedence)
        merged.extend(host_vars.clone());

        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::parsed::{ParsedGroup, ParsedHost};

    fn create_test_inventory() -> ParsedInventory {
        let mut hosts = HashMap::new();
        let mut groups = HashMap::new();

        // Create test hosts
        hosts.insert(
            "web1".to_string(),
            ParsedHost {
                name: "web1".to_string(),
                address: Some("192.168.1.10".to_string()),
                port: None,
                user: None,
                vars: {
                    let mut vars = HashMap::new();
                    vars.insert(
                        "host_var".to_string(),
                        serde_json::Value::String("host_value".to_string()),
                    );
                    vars
                },
                groups: vec!["webservers".to_string(), "production".to_string()],
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
            },
        );

        hosts.insert(
            "web2".to_string(),
            ParsedHost {
                name: "web2".to_string(),
                address: Some("192.168.1.11".to_string()),
                port: None,
                user: None,
                vars: HashMap::new(),
                groups: vec!["webservers".to_string(), "production".to_string()],
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
            },
        );

        // Create test groups
        groups.insert(
            "webservers".to_string(),
            ParsedGroup {
                name: "webservers".to_string(),
                hosts: vec!["web1".to_string(), "web2".to_string()],
                children: Vec::new(),
                vars: {
                    let mut vars = HashMap::new();
                    vars.insert(
                        "http_port".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(80)),
                    );
                    vars.insert(
                        "group_var".to_string(),
                        serde_json::Value::String("webserver_value".to_string()),
                    );
                    vars
                },
            },
        );

        groups.insert(
            "production".to_string(),
            ParsedGroup {
                name: "production".to_string(),
                hosts: Vec::new(),
                children: vec!["webservers".to_string()],
                vars: {
                    let mut vars = HashMap::new();
                    vars.insert(
                        "env".to_string(),
                        serde_json::Value::String("production".to_string()),
                    );
                    vars.insert(
                        "group_var".to_string(),
                        serde_json::Value::String("production_value".to_string()),
                    );
                    vars
                },
            },
        );

        groups.insert(
            "all".to_string(),
            ParsedGroup {
                name: "all".to_string(),
                hosts: vec!["web1".to_string(), "web2".to_string()],
                children: Vec::new(),
                vars: {
                    let mut vars = HashMap::new();
                    vars.insert(
                        "global_var".to_string(),
                        serde_json::Value::String("global_value".to_string()),
                    );
                    vars
                },
            },
        );

        ParsedInventory {
            hosts,
            groups,
            variables: HashMap::new(),
        }
    }

    #[test]
    fn test_variable_inheritance_resolution() {
        let mut inventory = create_test_inventory();

        let result = VariableInheritanceResolver::resolve_group_inheritance(&mut inventory);
        assert!(result.is_ok());

        // Check that host variables have been inherited correctly
        let web1 = inventory.hosts.get("web1").unwrap();

        // Host-specific variable should be preserved (highest precedence)
        assert_eq!(
            web1.vars.get("host_var").unwrap().as_str().unwrap(),
            "host_value"
        );

        // Group variables should be inherited
        assert_eq!(web1.vars.get("http_port").unwrap().as_u64().unwrap(), 80);
        assert_eq!(
            web1.vars.get("env").unwrap().as_str().unwrap(),
            "production"
        );

        // Global variable should be inherited
        assert_eq!(
            web1.vars.get("global_var").unwrap().as_str().unwrap(),
            "global_value"
        );
    }

    #[test]
    fn test_effective_host_variables() {
        let inventory = create_test_inventory();

        let effective_vars =
            VariableInheritanceResolver::get_effective_host_variables(&inventory, "web1");

        // Should include variables from all sources
        assert!(effective_vars.contains_key("host_var"));
        assert!(effective_vars.contains_key("http_port"));
        assert!(effective_vars.contains_key("env"));
        assert!(effective_vars.contains_key("global_var"));

        // Host variables should have highest precedence
        assert_eq!(
            effective_vars.get("host_var").unwrap().as_str().unwrap(),
            "host_value"
        );
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut inventory = ParsedInventory {
            hosts: HashMap::new(),
            groups: HashMap::new(),
            variables: HashMap::new(),
        };

        // Create circular dependency: group1 -> group2 -> group1
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

        let result = VariableInheritanceResolver::resolve_group_inheritance(&mut inventory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::CircularGroupDependency { .. }
        ));
    }

    #[test]
    fn test_get_all_group_hosts() {
        let inventory = create_test_inventory();

        let production_hosts =
            VariableInheritanceResolver::get_all_group_hosts(&inventory, "production");
        assert_eq!(production_hosts.len(), 2);
        assert!(production_hosts.contains("web1"));
        assert!(production_hosts.contains("web2"));

        let webserver_hosts =
            VariableInheritanceResolver::get_all_group_hosts(&inventory, "webservers");
        assert_eq!(webserver_hosts.len(), 2);
        assert!(webserver_hosts.contains("web1"));
        assert!(webserver_hosts.contains("web2"));
    }

    #[test]
    fn test_variable_precedence_merge() {
        let global_vars = {
            let mut vars = HashMap::new();
            vars.insert(
                "var1".to_string(),
                serde_json::Value::String("global".to_string()),
            );
            vars.insert(
                "var2".to_string(),
                serde_json::Value::String("global".to_string()),
            );
            vars
        };

        let group_vars = {
            let mut vars = HashMap::new();
            vars.insert(
                "var2".to_string(),
                serde_json::Value::String("group".to_string()),
            );
            vars.insert(
                "var3".to_string(),
                serde_json::Value::String("group".to_string()),
            );
            vars
        };

        let host_vars = {
            let mut vars = HashMap::new();
            vars.insert(
                "var3".to_string(),
                serde_json::Value::String("host".to_string()),
            );
            vars.insert(
                "var4".to_string(),
                serde_json::Value::String("host".to_string()),
            );
            vars
        };

        let merged = VariablePrecedence::merge_variables(&global_vars, &[&group_vars], &host_vars);

        assert_eq!(merged.get("var1").unwrap().as_str().unwrap(), "global");
        assert_eq!(merged.get("var2").unwrap().as_str().unwrap(), "group");
        assert_eq!(merged.get("var3").unwrap().as_str().unwrap(), "host");
        assert_eq!(merged.get("var4").unwrap().as_str().unwrap(), "host");
    }
}
