use crate::parser::error::ParseError;
use crate::types::parsed::ParsedPlay;
use petgraph::{graph::DiGraph, Graph};
use std::collections::HashMap;

pub fn resolve_task_dependencies(plays: &[ParsedPlay]) -> Vec<String> {
    let mut all_tasks = Vec::new();

    for play in plays {
        for task in &play.tasks {
            all_tasks.push(task.id.clone());
        }
        for handler in &play.handlers {
            all_tasks.push(handler.id.clone());
        }
    }

    // For now, just return the task IDs in order
    // TODO: Implement proper dependency graph resolution
    all_tasks
}

pub fn resolve_task_dependencies_with_graph(
    plays: &[ParsedPlay],
) -> Result<Vec<String>, ParseError> {
    let mut graph: DiGraph<String, ()> = Graph::new();
    let mut task_indices = HashMap::new();

    // Add all tasks to the graph
    for play in plays {
        for task in &play.tasks {
            let node = graph.add_node(task.id.clone());
            task_indices.insert(task.id.clone(), node);
        }
        for handler in &play.handlers {
            let node = graph.add_node(handler.id.clone());
            task_indices.insert(handler.id.clone(), node);
        }
    }

    // Add dependency edges
    for play in plays {
        for task in &play.tasks {
            if let Some(task_node) = task_indices.get(&task.id) {
                // Add edges for explicit dependencies
                for dep in &task.dependencies {
                    if let Some(dep_node) = task_indices.get(dep) {
                        graph.add_edge(*dep_node, *task_node, ());
                    }
                }

                // Add edges for handler notifications
                for notify in &task.notify {
                    if let Some(handler_node) = task_indices.get(notify) {
                        graph.add_edge(*task_node, *handler_node, ());
                    }
                }
            }
        }
    }

    // Topological sort for execution order
    petgraph::algo::toposort(&graph, None)
        .map_err(|_| ParseError::CircularDependency {
            cycle: "task dependency cycle detected".to_string(),
        })
        .map(|sorted| sorted.into_iter().map(|node| graph[node].clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::parsed::ParsedTask;
    use std::collections::HashMap;

    fn create_test_task(id: &str, dependencies: Vec<String>, notify: Vec<String>) -> ParsedTask {
        ParsedTask {
            id: id.to_string(),
            name: format!("Task {}", id),
            module: "debug".to_string(),
            args: HashMap::new(),
            vars: HashMap::new(),
            when: None,
            loop_items: None,
            tags: vec![],
            notify,
            changed_when: None,
            failed_when: None,
            ignore_errors: false,
            delegate_to: None,
            dependencies,
        }
    }

    fn create_test_handler(id: &str) -> ParsedTask {
        ParsedTask {
            id: id.to_string(),
            name: format!("Handler {}", id),
            module: "service".to_string(),
            args: HashMap::new(),
            vars: HashMap::new(),
            when: None,
            loop_items: None,
            tags: vec![],
            notify: vec![],
            changed_when: None,
            failed_when: None,
            ignore_errors: false,
            delegate_to: None,
            dependencies: vec![],
        }
    }

    fn create_test_play(tasks: Vec<ParsedTask>, handlers: Vec<ParsedTask>) -> ParsedPlay {
        use crate::types::parsed::{ExecutionStrategy, HostPattern};
        ParsedPlay {
            name: "Test Play".to_string(),
            hosts: HostPattern::Single("all".to_string()),
            tasks,
            handlers,
            vars: HashMap::new(),
            roles: vec![],
            strategy: ExecutionStrategy::default(),
            serial: None,
            max_fail_percentage: None,
        }
    }

    #[test]
    fn test_resolve_task_dependencies_empty() {
        let plays = vec![];
        let result = resolve_task_dependencies(&plays);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_task_dependencies_single_task() {
        let task = create_test_task("task1", vec![], vec![]);
        let play = create_test_play(vec![task], vec![]);
        let plays = vec![play];

        let result = resolve_task_dependencies(&plays);
        assert_eq!(result, vec!["task1"]);
    }

    #[test]
    fn test_resolve_task_dependencies_multiple_tasks() {
        let task1 = create_test_task("task1", vec![], vec![]);
        let task2 = create_test_task("task2", vec![], vec![]);
        let handler1 = create_test_handler("handler1");

        let play = create_test_play(vec![task1, task2], vec![handler1]);
        let plays = vec![play];

        let result = resolve_task_dependencies(&plays);
        assert_eq!(result, vec!["task1", "task2", "handler1"]);
    }

    #[test]
    fn test_resolve_task_dependencies_with_graph_empty() {
        let plays = vec![];
        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_resolve_task_dependencies_with_graph_single_task() {
        let task = create_test_task("task1", vec![], vec![]);
        let play = create_test_play(vec![task], vec![]);
        let plays = vec![play];

        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["task1"]);
    }

    #[test]
    fn test_resolve_task_dependencies_with_graph_dependency_order() {
        let task1 = create_test_task("task1", vec![], vec![]);
        let task2 = create_test_task("task2", vec!["task1".to_string()], vec![]);
        let play = create_test_play(vec![task1, task2], vec![]);
        let plays = vec![play];

        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        let resolved = result.unwrap();

        // task1 should come before task2 due to dependency
        let task1_pos = resolved.iter().position(|x| x == "task1").unwrap();
        let task2_pos = resolved.iter().position(|x| x == "task2").unwrap();
        assert!(task1_pos < task2_pos);
    }

    #[test]
    fn test_resolve_task_dependencies_with_graph_handler_notification() {
        let handler = create_test_handler("handler1");
        let task = create_test_task("task1", vec![], vec!["handler1".to_string()]);
        let play = create_test_play(vec![task], vec![handler]);
        let plays = vec![play];

        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        let resolved = result.unwrap();

        // task1 should come before handler1 due to notification
        let task_pos = resolved.iter().position(|x| x == "task1").unwrap();
        let handler_pos = resolved.iter().position(|x| x == "handler1").unwrap();
        assert!(task_pos < handler_pos);
    }

    #[test]
    fn test_resolve_task_dependencies_with_graph_missing_dependency() {
        let task = create_test_task("task1", vec!["nonexistent".to_string()], vec![]);
        let play = create_test_play(vec![task], vec![]);
        let plays = vec![play];

        // Should still work, just ignore missing dependencies
        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["task1"]);
    }

    #[test]
    fn test_resolve_task_dependencies_complex_scenario() {
        // Create a complex dependency graph:
        // task1 -> task2 -> handler1
        // task3 -> handler2
        let task1 = create_test_task("task1", vec![], vec![]);
        let task2 = create_test_task(
            "task2",
            vec!["task1".to_string()],
            vec!["handler1".to_string()],
        );
        let task3 = create_test_task("task3", vec![], vec!["handler2".to_string()]);
        let handler1 = create_test_handler("handler1");
        let handler2 = create_test_handler("handler2");

        let play = create_test_play(vec![task1, task2, task3], vec![handler1, handler2]);
        let plays = vec![play];

        let result = resolve_task_dependencies_with_graph(&plays);
        assert!(result.is_ok());
        let resolved = result.unwrap();

        // Verify ordering constraints
        let task1_pos = resolved.iter().position(|x| x == "task1").unwrap();
        let task2_pos = resolved.iter().position(|x| x == "task2").unwrap();
        let handler1_pos = resolved.iter().position(|x| x == "handler1").unwrap();
        let task3_pos = resolved.iter().position(|x| x == "task3").unwrap();
        let handler2_pos = resolved.iter().position(|x| x == "handler2").unwrap();

        assert!(task1_pos < task2_pos);
        assert!(task2_pos < handler1_pos);
        assert!(task3_pos < handler2_pos);
    }
}
