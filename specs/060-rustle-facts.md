# Spec 060: Rustle Facts Tool

## Feature Summary

The `rustle-facts` tool is a specialized system information gatherer that discovers and collects detailed information about target hosts. It provides fast, cacheable system profiling with modular fact collectors and serves as the foundation for conditional logic and system-aware configuration management.

**Problem it solves**: Centralizes system discovery and fact gathering, enabling other tools to make informed decisions based on accurate, cached system information without redundant data collection.

**High-level approach**: Create a standalone binary that connects to remote hosts, runs modular fact collection modules, and outputs structured system information in a standardized format with intelligent caching.

## Goals & Requirements

### Functional Requirements
- Gather comprehensive system facts from remote hosts
- Support modular fact collectors (OS, network, hardware, software)
- Provide intelligent caching and fact staleness detection
- Support both push and pull fact collection modes
- Generate structured output compatible with template engines
- Handle fact collection timeouts and partial failures gracefully
- Support custom fact collectors and plugins
- Provide fact filtering and selective collection
- Support fact validation and verification

### Non-functional Requirements
- **Performance**: Collect complete facts from 100 hosts in &lt;30 seconds
- **Accuracy**: 99.9% accuracy in system fact detection
- **Reliability**: Handle 95% of hosts even with partial connectivity issues
- **Caching**: Reduce fact collection time by 80% with intelligent caching
- **Memory**: &lt;10MB memory usage per concurrent fact collection

### Success Criteria
- Facts are compatible with Ansible fact format for migration
- Custom fact collectors can be easily added and distributed
- Fact caching eliminates redundant collection in 90% of scenarios
- All major operating systems and distributions are supported
- Performance is 5x+ faster than Ansible fact gathering

## API/Interface Design

### Command Line Interface
```bash
rustle-facts [OPTIONS] [HOST_PATTERN]

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Inventory file with hosts
    -l, --limit &lt;PATTERN&gt;          Limit to specific hosts
    --gather-subset &lt;SUBSET&gt;      Collect only specific fact subsets
    --exclude-subset &lt;SUBSET&gt;     Exclude specific fact subsets
    -c, --cache-dir &lt;DIR&gt;          Fact cache directory
    --cache-timeout &lt;SECONDS&gt;     Cache validity period [default: 3600]
    --no-cache                    Disable fact caching
    --force-refresh               Force cache refresh
    --custom-facts-dir &lt;DIR&gt;      Directory for custom fact collectors
    -f, --format &lt;FORMAT&gt;         Output format: json, yaml [default: json]
    -o, --output &lt;FILE&gt;           Output file (default: stdout)
    --tree &lt;DIR&gt;                  Save facts to directory tree structure
    --parallel &lt;NUM&gt;              Parallel fact collection processes [default: 50]
    --timeout &lt;SECONDS&gt;           Fact collection timeout per host [default: 30]
    --verify                      Verify fact accuracy with multiple methods
    --list-subsets                List available fact subsets
    --list-collectors             List available fact collectors
    -v, --verbose                 Enable verbose output
    --connection-plugin &lt;PLUGIN&gt;  Connection method [default: ssh]

ARGS:
    &lt;HOST_PATTERN&gt;  Host pattern to gather facts from (default: all)
```

### Core Data Structures

```rust
// Main facts output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostFacts {
    pub hostname: String,
    pub collected_at: DateTime&lt;Utc&gt;,
    pub collection_duration: Duration,
    pub rustle_version: String,
    pub facts: FactCollection,
    pub custom_facts: HashMap&lt;String, Value&gt;,
    pub collection_errors: Vec&lt;FactError&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCollection {
    // System facts
    pub ansible_system: String,
    pub ansible_os_family: String,
    pub ansible_distribution: String,
    pub ansible_distribution_version: String,
    pub ansible_kernel: String,
    pub ansible_architecture: String,
    pub ansible_hostname: String,
    pub ansible_fqdn: String,
    pub ansible_domain: String,
    
    // Hardware facts
    pub ansible_processor: Vec&lt;String&gt;,
    pub ansible_processor_count: u32,
    pub ansible_processor_cores: u32,
    pub ansible_processor_threads_per_core: u32,
    pub ansible_memtotal_mb: u64,
    pub ansible_memfree_mb: u64,
    pub ansible_swaptotal_mb: u64,
    pub ansible_devices: HashMap&lt;String, DeviceInfo&gt;,
    
    // Network facts
    pub ansible_interfaces: Vec&lt;String&gt;,
    pub ansible_default_ipv4: Option&lt;NetworkInterface&gt;,
    pub ansible_default_ipv6: Option&lt;NetworkInterface&gt;,
    pub ansible_all_ipv4_addresses: Vec&lt;String&gt;,
    pub ansible_all_ipv6_addresses: Vec&lt;String&gt;,
    
    // Software facts
    pub ansible_python_version: String,
    pub ansible_service_mgr: String,
    pub ansible_package_manager: String,
    pub ansible_env: HashMap&lt;String, String&gt;,
    pub ansible_mounts: Vec&lt;MountInfo&gt;,
    
    // Extended facts
    pub system_info: SystemInfo,
    pub performance_metrics: PerformanceMetrics,
    pub security_info: SecurityInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_type: String,
    pub size: Option&lt;u64&gt;,
    pub model: Option&lt;String&gt;,
    pub vendor: Option&lt;String&gt;,
    pub partitions: Vec&lt;PartitionInfo&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub interface: String,
    pub address: String,
    pub netmask: String,
    pub network: String,
    pub gateway: Option&lt;String&gt;,
    pub mtu: u32,
    pub type_field: String,
    pub macaddress: Option&lt;String&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub device: String,
    pub mount: String,
    pub fstype: String,
    pub options: Vec&lt;String&gt;,
    pub size_total: u64,
    pub size_available: u64,
}

// Fact collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCollectionConfig {
    pub gather_subset: Vec&lt;String&gt;,
    pub exclude_subset: Vec&lt;String&gt;,
    pub timeout: Duration,
    pub cache_enabled: bool,
    pub cache_timeout: Duration,
    pub custom_facts_dirs: Vec&lt;PathBuf&gt;,
    pub parallel_limit: u32,
    pub verify_facts: bool,
}
```

### Fact Collector API

```rust
pub trait FactCollector: Send + Sync {
    fn name(&amp;self) -&gt; &amp;str;
    fn subset(&amp;self) -&gt; &amp;str;
    fn dependencies(&amp;self) -&gt; Vec&lt;String&gt;;
    fn platforms(&amp;self) -&gt; Vec&lt;Platform&gt;;
    
    async fn collect(
        &amp;self,
        connection: &amp;dyn RemoteConnection,
        context: &amp;FactContext,
    ) -&gt; Result&lt;FactValue, FactError&gt;;
    
    fn cache_key(&amp;self, context: &amp;FactContext) -&gt; String;
    fn cache_ttl(&amp;self) -&gt; Duration;
    fn is_cacheable(&amp;self) -&gt; bool { true }
}

pub struct FactCollectionEngine {
    collectors: HashMap&lt;String, Box&lt;dyn FactCollector&gt;&gt;,
    cache: Option&lt;FactCache&gt;,
    config: FactCollectionConfig,
}

impl FactCollectionEngine {
    pub fn new(config: FactCollectionConfig) -&gt; Self;
    pub fn register_collector(&amp;mut self, collector: Box&lt;dyn FactCollector&gt;);
    pub fn load_custom_collectors(&amp;mut self, dir: &amp;Path) -&gt; Result&lt;(), FactError&gt;;
    
    pub async fn collect_facts(
        &amp;self,
        connection: &amp;dyn RemoteConnection,
        subset_filter: Option&lt;&amp;[String]&gt;,
    ) -&gt; Result&lt;HostFacts, FactError&gt;;
    
    pub async fn collect_facts_parallel(
        &amp;self,
        connections: Vec&lt;&amp;dyn RemoteConnection&gt;,
    ) -&gt; Vec&lt;Result&lt;HostFacts, FactError&gt;&gt;;
    
    pub fn list_available_subsets(&amp;self) -&gt; Vec&lt;String&gt;;
    pub fn list_collectors(&amp;self) -&gt; Vec&lt;&amp;dyn FactCollector&gt;;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error, Clone, Serialize, Deserialize)]
pub enum FactError {
    #[error("Connection failed to {host}: {reason}")]
    ConnectionFailed { host: String, reason: String },
    
    #[error("Command execution failed: {command}: {reason}")]
    CommandFailed { command: String, reason: String },
    
    #[error("Fact collector '{collector}' timed out after {timeout}s")]
    CollectorTimeout { collector: String, timeout: u64 },
    
    #[error("Unsupported platform for collector '{collector}': {platform}")]
    UnsupportedPlatform { collector: String, platform: String },
    
    #[error("Parse error in collector '{collector}': {message}")]
    ParseError { collector: String, message: String },
    
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    #[error("Permission denied accessing {resource}")]
    PermissionDenied { resource: String },
    
    #[error("Custom fact collector error in '{collector}': {message}")]
    CustomCollectorError { collector: String, message: String },
    
    #[error("Fact validation failed for '{fact}': expected {expected}, got {actual}")]
    ValidationFailed { fact: String, expected: String, actual: String },
}
```

## File and Package Structure

```
src/bin/rustle-facts.rs         # Main binary entry point
src/facts/
├── mod.rs                      # Module exports
├── engine.rs                   # Fact collection engine
├── cache.rs                    # Fact caching system
├── collectors/                 # Built-in fact collectors
│   ├── mod.rs
│   ├── system.rs              # System information
│   ├── hardware.rs            # Hardware details
│   ├── network.rs             # Network configuration
│   ├── software.rs            # Software inventory
│   ├── security.rs            # Security-related facts
│   ├── performance.rs         # Performance metrics
│   └── custom.rs              # Custom fact loader
├── platforms/                  # Platform-specific implementations
│   ├── mod.rs
│   ├── linux.rs              # Linux fact collection
│   ├── windows.rs            # Windows fact collection
│   ├── macos.rs              # macOS fact collection
│   └── bsd.rs                # BSD variants
├── output.rs                  # Output formatting
├── validation.rs              # Fact validation
└── error.rs                   # Error types

src/types/
├── facts.rs                   # Fact data structures
└── platform.rs               # Platform definitions

tests/facts/
├── engine_tests.rs
├── collectors_tests.rs
├── cache_tests.rs
├── validation_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Core Infrastructure
1. Implement basic fact collection engine
2. Create core data structures for system facts
3. Add connection abstraction for remote execution
4. Implement basic caching mechanism

### Phase 2: Built-in Collectors
1. Implement system information collectors
2. Add hardware discovery collectors
3. Create network interface collectors
4. Add software inventory collectors

### Phase 3: Platform Support
1. Add Linux-specific fact collection
2. Implement Windows fact gathering
3. Add macOS and BSD support
4. Create platform detection and routing

### Phase 4: Advanced Features
1. Add custom fact collector support
2. Implement fact validation and verification
3. Add performance monitoring facts
4. Create security-related fact collection

### Key Algorithms

**Parallel Fact Collection**:
```rust
async fn collect_facts_parallel(
    &amp;self,
    connections: Vec&lt;&amp;dyn RemoteConnection&gt;,
) -&gt; Vec&lt;Result&lt;HostFacts, FactError&gt;&gt; {
    let semaphore = Arc::new(Semaphore::new(self.config.parallel_limit as usize));
    let results = Arc::new(Mutex::new(Vec::new()));
    
    let tasks: Vec&lt;_&gt; = connections
        .into_iter()
        .enumerate()
        .map(|(index, connection)| {
            let semaphore = semaphore.clone();
            let results = results.clone();
            let engine = self.clone();
            
            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let start_time = Instant::now();
                let result = engine.collect_facts_single(connection).await;
                let duration = start_time.elapsed();
                
                tracing::info!(
                    "Fact collection for {} completed in {:?}",
                    connection.hostname(),
                    duration
                );
                
                let mut results_guard = results.lock().await;
                results_guard.push((index, result));
            })
        })
        .collect();
    
    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }
    
    // Sort results by original index to maintain order
    let mut final_results = results.lock().await;
    final_results.sort_by_key(|(index, _)| *index);
    
    final_results
        .into_iter()
        .map(|(_, result)| result)
        .collect()
}
```

**Intelligent Fact Caching**:
```rust
impl FactCache {
    async fn get_cached_facts(
        &amp;self,
        host: &amp;str,
        subset: &amp;[String],
    ) -&gt; Result&lt;Option&lt;HostFacts&gt;, FactError&gt; {
        let cache_key = self.build_cache_key(host, subset);
        let cache_path = self.cache_dir.join(&amp;cache_key);
        
        if !cache_path.exists() {
            return Ok(None);
        }
        
        let metadata = fs::metadata(&amp;cache_path).await?;
        let age = SystemTime::now()
            .duration_since(metadata.modified()?)
            .unwrap_or(Duration::MAX);
        
        if age &gt; self.config.cache_timeout {
            // Cache expired, remove stale entry
            let _ = fs::remove_file(&amp;cache_path).await;
            return Ok(None);
        }
        
        // Check if any subset requires refresh based on staleness rules
        let cached_facts: HostFacts = serde_json::from_slice(
            &amp;fs::read(&amp;cache_path).await?
        )?;
        
        for subset_name in subset {
            if self.is_subset_stale(&amp;cached_facts, subset_name, age) {
                return Ok(None);
            }
        }
        
        Ok(Some(cached_facts))
    }
    
    fn is_subset_stale(
        &amp;self,
        facts: &amp;HostFacts,
        subset: &amp;str,
        age: Duration,
    ) -&gt; bool {
        match subset {
            "network" =&gt; age &gt; Duration::from_secs(300), // Network changes frequently
            "hardware" =&gt; age &gt; Duration::from_secs(86400), // Hardware rarely changes
            "software" =&gt; age &gt; Duration::from_secs(3600), // Software changes occasionally
            "performance" =&gt; true, // Always refresh performance metrics
            _ =&gt; age &gt; self.config.cache_timeout,
        }
    }
}
```

**Platform-specific Fact Collection**:
```rust
struct LinuxSystemCollector;

impl FactCollector for LinuxSystemCollector {
    fn name(&amp;self) -&gt; &amp;str { "linux_system" }
    fn subset(&amp;self) -&gt; &amp;str { "system" }
    fn platforms(&amp;self) -&gt; Vec&lt;Platform&gt; { vec![Platform::Linux] }
    
    async fn collect(
        &amp;self,
        connection: &amp;dyn RemoteConnection,
        _context: &amp;FactContext,
    ) -&gt; Result&lt;FactValue, FactError&gt; {
        let mut facts = HashMap::new();
        
        // Collect OS release information
        let os_release = connection
            .execute_command("cat /etc/os-release")
            .await?;
        facts.extend(self.parse_os_release(&amp;os_release.stdout)?);
        
        // Collect kernel information
        let uname = connection
            .execute_command("uname -a")
            .await?;
        facts.extend(self.parse_uname(&amp;uname.stdout)?);
        
        // Collect hostname information
        let hostname = connection
            .execute_command("hostname -f")
            .await?;
        facts.insert(
            "ansible_fqdn".to_string(),
            Value::String(hostname.stdout.trim().to_string())
        );
        
        // Collect memory information
        let meminfo = connection
            .execute_command("cat /proc/meminfo")
            .await?;
        facts.extend(self.parse_meminfo(&amp;meminfo.stdout)?);
        
        Ok(FactValue::Object(facts))
    }
}

impl LinuxSystemCollector {
    fn parse_os_release(&amp;self, content: &amp;str) -&gt; Result&lt;HashMap&lt;String, Value&gt;, FactError&gt; {
        let mut facts = HashMap::new();
        
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let clean_value = value.trim_matches('"');
                match key {
                    "ID" =&gt; facts.insert("ansible_distribution".to_string(), Value::String(clean_value.to_string())),
                    "VERSION_ID" =&gt; facts.insert("ansible_distribution_version".to_string(), Value::String(clean_value.to_string())),
                    "ID_LIKE" =&gt; facts.insert("ansible_os_family".to_string(), Value::String(clean_value.to_string())),
                    _ =&gt; None,
                };
            }
        }
        
        Ok(facts)
    }
    
    fn parse_meminfo(&amp;self, content: &amp;str) -&gt; Result&lt;HashMap&lt;String, Value&gt;, FactError&gt; {
        let mut facts = HashMap::new();
        
        for line in content.lines() {
            if let Some((key, value_str)) = line.split_once(':') {
                if let Some(value_kb) = value_str.trim().split_whitespace().next() {
                    if let Ok(kb) = value_kb.parse::&lt;u64&gt;() {
                        let mb = kb / 1024;
                        match key {
                            "MemTotal" =&gt; facts.insert("ansible_memtotal_mb".to_string(), Value::Number(mb.into())),
                            "MemFree" =&gt; facts.insert("ansible_memfree_mb".to_string(), Value::Number(mb.into())),
                            "SwapTotal" =&gt; facts.insert("ansible_swaptotal_mb".to_string(), Value::Number(mb.into())),
                            _ =&gt; None,
                        };
                    }
                }
            }
        }
        
        Ok(facts)
    }
}
```

## Testing Strategy

### Unit Tests
- **Fact collectors**: Test individual collectors with mock data
- **Caching logic**: Test cache hit/miss scenarios and expiration
- **Platform detection**: Test OS and platform identification
- **Data parsing**: Test parsing of various system command outputs
- **Error handling**: Test behavior with command failures and timeouts

### Integration Tests
- **Multi-platform**: Test fact collection across different operating systems
- **Large-scale collection**: Test parallel collection from many hosts
- **Cache performance**: Test caching effectiveness and performance
- **Custom collectors**: Test loading and execution of custom fact collectors
- **Network resilience**: Test behavior with connectivity issues

### Test Data Structure
```
tests/fixtures/
├── platform_outputs/
│   ├── linux/
│   │   ├── ubuntu_20_04/      # Sample command outputs
│   │   ├── centos_8/
│   │   └── debian_11/
│   ├── windows/
│   │   ├── server_2019/
│   │   └── windows_10/
│   └── macos/
│       ├── big_sur/
│       └── monterey/
├── expected_facts/
│   ├── linux_ubuntu.json      # Expected fact output
│   ├── windows_server.json
│   └── macos_big_sur.json
└── custom_collectors/
    ├── example_collector.rs    # Sample custom collector
    └── test_collector.py      # Python custom collector
```

### Performance Benchmarks
- Fact collection time vs. number of hosts
- Cache hit ratio and performance improvement
- Memory usage during large-scale collection
- Custom collector performance impact

## Edge Cases &amp; Error Handling

### System Variations
- Missing system files or commands
- Different OS versions and distributions
- Containers vs. bare metal differences
- Virtualized environment detection

### Network and Connectivity
- Command execution timeouts
- Partial connectivity to hosts
- Permission denied for system commands
- Host unreachability during collection

### Data Quality
- Malformed command output
- Encoding issues in system data
- Missing or incomplete information
- Conflicting information from multiple sources

### Resource Constraints
- Memory limits during large collections
- File system space for caching
- Network bandwidth limitations
- CPU usage during intensive fact gathering

## Dependencies

### External Crates
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
shellexpand = "3"
dirs = "5"
sha2 = "0.10"
hex = "0.4"
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle-connect` - SSH connection management
- Shared platform detection utilities

## Configuration

### Environment Variables
- `RUSTLE_FACTS_CACHE_DIR`: Default cache directory
- `RUSTLE_FACTS_TIMEOUT`: Default collection timeout
- `RUSTLE_CUSTOM_FACTS_DIR`: Directory for custom collectors
- `RUSTLE_FACTS_PARALLEL`: Default parallelism level
- `RUSTLE_FACTS_CACHE_TTL`: Default cache timeout

### Configuration File Support
```toml
[facts]
cache_enabled = true
cache_dir = "~/.rustle/facts_cache"
cache_timeout_secs = 3600
parallel_limit = 50
collection_timeout_secs = 30
verify_facts = false

[custom_facts]
enabled = true
directories = ["~/.rustle/custom_facts", "/etc/rustle/facts.d"]
timeout_secs = 10

[collectors]
enabled_subsets = ["all"]
disabled_subsets = []

[subset_cache_overrides]
network = 300      # Network facts expire quickly
hardware = 86400   # Hardware facts expire slowly
performance = 0    # Performance facts never cached
```

## Documentation

### CLI Help Text
```
rustle-facts - Gather system facts from remote hosts

USAGE:
    rustle-facts [OPTIONS] [HOST_PATTERN]

ARGS:
    &lt;HOST_PATTERN&gt;    Host pattern to gather facts from [default: all]

OPTIONS:
    -i, --inventory &lt;FILE&gt;         Inventory file with hosts
    -l, --limit &lt;PATTERN&gt;          Limit to specific hosts
        --gather-subset &lt;SUBSET&gt;  Collect only specific fact subsets [possible values: all, hardware, network, virtual, ohai, facter]
        --exclude-subset &lt;SUBSET&gt; Exclude specific fact subsets
    -c, --cache-dir &lt;DIR&gt;          Fact cache directory
        --cache-timeout &lt;SECONDS&gt; Cache validity period [default: 3600]
        --no-cache                Disable fact caching
        --force-refresh           Force cache refresh
        --custom-facts-dir &lt;DIR&gt;  Directory for custom fact collectors
    -f, --format &lt;FORMAT&gt;         Output format [default: json] [possible values: json, yaml]
    -o, --output &lt;FILE&gt;           Output file (default: stdout)
        --tree &lt;DIR&gt;              Save facts to directory tree structure
        --parallel &lt;NUM&gt;          Parallel fact collection processes [default: 50]
        --timeout &lt;SECONDS&gt;       Fact collection timeout per host [default: 30]
        --verify                  Verify fact accuracy with multiple methods
        --list-subsets            List available fact subsets
        --list-collectors         List available fact collectors
    -v, --verbose                 Enable verbose output
    -h, --help                    Print help information
    -V, --version                 Print version information

EXAMPLES:
    rustle-facts web_servers                                    # Gather facts from web servers group
    rustle-facts --gather-subset network,hardware hosts.yml    # Collect specific fact subsets
    rustle-facts --no-cache --force-refresh all                # Force fresh fact collection
    rustle-facts --tree /tmp/facts hosts.yml                   # Save to directory structure
    rustle-facts --list-subsets                                # Show available fact categories
```

### API Documentation
Comprehensive rustdoc documentation including:
- Fact collector development guide
- Platform-specific implementation notes
- Performance optimization techniques
- Custom fact integration patterns

### Integration Examples
```bash
# Basic fact collection
rustle-facts inventory.yml &gt; host_facts.json

# Cached fact collection with specific subsets
rustle-facts --gather-subset network,software --cache-dir /tmp/cache hosts

# Integration with planning
rustle-facts inventory.yml | \
  rustle-parse -e @host_facts.json playbook.yml | \
  rustle-plan

# Custom fact collection
rustle-facts --custom-facts-dir ./custom_facts hosts.yml

# Fact verification and validation
rustle-facts --verify --verbose production_hosts.yml
```