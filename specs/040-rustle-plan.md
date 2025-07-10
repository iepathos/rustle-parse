# Spec 040: Rustle Plan Tool

## Feature Summary

The `rustle-plan` tool is a specialized execution planner that takes parsed playbooks and generates optimized execution plans. It analyzes task dependencies, determines parallelization opportunities, handles conditional logic, and produces detailed execution graphs for consumption by executor tools.

**Problem it solves**: Separates execution planning from parsing and execution, enabling better optimization, dry-run capabilities, and execution strategy analysis without running actual tasks.

**High-level approach**: Create a standalone binary that reads parsed playbook data, analyzes dependencies and constraints, and outputs detailed execution plans with parallelization and ordering information.

## Goals & Requirements

### Functional Requirements
- Generate optimized execution plans from parsed playbooks
- Analyze task dependencies and execution order
- Determine parallelization opportunities within plays and across hosts
- Handle conditional execution (when clauses, tags, limits)
- Support different execution strategies (linear, rolling, batch)
- Generate execution graphs and dependency visualizations
- Provide execution time estimates
- Support dry-run and check mode planning

### Non-functional Requirements
- **Performance**: Plan 1000+ task playbooks in &lt;1 second
- **Memory**: Keep memory usage &lt;50MB for typical playbooks
- **Scalability**: Handle plans for 1000+ hosts efficiently
- **Accuracy**: 99%+ accuracy in dependency detection
- **Optimization**: Identify 80%+ of possible parallelization opportunities

### Success Criteria
- Generated plans execute correctly when consumed by rustle-exec
- Performance benchmarks show 10x+ improvement over Ansible planning
- Parallelization strategies reduce execution time by 40%+ on multi-host deployments
- Dry-run mode provides accurate execution predictions

## API/Interface Design

### Command Line Interface
```bash
rustle-plan [OPTIONS] [PARSED_PLAYBOOK]

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Parsed inventory file
    -l, --limit &lt;PATTERN&gt;          Limit execution to specific hosts
    -t, --tags &lt;TAGS&gt;              Only run tasks with these tags
    --skip-tags &lt;TAGS&gt;             Skip tasks with these tags
    -s, --strategy &lt;STRATEGY&gt;      Execution strategy [default: linear]
    --serial &lt;NUM&gt;                 Number of hosts to run at once
    --forks &lt;NUM&gt;                  Maximum parallel processes [default: 50]
    -c, --check                    Check mode (don't make changes)
    --diff                         Show file differences
    --list-tasks                   List all planned tasks
    --list-hosts                   List all target hosts
    --visualize                    Generate execution graph visualization
    -o, --output &lt;FORMAT&gt;          Output format: json, binary, dot [default: json]
    --optimize                     Enable execution optimizations
    --estimate-time                Include execution time estimates
    --dry-run                      Plan but don't output execution plan
    -v, --verbose                  Enable verbose output

ARGS:
    &lt;PARSED_PLAYBOOK&gt;  Path to parsed playbook file (or stdin if -)
```

### Core Data Structures

```rust
// Main execution plan output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub metadata: PlanMetadata,
    pub plays: Vec&lt;PlayPlan&gt;,
    pub total_tasks: usize,
    pub estimated_duration: Option&lt;Duration&gt;,
    pub parallelism_score: f32,
    pub hosts: Vec&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    pub created_at: DateTime&lt;Utc&gt;,
    pub rustle_version: String,
    pub playbook_hash: String,
    pub inventory_hash: String,
    pub planning_options: PlanningOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayPlan {
    pub play_id: String,
    pub name: String,
    pub strategy: ExecutionStrategy,
    pub serial: Option&lt;u32&gt;,
    pub hosts: Vec&lt;String&gt;,
    pub batches: Vec&lt;ExecutionBatch&gt;,
    pub handlers: Vec&lt;HandlerPlan&gt;,
    pub estimated_duration: Option&lt;Duration&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBatch {
    pub batch_id: String,
    pub hosts: Vec&lt;String&gt;,
    pub tasks: Vec&lt;TaskPlan&gt;,
    pub parallel_groups: Vec&lt;ParallelGroup&gt;,
    pub dependencies: Vec&lt;String&gt;,
    pub estimated_duration: Option&lt;Duration&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap&lt;String, Value&gt;,
    pub hosts: Vec&lt;String&gt;,
    pub dependencies: Vec&lt;String&gt;,
    pub conditions: Vec&lt;ExecutionCondition&gt;,
    pub tags: Vec&lt;String&gt;,
    pub notify: Vec&lt;String&gt;,
    pub execution_order: u32,
    pub can_run_parallel: bool,
    pub estimated_duration: Option&lt;Duration&gt;,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub group_id: String,
    pub tasks: Vec&lt;String&gt;,
    pub max_parallelism: u32,
    pub shared_resources: Vec&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Linear,
    Rolling { batch_size: u32 },
    Free,
    HostPinned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionCondition {
    When { expression: String },
    Tag { tags: Vec&lt;String&gt; },
    Host { pattern: String },
    SkipTag { tags: Vec&lt;String&gt; },
    CheckMode { enabled: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,    // Read-only operations
    Medium, // File modifications
    High,   // Service restarts, system changes
    Critical, // Destructive operations
}
```

### Planner API

```rust
pub struct ExecutionPlanner {
    strategy: ExecutionStrategy,
    forks: u32,
    optimize: bool,
    check_mode: bool,
    task_estimator: TaskEstimator,
}

impl ExecutionPlanner {
    pub fn new() -&gt; Self;
    pub fn with_strategy(mut self, strategy: ExecutionStrategy) -&gt; Self;
    pub fn with_forks(mut self, forks: u32) -&gt; Self;
    pub fn with_optimization(mut self, enabled: bool) -&gt; Self;
    pub fn with_check_mode(mut self, enabled: bool) -&gt; Self;
    
    pub fn plan_execution(
        &amp;self, 
        playbook: &amp;ParsedPlaybook, 
        inventory: &amp;ParsedInventory,
        options: &amp;PlanningOptions,
    ) -&gt; Result&lt;ExecutionPlan, PlanError&gt;;
    
    pub fn analyze_dependencies(
        &amp;self, 
        tasks: &amp;[ParsedTask]
    ) -&gt; Result&lt;DependencyGraph, PlanError&gt;;
    
    pub fn optimize_execution_order(
        &amp;self, 
        tasks: &amp;[TaskPlan]
    ) -&gt; Result&lt;Vec&lt;TaskPlan&gt;, PlanError&gt;;
    
    pub fn estimate_duration(
        &amp;self, 
        plan: &amp;ExecutionPlan
    ) -&gt; Result&lt;Duration, PlanError&gt;;
    
    pub fn validate_plan(
        &amp;self, 
        plan: &amp;ExecutionPlan
    ) -&gt; Result&lt;ValidationReport, PlanError&gt;;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningOptions {
    pub limit: Option&lt;String&gt;,
    pub tags: Vec&lt;String&gt;,
    pub skip_tags: Vec&lt;String&gt;,
    pub check_mode: bool,
    pub diff_mode: bool,
    pub forks: u32,
    pub serial: Option&lt;u32&gt;,
    pub strategy: ExecutionStrategy,
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("Circular dependency detected in tasks: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Invalid host pattern '{pattern}': {reason}")]
    InvalidHostPattern { pattern: String, reason: String },
    
    #[error("Unknown task '{task_id}' referenced in dependency")]
    UnknownTaskDependency { task_id: String },
    
    #[error("Conflicting execution strategies: {conflict}")]
    StrategyConflict { conflict: String },
    
    #[error("Resource contention detected: {resources}")]
    ResourceContention { resources: Vec&lt;String&gt; },
    
    #[error("Planning timeout exceeded: {timeout_secs}s")]
    PlanningTimeout { timeout_secs: u64 },
    
    #[error("Invalid tag expression: {expression}")]
    InvalidTagExpression { expression: String },
    
    #[error("Insufficient resources for parallelism: required {required}, available {available}")]
    InsufficientResources { required: u32, available: u32 },
}
```

## File and Package Structure

```
src/bin/rustle-plan.rs          # Main binary entry point
src/planner/
├── mod.rs                      # Module exports
├── execution_plan.rs           # Core planning logic
├── dependency.rs               # Dependency analysis
├── optimization.rs             # Execution optimization
├── strategy.rs                 # Execution strategies
├── condition.rs                # Conditional execution
├── estimation.rs               # Time estimation
├── validation.rs               # Plan validation
├── graph.rs                    # Dependency graphs
└── error.rs                    # Error types

src/types/
├── plan.rs                     # Plan data structures
└── strategy.rs                 # Strategy definitions

tests/planner/
├── execution_plan_tests.rs
├── dependency_tests.rs
├── optimization_tests.rs
├── strategy_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Basic Planning
1. Implement core execution plan data structures
2. Create basic dependency analysis from parsed tasks
3. Add support for linear execution strategy
4. Implement host filtering and task selection

### Phase 2: Dependency Analysis
1. Build comprehensive dependency graph from tasks
2. Implement topological sorting for execution order
3. Add circular dependency detection
4. Handle handler notification dependencies

### Phase 3: Parallelization Optimization
1. Analyze tasks for parallel execution opportunities
2. Implement resource contention detection
3. Generate parallel execution groups
4. Optimize batch sizing for different strategies

### Phase 4: Advanced Features
1. Add execution time estimation
2. Implement rolling update strategies
3. Add plan validation and verification
4. Create visualization output formats

### Key Algorithms

**Dependency Analysis**:
```rust
fn analyze_task_dependencies(tasks: &amp;[ParsedTask]) -&gt; Result&lt;DependencyGraph, PlanError&gt; {
    let mut graph = DiGraph::new();
    let mut task_map = HashMap::new();
    
    // Add all tasks as nodes
    for task in tasks {
        let node = graph.add_node(task.id.clone());
        task_map.insert(task.id.clone(), (node, task));
    }
    
    // Add explicit dependencies
    for (node, task) in task_map.values() {
        for dep_id in &amp;task.dependencies {
            if let Some((dep_node, _)) = task_map.get(dep_id) {
                graph.add_edge(*dep_node, *node, DependencyType::Explicit);
            }
        }
        
        // Add implicit dependencies (file operations, service management)
        for other_task in tasks {
            if let Some(dependency_type) = detect_implicit_dependency(task, other_task) {
                if let Some((other_node, _)) = task_map.get(&amp;other_task.id) {
                    graph.add_edge(*other_node, *node, dependency_type);
                }
            }
        }
    }
    
    Ok(DependencyGraph::new(graph))
}

fn detect_implicit_dependency(task1: &amp;ParsedTask, task2: &amp;ParsedTask) -&gt; Option&lt;DependencyType&gt; {
    // File-based dependencies
    if let (Some(dest1), Some(dest2)) = (
        task1.args.get("dest").and_then(|v| v.as_str()),
        task2.args.get("src").and_then(|v| v.as_str())
    ) {
        if dest1 == dest2 {
            return Some(DependencyType::FileOutput);
        }
    }
    
    // Service dependencies
    if task1.module == "service" &amp;&amp; task2.module == "package" {
        if let (Some(service), Some(package)) = (
            task1.args.get("name").and_then(|v| v.as_str()),
            task2.args.get("name").and_then(|v| v.as_str())
        ) {
            if service == package {
                return Some(DependencyType::ServicePackage);
            }
        }
    }
    
    None
}
```

**Parallelization Analysis**:
```rust
fn find_parallel_groups(
    tasks: &amp;[TaskPlan], 
    dependency_graph: &amp;DependencyGraph
) -&gt; Vec&lt;ParallelGroup&gt; {
    let mut groups = Vec::new();
    let mut visited = HashSet::new();
    
    for task in tasks {
        if visited.contains(&amp;task.task_id) {
            continue;
        }
        
        let mut group_tasks = vec![task.task_id.clone()];
        visited.insert(task.task_id.clone());
        
        // Find tasks that can run in parallel with this one
        for other_task in tasks {
            if visited.contains(&amp;other_task.task_id) {
                continue;
            }
            
            if can_run_parallel(task, other_task, dependency_graph) {
                group_tasks.push(other_task.task_id.clone());
                visited.insert(other_task.task_id.clone());
            }
        }
        
        if group_tasks.len() &gt; 1 {
            groups.push(ParallelGroup {
                group_id: format!("group_{}", groups.len()),
                tasks: group_tasks,
                max_parallelism: calculate_max_parallelism(&amp;group_tasks),
                shared_resources: find_shared_resources(&amp;group_tasks),
            });
        }
    }
    
    groups
}

fn can_run_parallel(
    task1: &amp;TaskPlan, 
    task2: &amp;TaskPlan, 
    graph: &amp;DependencyGraph
) -&gt; bool {
    // Check for direct dependencies
    if graph.has_path(&amp;task1.task_id, &amp;task2.task_id) ||
       graph.has_path(&amp;task2.task_id, &amp;task1.task_id) {
        return false;
    }
    
    // Check for resource conflicts
    if has_resource_conflict(task1, task2) {
        return false;
    }
    
    // Check module-specific constraints
    if requires_exclusive_access(&amp;task1.module) || requires_exclusive_access(&amp;task2.module) {
        return false;
    }
    
    true
}
```

## Testing Strategy

### Unit Tests
- **Dependency analysis**: Test graph construction with various task types
- **Parallelization**: Test parallel group detection and optimization
- **Strategy handling**: Test different execution strategies
- **Condition evaluation**: Test tag and host filtering logic

### Integration Tests
- **End-to-end planning**: Test complete planning workflows
- **Large playbooks**: Test performance with complex playbooks
- **Error scenarios**: Test error handling with invalid inputs
- **Optimization verification**: Test execution time improvements

### Test Data Structure
```
tests/fixtures/
├── playbooks/
│   ├── simple_plan.json        # Basic parsed playbook
│   ├── complex_plan.json       # Multi-play with dependencies
│   ├── parallel_tasks.json     # Tasks suitable for parallelization
│   └── rolling_update.json     # Rolling deployment scenario
├── inventories/
│   ├── small_inventory.json    # 5 hosts
│   ├── large_inventory.json    # 100+ hosts
│   └── groups_inventory.json   # Complex group structure
└── expected_plans/
    ├── simple_linear.json      # Expected linear execution plan
    ├── optimized_parallel.json # Expected optimized plan
    └── rolling_batches.json    # Expected rolling update plan
```

### Performance Benchmarks
- Planning time vs. playbook size
- Memory usage with large inventories
- Parallelization effectiveness measurements
- Comparison with Ansible planning times

## Edge Cases &amp; Error Handling

### Dependency Analysis
- Circular dependencies between tasks
- Missing task references in dependencies
- Complex conditional dependencies
- Cross-play dependencies

### Resource Management
- Memory limits with large dependency graphs
- Timeout handling for complex planning
- Resource contention detection
- Invalid parallelism constraints

### Execution Strategies
- Conflicting strategy requirements
- Serial constraints with parallel tasks
- Host availability during planning
- Failed host handling in rolling updates

## Dependencies

### External Crates
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
petgraph = "0.6"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
regex = "1"
tokio = { version = "1", features = ["time"] }
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle-parse` output types
- Shared planning algorithms

## Configuration

### Environment Variables
- `RUSTLE_DEFAULT_FORKS`: Default parallelism level
- `RUSTLE_PLANNING_TIMEOUT`: Maximum planning time
- `RUSTLE_OPTIMIZATION_LEVEL`: Optimization aggressiveness (0-3)
- `RUSTLE_STRATEGY`: Default execution strategy

### Configuration File Support
```toml
[planner]
default_strategy = "linear"
default_forks = 50
enable_optimization = true
planning_timeout_secs = 30
max_parallelism = 100

[estimation]
enable_time_estimation = true
default_task_duration_secs = 5
module_duration_overrides = { "package" = 30, "service" = 10 }

[optimization]
enable_parallel_groups = true
resource_contention_detection = true
batch_size_optimization = true
```

## Documentation

### CLI Help Text
```
rustle-plan - Generate optimized execution plans from parsed playbooks

USAGE:
    rustle-plan [OPTIONS] [PARSED_PLAYBOOK]

ARGS:
    &lt;PARSED_PLAYBOOK&gt;    Path to parsed playbook file (or stdin if -)

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Parsed inventory file
    -l, --limit &lt;PATTERN&gt;          Limit execution to specific hosts
    -t, --tags &lt;TAGS&gt;              Only run tasks with these tags
        --skip-tags &lt;TAGS&gt;         Skip tasks with these tags
    -s, --strategy &lt;STRATEGY&gt;      Execution strategy [default: linear] [possible values: linear, rolling, free]
        --serial &lt;NUM&gt;             Number of hosts to run at once
        --forks &lt;NUM&gt;              Maximum parallel processes [default: 50]
    -c, --check                    Check mode (don't make changes)
        --diff                     Show file differences
        --list-tasks               List all planned tasks
        --list-hosts               List all target hosts
        --visualize                Generate execution graph visualization
    -o, --output &lt;FORMAT&gt;          Output format [default: json] [possible values: json, binary, dot]
        --optimize                 Enable execution optimizations
        --estimate-time            Include execution time estimates
        --dry-run                  Plan but don't output execution plan
    -v, --verbose                  Enable verbose output
    -h, --help                     Print help information
    -V, --version                  Print version information

EXAMPLES:
    rustle-plan parsed_playbook.json                           # Generate basic execution plan
    rustle-plan -i inventory.json --optimize playbook.json     # Optimized plan with inventory
    rustle-plan --strategy rolling --serial 5 playbook.json    # Rolling update strategy
    rustle-plan --list-tasks --tags deploy playbook.json       # List deployment tasks only
    rustle-plan --visualize -o dot playbook.json &gt; graph.dot   # Generate dependency graph
```

### API Documentation
Comprehensive rustdoc documentation including:
- Planning algorithm explanations
- Performance characteristics
- Strategy selection guidelines
- Optimization techniques

### Integration Examples
```bash
# Basic planning pipeline
rustle-parse playbook.yml | rustle-plan | rustle-exec

# Optimized rolling deployment
rustle-parse -i inventory.ini deploy.yml | \
  rustle-plan --strategy rolling --serial 5 --optimize | \
  rustle-exec

# Dry-run with time estimation
rustle-parse playbook.yml | \
  rustle-plan --check --estimate-time | \
  jq '.estimated_duration'

# Parallel task analysis
rustle-parse complex.yml | \
  rustle-plan --optimize --list-tasks | \
  jq '.plays[].batches[].parallel_groups'
```