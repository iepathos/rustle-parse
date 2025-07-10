# Spec 030: Rustle Parse Tool

## Feature Summary

The `rustle-parse` tool is a specialized YAML and inventory parser that converts Ansible-compatible playbooks and inventory files into structured JSON/binary format. This tool handles syntax validation, variable resolution, templating, and produces standardized output for consumption by other Rustle tools.

**Problem it solves**: Centralizes all parsing logic into a single, focused tool that can be reused across the Rustle ecosystem, enabling better performance, caching, and error reporting.

**High-level approach**: Create a standalone binary that reads YAML playbooks and inventory files, resolves variables and templates, validates syntax, and outputs structured data in a standardized format.

## Goals & Requirements

### Functional Requirements
- Parse Ansible-compatible YAML playbooks
- Parse inventory files (INI, YAML, JSON formats)
- Resolve Jinja2 templates and variable substitutions
- Validate syntax and semantic correctness
- Support include/import directives
- Handle encrypted vault variables
- Generate dependency graphs for tasks and roles
- Output standardized JSON or binary format

### Non-functional Requirements
- **Performance**: Parse large playbooks (&gt;1000 tasks) in &lt;2 seconds
- **Memory**: Keep memory usage &lt;100MB for typical playbooks
- **Compatibility**: 100% compatible with Ansible YAML syntax
- **Reliability**: Comprehensive error reporting with line numbers
- **Caching**: Support output caching to avoid re-parsing

### Success Criteria
- All existing Rustle test playbooks parse successfully
- Performance benchmarks show 5x+ improvement over Python parsing
- Error messages are more detailed than ansible-playbook --syntax-check
- Output format is consumable by other Rustle tools

## API/Interface Design

### Command Line Interface
```bash
rustle-parse [OPTIONS] [PLAYBOOK_FILE]

OPTIONS:
    -i, --inventory &lt;FILE&gt;     Inventory file path
    -e, --extra-vars &lt;VARS&gt;    Extra variables (key=value,...)
    -o, --output &lt;FORMAT&gt;      Output format: json, binary, yaml [default: json]
    -c, --cache-dir &lt;DIR&gt;      Cache directory for parsed results
    -v, --vault-password-file &lt;FILE&gt;  Vault password file
    --syntax-check             Only validate syntax, don't output
    --list-tasks               List all tasks with metadata
    --list-hosts               List all hosts with variables
    --verbose                  Enable verbose output
    --dry-run                  Parse but don't write output
    
ARGS:
    &lt;PLAYBOOK_FILE&gt;  Path to playbook file (or stdin if -)
```

### Core Data Structures

```rust
// Output format for parsed playbooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlaybook {
    pub metadata: PlaybookMetadata,
    pub plays: Vec&lt;ParsedPlay&gt;,
    pub variables: HashMap&lt;String, Value&gt;,
    pub facts_required: bool,
    pub vault_ids: Vec&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlay {
    pub name: String,
    pub hosts: HostPattern,
    pub vars: HashMap&lt;String, Value&gt;,
    pub tasks: Vec&lt;ParsedTask&gt;,
    pub handlers: Vec&lt;ParsedTask&gt;,
    pub roles: Vec&lt;ParsedRole&gt;,
    pub strategy: ExecutionStrategy,
    pub serial: Option&lt;u32&gt;,
    pub max_fail_percentage: Option&lt;f32&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTask {
    pub id: String,
    pub name: String,
    pub module: String,
    pub args: HashMap&lt;String, Value&gt;,
    pub vars: HashMap&lt;String, Value&gt;,
    pub when: Option&lt;String&gt;,
    pub loop_items: Option&lt;Value&gt;,
    pub tags: Vec&lt;String&gt;,
    pub notify: Vec&lt;String&gt;,
    pub changed_when: Option&lt;String&gt;,
    pub failed_when: Option&lt;String&gt;,
    pub ignore_errors: bool,
    pub delegate_to: Option&lt;String&gt;,
    pub dependencies: Vec&lt;String&gt;,
}

// Output format for parsed inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInventory {
    pub hosts: HashMap&lt;String, ParsedHost&gt;,
    pub groups: HashMap&lt;String, ParsedGroup&gt;,
    pub variables: HashMap&lt;String, Value&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedHost {
    pub name: String,
    pub address: Option&lt;String&gt;,
    pub port: Option&lt;u16&gt;,
    pub user: Option&lt;String&gt;,
    pub vars: HashMap&lt;String, Value&gt;,
    pub groups: Vec&lt;String&gt;,
}
```

### Parser API

```rust
pub struct PlaybookParser {
    vault_password: Option&lt;String&gt;,
    extra_vars: HashMap&lt;String, Value&gt;,
    template_engine: TemplateEngine,
    cache: Option&lt;ParseCache&gt;,
}

impl PlaybookParser {
    pub fn new() -&gt; Self;
    pub fn with_vault_password(mut self, password: String) -&gt; Self;
    pub fn with_extra_vars(mut self, vars: HashMap&lt;String, Value&gt;) -&gt; Self;
    pub fn with_cache(mut self, cache_dir: PathBuf) -&gt; Self;
    
    pub fn parse_playbook(&amp;self, path: &amp;Path) -&gt; Result&lt;ParsedPlaybook, ParseError&gt;;
    pub fn parse_inventory(&amp;self, path: &amp;Path) -&gt; Result&lt;ParsedInventory, ParseError&gt;;
    pub fn validate_syntax(&amp;self, path: &amp;Path) -&gt; Result&lt;(), ParseError&gt;;
    pub fn resolve_dependencies(&amp;self, playbook: &amp;ParsedPlaybook) -&gt; Vec&lt;String&gt;;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("YAML syntax error at line {line}, column {column}: {message}")]
    YamlSyntax { line: usize, column: usize, message: String },
    
    #[error("Template error in {file} at line {line}: {message}")]
    Template { file: String, line: usize, message: String },
    
    #[error("Variable '{variable}' is undefined")]
    UndefinedVariable { variable: String },
    
    #[error("Vault decryption failed: {message}")]
    VaultDecryption { message: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Invalid module '{module}' in task '{task}'")]
    InvalidModule { module: String, task: String },
    
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## File and Package Structure

```
src/bin/rustle-parse.rs         # Main binary entry point
src/parser/
├── mod.rs                      # Module exports
├── playbook.rs                 # Playbook parsing logic
├── inventory.rs                # Inventory parsing logic
├── template.rs                 # Template resolution
├── vault.rs                    # Vault decryption
├── cache.rs                    # Parse result caching
├── validator.rs                # Syntax validation
├── dependency.rs               # Dependency resolution
└── error.rs                    # Error types

src/types/
├── parsed.rs                   # Parsed data structures
└── output.rs                   # Output formatting

tests/parser/
├── playbook_tests.rs
├── inventory_tests.rs
├── template_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Basic YAML Parsing
1. Implement basic YAML deserialization using `serde_yaml`
2. Create core data structures for parsed playbooks
3. Add comprehensive error handling with line numbers
4. Implement basic validation for required fields

### Phase 2: Template Resolution
1. Integrate Jinja2 template engine (minijinja)
2. Implement variable resolution and substitution
3. Handle complex template expressions and filters
4. Add support for conditional includes

### Phase 3: Inventory Integration
1. Extend inventory parsing from existing code
2. Add support for dynamic inventory scripts
3. Implement variable inheritance and precedence
4. Add host pattern matching

### Phase 4: Advanced Features
1. Implement vault decryption
2. Add dependency graph generation
3. Implement result caching
4. Add performance optimizations

### Key Algorithms

**Dependency Resolution**:
```rust
fn resolve_task_dependencies(tasks: &amp;[ParsedTask]) -&gt; Result&lt;Vec&lt;String&gt;, ParseError&gt; {
    let mut graph = DiGraph::new();
    let mut task_indices = HashMap::new();
    
    // Build dependency graph
    for (i, task) in tasks.iter().enumerate() {
        let node = graph.add_node(task.id.clone());
        task_indices.insert(task.id.clone(), node);
        
        // Add edges for explicit dependencies
        for dep in &amp;task.dependencies {
            if let Some(dep_node) = task_indices.get(dep) {
                graph.add_edge(*dep_node, node, ());
            }
        }
        
        // Add edges for handler notifications
        for notify in &amp;task.notify {
            if let Some(handler_node) = task_indices.get(notify) {
                graph.add_edge(node, *handler_node, ());
            }
        }
    }
    
    // Topological sort for execution order
    petgraph::algo::toposort(&amp;graph, None)
        .map_err(|_| ParseError::CircularDependency { 
            cycle: "task dependency cycle detected".to_string() 
        })
        .map(|sorted| sorted.into_iter().map(|node| graph[node].clone()).collect())
}
```

**Template Resolution**:
```rust
fn resolve_templates(
    content: &amp;str, 
    vars: &amp;HashMap&lt;String, Value&gt;
) -&gt; Result&lt;String, ParseError&gt; {
    let mut env = minijinja::Environment::new();
    
    // Add Ansible-compatible filters
    env.add_filter("default", filters::default_filter);
    env.add_filter("mandatory", filters::mandatory_filter);
    env.add_filter("regex_replace", filters::regex_replace_filter);
    
    let template = env.template_from_str(content)
        .map_err(|e| ParseError::Template {
            file: "inline".to_string(),
            line: 0,
            message: e.to_string(),
        })?;
    
    template.render(vars)
        .map_err(|e| ParseError::Template {
            file: "inline".to_string(),
            line: 0,
            message: e.to_string(),
        })
}
```

## Testing Strategy

### Unit Tests
- **Parser modules**: Test individual parsing functions with various YAML inputs
- **Template engine**: Test variable resolution and filter functions
- **Validation**: Test error detection and reporting
- **Caching**: Test cache hit/miss scenarios

### Integration Tests
- **End-to-end parsing**: Test complete playbook parsing workflows
- **Error scenarios**: Test error handling with malformed YAML
- **Performance**: Benchmark parsing speed with large playbooks
- **Compatibility**: Test with real Ansible playbooks

### Test Data Structure
```
tests/fixtures/
├── playbooks/
│   ├── simple.yml              # Basic playbook
│   ├── complex.yml             # Multi-play with roles
│   ├── templates.yml           # Heavy template usage
│   ├── vault.yml               # Encrypted variables
│   └── invalid/                # Malformed playbooks
├── inventories/
│   ├── hosts.ini               # INI format
│   ├── hosts.yml               # YAML format
│   └── dynamic/                # Dynamic inventory scripts
└── expected/
    ├── simple.json             # Expected parsed output
    └── complex.json
```

### Mock Requirements
- File system mocking for testing file operations
- Network mocking for dynamic inventory scripts
- Time mocking for cache expiration tests

## Edge Cases &amp; Error Handling

### Input Validation
- Empty or malformed YAML files
- Circular includes and imports
- Missing required fields
- Invalid variable references
- Unsupported Ansible features

### Resource Management
- Large playbooks exceeding memory limits
- Deeply nested include structures
- Template recursion limits
- Cache storage limits

### Error Recovery
- Partial parsing on non-critical errors
- Graceful degradation for unsupported features
- Detailed error context with file/line information
- Suggestions for common syntax errors

## Dependencies

### External Crates
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
minijinja = "2"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tokio = { version = "1", features = ["fs"] }
petgraph = "0.6"
regex = "1"
base64 = "0.21"
sha2 = "0.10"
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle::vault` - Vault decryption
- `rustle::template` - Template engine

## Configuration

### Environment Variables
- `RUSTLE_CACHE_DIR`: Default cache directory
- `RUSTLE_VAULT_PASSWORD_FILE`: Default vault password file
- `RUSTLE_TEMPLATE_ENGINE`: Template engine selection (minijinja, tera)
- `RUSTLE_MAX_PARSE_TIME`: Maximum parsing time in seconds

### Configuration File Support
```toml
[parser]
cache_enabled = true
cache_dir = "~/.rustle/cache"
template_engine = "minijinja"
max_include_depth = 10
memory_limit_mb = 500

[output]
default_format = "json"
pretty_print = true
include_metadata = true
```

## Documentation

### CLI Help Text
```
rustle-parse - Parse Ansible playbooks and inventory files

USAGE:
    rustle-parse [OPTIONS] [PLAYBOOK_FILE]

ARGS:
    &lt;PLAYBOOK_FILE&gt;    Path to playbook file (or stdin if -)

OPTIONS:
    -i, --inventory &lt;FILE&gt;        Inventory file path
    -e, --extra-vars &lt;VARS&gt;       Extra variables (key=value,...)
    -o, --output &lt;FORMAT&gt;         Output format [default: json] [possible values: json, binary, yaml]
    -c, --cache-dir &lt;DIR&gt;         Cache directory for parsed results
    -v, --vault-password-file &lt;FILE&gt;  Vault password file
        --syntax-check            Only validate syntax, don't output
        --list-tasks              List all tasks with metadata
        --list-hosts              List all hosts with variables
        --verbose                 Enable verbose output
        --dry-run                 Parse but don't write output
    -h, --help                    Print help information
    -V, --version                 Print version information

EXAMPLES:
    rustle-parse playbook.yml                    # Parse and output JSON
    rustle-parse -i hosts.ini playbook.yml      # Include inventory
    rustle-parse --syntax-check playbook.yml    # Validate syntax only
    rustle-parse --list-tasks playbook.yml      # List all tasks
    echo "playbook content" | rustle-parse -    # Parse from stdin
```

### API Documentation
Comprehensive rustdoc documentation for all public APIs, including:
- Usage examples for each function
- Error handling patterns
- Performance considerations
- Compatibility notes

### Integration Examples
```bash
# Basic usage in pipeline
rustle-parse playbook.yml | rustle-plan | rustle-exec

# With inventory and variables
rustle-parse -i inventory.ini -e "env=prod,version=1.2.3" playbook.yml

# Syntax validation in CI
rustle-parse --syntax-check *.yml || exit 1

# Cache for repeated parsing
rustle-parse -c /tmp/cache playbook.yml
```