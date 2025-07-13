use crate::parser::error::ParseError;
use crate::parser::include::IncludeType;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Dependency information for include/import
#[derive(Debug, Clone)]
pub struct IncludeDependency {
    pub target_file: String,
    pub include_type: IncludeType,
    pub conditional: bool,
}

/// Graph for tracking include/import dependencies
#[derive(Debug)]
pub struct IncludeDependencyGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<IncludeDependency>>,
}

impl IncludeDependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a dependency between two files
    pub fn add_dependency(
        &mut self,
        source_file: String,
        target_file: String,
        include_type: IncludeType,
    ) {
        self.nodes.insert(source_file.clone());
        self.nodes.insert(target_file.clone());

        let dependency = IncludeDependency {
            target_file: target_file.clone(),
            include_type,
            conditional: false, // TODO: Detect conditional includes
        };

        self.edges.entry(source_file).or_default().push(dependency);
    }

    /// Check for circular dependencies in the graph
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                if let Some(cycle) =
                    self.dfs_cycle_detection(node, &mut visited, &mut rec_stack, &mut path)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// Get dependencies for a specific file
    pub fn get_dependencies(&self, file: &str) -> Option<&Vec<IncludeDependency>> {
        self.edges.get(file)
    }

    /// Get all files that depend on the given file
    pub fn get_dependents(&self, file: &str) -> Vec<String> {
        let mut dependents = Vec::new();

        for (source, deps) in &self.edges {
            if deps.iter().any(|dep| dep.target_file == file) {
                dependents.push(source.clone());
            }
        }

        dependents
    }

    /// Get topological order of dependencies
    pub fn topological_sort(&self) -> Result<Vec<String>, ParseError> {
        if let Some(cycle) = self.detect_cycles() {
            return Err(ParseError::CircularIncludeDependency {
                cycle: cycle.join(" -> "),
            });
        }

        let mut visited = HashSet::new();
        let mut result = Vec::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                self.topological_visit(node, &mut visited, &mut result);
            }
        }

        result.reverse();
        Ok(result)
    }

    /// DFS for cycle detection
    fn dfs_cycle_detection(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(dependencies) = self.edges.get(node) {
            for dep in dependencies {
                let target = &dep.target_file;

                if !visited.contains(target) {
                    if let Some(cycle) = self.dfs_cycle_detection(target, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(target) {
                    // Found a cycle - extract the cycle path
                    let cycle_start = path.iter().position(|x| x == target).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(target.to_string());
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// DFS for topological sorting
    fn topological_visit(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        visited.insert(node.to_string());

        if let Some(dependencies) = self.edges.get(node) {
            for dep in dependencies {
                if !visited.contains(&dep.target_file) {
                    self.topological_visit(&dep.target_file, visited, result);
                }
            }
        }

        result.push(node.to_string());
    }
}

impl Default for IncludeDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack for tracking current include chain to detect circular dependencies
#[derive(Debug, Clone)]
pub struct IncludeStack {
    stack: Vec<PathBuf>,
    max_depth: usize,
}

impl IncludeStack {
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Vec::new(),
            max_depth,
        }
    }

    /// Push a file onto the include stack
    pub fn push(&mut self, file_path: PathBuf) -> Result<(), ParseError> {
        // Check for circular dependency
        if self.stack.contains(&file_path) {
            let cycle = self.build_cycle_description(&file_path);
            return Err(ParseError::CircularIncludeDependency { cycle });
        }

        // Check for maximum depth
        if self.stack.len() >= self.max_depth {
            return Err(ParseError::MaxIncludeDepthExceeded {
                depth: self.max_depth,
                file: file_path.to_string_lossy().to_string(),
            });
        }

        self.stack.push(file_path);
        Ok(())
    }

    /// Pop a file from the include stack
    pub fn pop(&mut self) -> Option<PathBuf> {
        self.stack.pop()
    }

    /// Get current include depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if a file would create a circular dependency
    pub fn would_create_cycle(&self, file_path: &Path) -> bool {
        self.stack.contains(&file_path.to_path_buf())
    }

    /// Build description of the circular dependency
    fn build_cycle_description(&self, file_path: &Path) -> String {
        let mut cycle_files = Vec::new();
        let mut found_start = false;

        for stack_file in &self.stack {
            if stack_file == file_path {
                found_start = true;
            }
            if found_start {
                cycle_files.push(stack_file.to_string_lossy().to_string());
            }
        }

        cycle_files.push(file_path.to_string_lossy().to_string());
        cycle_files.join(" -> ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_basic() {
        let mut graph = IncludeDependencyGraph::new();

        graph.add_dependency(
            "main.yml".to_string(),
            "tasks.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "tasks.yml".to_string(),
            "subtasks.yml".to_string(),
            IncludeType::ImportTasks,
        );

        assert_eq!(graph.nodes.len(), 3);
        assert!(graph.get_dependencies("main.yml").is_some());
        assert!(graph.get_dependencies("tasks.yml").is_some());
        assert!(graph.get_dependencies("subtasks.yml").is_none());
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = IncludeDependencyGraph::new();

        // Create a cycle: a -> b -> c -> a
        graph.add_dependency(
            "a.yml".to_string(),
            "b.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "b.yml".to_string(),
            "c.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "c.yml".to_string(),
            "a.yml".to_string(),
            IncludeType::IncludeTasks,
        );

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3); // Should contain at least the cycle elements
    }

    #[test]
    fn test_no_cycle() {
        let mut graph = IncludeDependencyGraph::new();

        // Create a DAG: a -> b -> c
        graph.add_dependency(
            "a.yml".to_string(),
            "b.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "b.yml".to_string(),
            "c.yml".to_string(),
            IncludeType::IncludeTasks,
        );

        let cycle = graph.detect_cycles();
        assert!(cycle.is_none());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = IncludeDependencyGraph::new();

        graph.add_dependency(
            "main.yml".to_string(),
            "setup.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "main.yml".to_string(),
            "deploy.yml".to_string(),
            IncludeType::IncludeTasks,
        );
        graph.add_dependency(
            "deploy.yml".to_string(),
            "config.yml".to_string(),
            IncludeType::IncludeTasks,
        );

        let sorted = graph.topological_sort().unwrap();

        // setup.yml and config.yml should come before files that depend on them
        let setup_pos = sorted.iter().position(|x| x == "setup.yml").unwrap();
        let config_pos = sorted.iter().position(|x| x == "config.yml").unwrap();
        let deploy_pos = sorted.iter().position(|x| x == "deploy.yml").unwrap();
        let main_pos = sorted.iter().position(|x| x == "main.yml").unwrap();

        assert!(main_pos < setup_pos);
        assert!(deploy_pos < config_pos);
        assert!(main_pos < deploy_pos);
    }

    #[test]
    fn test_include_stack() {
        let mut stack = IncludeStack::new(3);

        // Push files onto stack
        assert!(stack.push(PathBuf::from("main.yml")).is_ok());
        assert!(stack.push(PathBuf::from("tasks.yml")).is_ok());
        assert_eq!(stack.depth(), 2);

        // Try to create circular dependency
        let result = stack.push(PathBuf::from("main.yml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::CircularIncludeDependency { .. }
        ));

        // Test max depth
        assert!(stack.push(PathBuf::from("subtasks.yml")).is_ok());
        let result = stack.push(PathBuf::from("more.yml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::MaxIncludeDepthExceeded { .. }
        ));
    }

    #[test]
    fn test_include_stack_pop() {
        let mut stack = IncludeStack::new(10);

        stack.push(PathBuf::from("main.yml")).unwrap();
        stack.push(PathBuf::from("tasks.yml")).unwrap();

        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.pop(), Some(PathBuf::from("tasks.yml")));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.pop(), Some(PathBuf::from("main.yml")));
        assert_eq!(stack.depth(), 0);
        assert_eq!(stack.pop(), None);
    }
}
