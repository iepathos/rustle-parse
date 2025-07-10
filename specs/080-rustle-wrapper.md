# Spec 080: Rustle Wrapper Tool

## Feature Summary

The `rustle` wrapper tool is the main command-line interface that orchestrates all specialized Rustle tools to provide a seamless, Ansible-compatible experience. It automatically chains together the appropriate tools based on user commands, manages tool configurations, and provides a unified interface for configuration management tasks.

**Problem it solves**: Provides a single entry point that maintains Ansible compatibility while leveraging the modular architecture of specialized tools underneath, enabling users to work with familiar commands while benefiting from the performance and flexibility of the modular design.

**High-level approach**: Create a wrapper binary that interprets Ansible-compatible commands, determines the appropriate tool chain, manages inter-tool communication through pipes and temporary files, and provides unified configuration and error handling.

## Goals & Requirements

### Functional Requirements
- Provide Ansible-compatible command-line interface
- Automatically orchestrate appropriate tool chains for common operations
- Support both simple commands and complex workflows
- Manage tool configurations and pass-through options
- Handle inter-tool communication and data flow
- Provide unified error handling and reporting
- Support direct tool invocation for advanced users
- Implement command aliases for common operations
- Cache intermediate results for performance
- Support plugin discovery and management

### Non-functional Requirements
- **Performance**: &lt;100ms overhead for tool orchestration
- **Compatibility**: 95%+ compatibility with ansible-playbook commands
- **Usability**: Intuitive command structure matching Ansible patterns
- **Flexibility**: Allow direct tool access when needed
- **Reliability**: Graceful handling of tool failures in the chain

### Success Criteria
- Existing Ansible users can use rustle with minimal learning curve
- Common operations complete 10x faster than Ansible
- Tool orchestration is transparent to users
- Advanced users can optimize workflows with direct tool access
- Error messages clearly indicate which tool in the chain failed

## API/Interface Design

### Command Line Interface
```bash
# Main wrapper command
rustle [GLOBAL_OPTIONS] COMMAND [COMMAND_OPTIONS]

# Ansible-compatible commands
rustle playbook [OPTIONS] PLAYBOOK.yml
rustle inventory [OPTIONS] --list
rustle vault [OPTIONS] COMMAND
rustle galaxy [OPTIONS] COMMAND
rustle console [OPTIONS] [HOST_PATTERN]

# Direct tool access
rustle tools parse [OPTIONS] PLAYBOOK.yml
rustle tools plan [OPTIONS] PARSED_PLAYBOOK
rustle tools exec [OPTIONS] EXECUTION_PLAN
rustle tools facts [OPTIONS] HOST_PATTERN
rustle tools connect [OPTIONS] COMMAND

# Utility commands
rustle init [PROJECT_NAME]
rustle validate PLAYBOOK.yml
rustle debug [OPTIONS] COMMAND
rustle config [OPTIONS]
rustle version --all

GLOBAL OPTIONS:
    -h, --help                     Show help information
    -V, --version                  Show version information
    -v, --verbose                  Increase verbosity (-vvv for debug)
    -q, --quiet                    Suppress non-error output
    --color &lt;WHEN&gt;                 Colorize output [auto|always|never]
    --config &lt;FILE&gt;                Specify config file
    --profile &lt;NAME&gt;               Use named configuration profile
    --log-file &lt;FILE&gt;              Log output to file
    --timing                       Show timing information
    --explain                      Show what tools would be invoked
    --no-cache                     Disable all caching
```

### Core Data Structures

```rust
// Command routing and orchestration
#[derive(Debug, Clone)]
pub struct CommandRoute {
    pub command: Command,
    pub tools: Vec&lt;ToolInvocation&gt;,
    pub pipeline: PipelineConfig,
    pub options: CommandOptions,
}

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub tool: Tool,
    pub args: Vec&lt;String&gt;,
    pub env: HashMap&lt;String, String&gt;,
    pub stdin: InputSource,
    pub stdout: OutputDestination,
    pub working_dir: Option&lt;PathBuf&gt;,
}

#[derive(Debug, Clone)]
pub enum Tool {
    Parse,
    Plan,
    Connect,
    Facts,
    Exec,
    Watch,
    Vault,
    Template,
    External { name: String, path: PathBuf },
}

#[derive(Debug, Clone)]
pub enum InputSource {
    Stdin,
    File(PathBuf),
    Pipe(usize), // Index of previous tool in pipeline
    Memory(Vec&lt;u8&gt;),
}

#[derive(Debug, Clone)]
pub enum OutputDestination {
    Stdout,
    File(PathBuf),
    Pipe,
    Memory,
    Null,
}

// Configuration management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustleConfig {
    pub profiles: HashMap&lt;String, ProfileConfig&gt;,
    pub tools: ToolsConfig,
    pub aliases: HashMap&lt;String, String&gt;,
    pub defaults: DefaultsConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    pub inventory: Option&lt;String&gt;,
    pub vault_password_file: Option&lt;String&gt;,
    pub private_key_file: Option&lt;String&gt;,
    pub remote_user: Option&lt;String&gt;,
    pub forks: Option&lt;u32&gt;,
    pub timeout: Option&lt;u32&gt;,
    pub env: HashMap&lt;String, String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub paths: HashMap&lt;String, PathBuf&gt;,
    pub defaults: HashMap&lt;String, HashMap&lt;String, Value&gt;&gt;,
    pub timeouts: HashMap&lt;String, u32&gt;,
}
```

### Command Router API

```rust
pub struct CommandRouter {
    config: RustleConfig,
    tool_registry: ToolRegistry,
    cache_manager: CacheManager,
}

impl CommandRouter {
    pub fn new(config: RustleConfig) -&gt; Result&lt;Self, RouterError&gt;;
    
    pub fn route_command(
        &amp;self,
        args: &amp;[String],
    ) -&gt; Result&lt;CommandRoute, RouterError&gt;;
    
    pub fn build_pipeline(
        &amp;self,
        route: &amp;CommandRoute,
    ) -&gt; Result&lt;Pipeline, RouterError&gt;;
    
    pub fn execute_pipeline(
        &amp;self,
        pipeline: Pipeline,
    ) -&gt; Result&lt;PipelineResult, RouterError&gt;;
    
    pub fn explain_command(
        &amp;self,
        args: &amp;[String],
    ) -&gt; Result&lt;String, RouterError&gt;;
}

// Pipeline execution
pub struct Pipeline {
    tools: Vec&lt;ToolProcess&gt;,
    connections: Vec&lt;PipeConnection&gt;,
    config: PipelineConfig,
}

impl Pipeline {
    pub async fn execute(&amp;mut self) -&gt; Result&lt;PipelineResult, PipelineError&gt;;
    pub fn abort(&amp;mut self) -&gt; Result&lt;(), PipelineError&gt;;
    pub fn get_progress(&amp;self) -&gt; PipelineProgress;
}

// Tool process management
pub struct ToolProcess {
    tool: Tool,
    process: Option&lt;Child&gt;,
    args: Vec&lt;String&gt;,
    status: ProcessStatus,
}

impl ToolProcess {
    pub fn spawn(&amp;mut self) -&gt; Result&lt;(), ProcessError&gt;;
    pub fn wait(&amp;mut self) -&gt; Result&lt;ExitStatus, ProcessError&gt;;
    pub fn kill(&amp;mut self) -&gt; Result&lt;(), ProcessError&gt;;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("Unknown command: {command}")]
    UnknownCommand { command: String },
    
    #[error("Tool '{tool}' not found in PATH or configured locations")]
    ToolNotFound { tool: String },
    
    #[error("Invalid command syntax: {message}")]
    InvalidSyntax { message: String },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Pipeline construction failed: {reason}")]
    PipelineError { reason: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Tool '{tool}' failed with exit code {code}")]
    ToolFailed { tool: String, code: i32 },
    
    #[error("Pipeline aborted by user")]
    Aborted,
    
    #[error("Inter-tool communication failed: {message}")]
    CommunicationError { message: String },
    
    #[error("Resource limit exceeded: {resource}")]
    ResourceExceeded { resource: String },
    
    #[error("Timeout in pipeline execution")]
    Timeout,
}
```

## File and Package Structure

```
src/bin/rustle.rs               # Main wrapper entry point
src/wrapper/
├── mod.rs                      # Module exports
├── router.rs                   # Command routing logic
├── pipeline.rs                 # Pipeline construction and execution
├── process.rs                  # Tool process management
├── config.rs                   # Configuration handling
├── cache.rs                    # Result caching
├── compat.rs                   # Ansible compatibility layer
├── explain.rs                  # Command explanation
└── error.rs                    # Error types

src/commands/
├── mod.rs                      # Command exports
├── playbook.rs                 # Playbook command implementation
├── inventory.rs                # Inventory command
├── vault.rs                    # Vault command
├── galaxy.rs                   # Galaxy command
├── console.rs                  # Interactive console
├── init.rs                     # Project initialization
└── debug.rs                    # Debug utilities

tests/wrapper/
├── router_tests.rs
├── pipeline_tests.rs
├── compat_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Core Router and Pipeline
1. Implement command parsing and routing logic
2. Create pipeline construction for tool chains
3. Add basic process management for tool execution
4. Implement inter-tool communication via pipes

### Phase 2: Ansible Compatibility
1. Add ansible-playbook command compatibility
2. Implement option translation between Ansible and Rustle tools
3. Add inventory and vault command support
4. Create compatibility testing framework

### Phase 3: Advanced Features
1. Add result caching between tool invocations
2. Implement command explanation mode
3. Add interactive console support
4. Create plugin discovery and management

### Phase 4: Performance and Polish
1. Optimize pipeline execution for common patterns
2. Add timing and profiling information
3. Implement advanced error recovery
4. Add shell completion support

### Key Algorithms

**Command Routing**:
```rust
impl CommandRouter {
    pub fn route_command(&amp;self, args: &amp;[String]) -&gt; Result&lt;CommandRoute, RouterError&gt; {
        let command = self.parse_command(args)?;
        
        let tools = match &amp;command {
            Command::Playbook { playbook, .. } =&gt; {
                vec![
                    ToolInvocation {
                        tool: Tool::Parse,
                        args: vec![playbook.to_string()],
                        stdin: InputSource::File(playbook.clone()),
                        stdout: OutputDestination::Pipe,
                        ..Default::default()
                    },
                    ToolInvocation {
                        tool: Tool::Plan,
                        args: self.build_plan_args(&amp;command),
                        stdin: InputSource::Pipe(0),
                        stdout: OutputDestination::Pipe,
                        ..Default::default()
                    },
                    ToolInvocation {
                        tool: Tool::Exec,
                        args: self.build_exec_args(&amp;command),
                        stdin: InputSource::Pipe(1),
                        stdout: OutputDestination::Stdout,
                        ..Default::default()
                    },
                ]
            }
            
            Command::Facts { pattern, .. } =&gt; {
                vec![
                    ToolInvocation {
                        tool: Tool::Facts,
                        args: vec![pattern.clone()],
                        stdin: InputSource::Stdin,
                        stdout: OutputDestination::Stdout,
                        ..Default::default()
                    },
                ]
            }
            
            Command::Check { playbook, .. } =&gt; {
                let mut tools = self.route_playbook_command(playbook, &amp;command)?;
                // Add --check flag to exec tool
                if let Some(exec_tool) = tools.iter_mut().find(|t| matches!(t.tool, Tool::Exec)) {
                    exec_tool.args.push("--check".to_string());
                }
                tools
            }
            
            _ =&gt; return Err(RouterError::UnknownCommand {
                command: format!("{:?}", command),
            }),
        };
        
        Ok(CommandRoute {
            command,
            tools,
            pipeline: self.build_pipeline_config(&amp;command),
            options: self.extract_options(args),
        })
    }
}
```

**Pipeline Execution with Caching**:
```rust
impl Pipeline {
    pub async fn execute(&amp;mut self) -&gt; Result&lt;PipelineResult, PipelineError&gt; {
        let mut results = Vec::new();
        let mut previous_output: Option&lt;Vec&lt;u8&gt;&gt; = None;
        
        for (i, tool) in self.tools.iter_mut().enumerate() {
            // Check cache for this tool's output
            if let Some(cached) = self.check_cache(&amp;tool).await? {
                tracing::debug!("Using cached result for tool: {:?}", tool.tool);
                previous_output = Some(cached);
                results.push(ToolResult::Cached);
                continue;
            }
            
            // Set up input based on pipeline configuration
            match &amp;tool.input {
                InputSource::Pipe(idx) if *idx &lt; i =&gt; {
                    if let Some(data) = &amp;previous_output {
                        tool.set_stdin(data.clone());
                    }
                }
                _ =&gt; {}
            }
            
            // Spawn and execute tool
            tool.spawn()?;
            
            let output = match &amp;tool.output {
                OutputDestination::Pipe | OutputDestination::Memory =&gt; {
                    let output = tool.capture_output().await?;
                    previous_output = Some(output.clone());
                    output
                }
                _ =&gt; {
                    tool.wait().await?;
                    Vec::new()
                }
            };
            
            // Cache successful results if configured
            if self.config.enable_caching &amp;&amp; tool.status == ProcessStatus::Success {
                self.cache_result(&amp;tool, &amp;output).await?;
            }
            
            results.push(ToolResult::Executed {
                exit_code: tool.get_exit_code(),
                duration: tool.get_duration(),
            });
        }
        
        Ok(PipelineResult {
            tools: results,
            total_duration: self.get_total_duration(),
            cached_tools: self.count_cached_tools(),
        })
    }
}
```

**Ansible Compatibility Layer**:
```rust
pub fn translate_ansible_args(args: &amp;[String]) -&gt; Result&lt;Vec&lt;String&gt;, CompatError&gt; {
    let mut rustle_args = Vec::new();
    let mut i = 0;
    
    while i &lt; args.len() {
        match args[i].as_str() {
            // Direct mappings
            "-i" | "--inventory" =&gt; {
                rustle_args.push("--inventory".to_string());
                if i + 1 &lt; args.len() {
                    rustle_args.push(args[i + 1].clone());
                    i += 1;
                }
            }
            
            // Flag translations
            "--check" =&gt; rustle_args.push("--check".to_string()),
            "--diff" =&gt; rustle_args.push("--diff".to_string()),
            "--syntax-check" =&gt; {
                // Route to parse tool only
                rustle_args.push("tools".to_string());
                rustle_args.push("parse".to_string());
                rustle_args.push("--syntax-check".to_string());
                return Ok(rustle_args);
            }
            
            // Option translations
            "--forks" =&gt; {
                rustle_args.push("--forks".to_string());
                if i + 1 &lt; args.len() {
                    rustle_args.push(args[i + 1].clone());
                    i += 1;
                }
            }
            
            // Environment variable mappings
            "--vault-password-file" =&gt; {
                if i + 1 &lt; args.len() {
                    std::env::set_var("RUSTLE_VAULT_PASSWORD_FILE", &amp;args[i + 1]);
                    i += 1;
                }
            }
            
            // Playbook should be last positional argument
            arg if !arg.starts_with('-') &amp;&amp; arg.ends_with(".yml") || arg.ends_with(".yaml") =&gt; {
                rustle_args.push("playbook".to_string());
                rustle_args.push(arg.to_string());
            }
            
            _ =&gt; rustle_args.push(args[i].clone()),
        }
        i += 1;
    }
    
    Ok(rustle_args)
}
```

## Testing Strategy

### Unit Tests
- **Command routing**: Test various command patterns and tool selection
- **Pipeline construction**: Test pipeline building for different workflows
- **Process management**: Test tool spawning and communication
- **Compatibility layer**: Test Ansible command translation
- **Cache management**: Test caching behavior and invalidation

### Integration Tests
- **End-to-end workflows**: Test complete playbook execution
- **Tool chain failures**: Test error handling in pipeline
- **Performance comparison**: Compare with direct Ansible execution
- **Compatibility testing**: Test with real Ansible playbooks
- **Resource management**: Test with large-scale operations

### Test Data Structure
```
tests/fixtures/
├── commands/
│   ├── ansible_commands.txt    # Real ansible-playbook commands
│   ├── rustle_equivalents.txt  # Expected rustle translations
│   └── edge_cases.txt          # Complex command scenarios
├── playbooks/
│   ├── simple.yml              # Basic test playbook
│   ├── complex.yml             # Multi-play playbook
│   └── ansible_compat/         # Real Ansible playbooks
└── configs/
    ├── default.toml            # Default configuration
    └── profiles/               # Test profiles
```

### Compatibility Testing
- Test suite of common Ansible commands
- Automated translation verification
- Output comparison with Ansible
- Performance benchmarking

## Edge Cases &amp; Error Handling

### Command Parsing
- Ambiguous command patterns
- Missing required arguments
- Conflicting options
- Unknown commands or options

### Tool Chain Failures
- Tool not found in PATH
- Tool crashes mid-pipeline
- Inter-tool communication failures
- Resource exhaustion during execution

### Compatibility Issues
- Unsupported Ansible features
- Version-specific Ansible syntax
- Plugin compatibility
- Custom module handling

### Performance Concerns
- Large pipeline construction overhead
- Cache invalidation strategies
- Memory usage with large data flows
- Concurrent pipeline execution

## Dependencies

### External Crates
```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
which = "5"
shellexpand = "3"
dirs = "5"
subprocess = "0.2"
```

### Internal Dependencies
- All rustle tool binaries
- Shared type definitions
- Common utilities

## Configuration

### Environment Variables
- `RUSTLE_CONFIG`: Configuration file path
- `RUSTLE_PROFILE`: Active profile name
- `RUSTLE_TOOLS_PATH`: Additional tool search paths
- `RUSTLE_CACHE_DIR`: Cache directory
- `RUSTLE_LOG_LEVEL`: Logging verbosity
- `ANSIBLE_*`: Support Ansible environment variables

### Configuration File Support
```toml
[defaults]
profile = "default"
color = "auto"
timing = false
cache_enabled = true
cache_dir = "~/.rustle/cache"

[tools]
parse = "rustle-parse"
plan = "rustle-plan"
exec = "rustle-exec"
facts = "rustle-facts"
connect = "rustle-connect"

[tools.defaults.parse]
cache_enabled = true
template_engine = "minijinja"

[tools.defaults.exec]
forks = 50
timeout = 600

[aliases]
deploy = "playbook -i production site.yml"
check = "playbook --check --diff"
facts = "tools facts -i"

[profiles.production]
inventory = "inventory/production.yml"
remote_user = "ansible"
private_key_file = "~/.ssh/production_key"
vault_password_file = "~/.vault_pass"

[cache]
enabled = true
ttl = 3600
max_size_mb = 1000
```

## Documentation

### CLI Help Text
```
rustle - High-performance configuration management with Ansible compatibility

USAGE:
    rustle [OPTIONS] &lt;COMMAND&gt;

COMMANDS:
    playbook    Execute an Ansible playbook
    inventory   Manage inventory
    vault       Encrypt/decrypt sensitive data
    galaxy      Manage roles and collections
    console     Interactive console
    tools       Direct access to specialized tools
    init        Initialize a new project
    validate    Validate playbooks and syntax
    config      Manage rustle configuration
    help        Print this message or the help of the given subcommand(s)

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
    -v, --verbose    Increase verbosity (-vvv for debug)
    -q, --quiet      Suppress non-error output
    --color &lt;WHEN&gt;   Colorize output [default: auto] [possible values: auto, always, never]
    --config &lt;FILE&gt;  Specify config file
    --profile &lt;NAME&gt; Use named configuration profile
    --explain        Show what tools would be invoked without executing

EXAMPLES:
    rustle playbook -i inventory.yml site.yml        # Run playbook
    rustle playbook --check site.yml                 # Check mode
    rustle inventory --list                          # List inventory
    rustle tools facts web_servers                   # Gather facts directly
    rustle explain playbook site.yml                 # Show tool pipeline

For Ansible users:
    Most ansible-playbook commands work with rustle:
    ansible-playbook -i hosts site.yml  →  rustle playbook -i hosts site.yml
```

### Migration Guide
Documentation for Ansible users including:
- Command mapping reference
- Feature compatibility matrix
- Performance comparison guide
- Common migration patterns

### Integration Examples
```bash
# Direct Ansible compatibility
rustle playbook -i inventory.yml --limit web --tags deploy site.yml

# Optimized pipeline with caching
rustle --profile production playbook deploy.yml

# Direct tool access for optimization
rustle tools parse playbook.yml | \
  rustle tools plan --optimize | \
  rustle tools exec --forks 100

# Debugging with explanation
rustle --explain playbook --check site.yml

# Using aliases
rustle deploy  # Expands to: playbook -i production site.yml
```