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
