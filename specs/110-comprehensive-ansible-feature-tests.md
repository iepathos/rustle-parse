# Spec 110: Comprehensive Ansible Feature Tests

## Feature Summary

Implement comprehensive test coverage for all major Ansible playbook features that are currently missing or inadequately tested. This includes advanced constructs like include/import directives, block constructs, complex conditionals, loops, error handling, and real-world compatibility scenarios. The goal is to ensure rustle-parse can handle the full spectrum of Ansible playbook features used in production environments.

## Goals & Requirements

### Functional Requirements
- Test all Ansible playbook constructs (blocks, includes, roles, conditionals)
- Verify complex variable precedence scenarios
- Test error handling and recovery mechanisms
- Validate template engine with advanced Jinja2 features
- Ensure compatibility with common Ansible patterns
- Test performance with large-scale playbooks (1000+ tasks)

### Non-functional Requirements
- Test execution time should remain under 30 seconds for full suite
- Memory usage during tests should not exceed 1GB
- Tests should be deterministic and reproducible
- Clear error messages when tests fail
- Support for parallel test execution

### Success Criteria
- 100% coverage of documented Ansible playbook features
- All tests pass consistently in CI/CD environment
- Performance benchmarks meet established targets
- Real-world playbook compatibility verified
- Property-based testing catches edge cases

## API/Interface Design

### Test Organization Structure

```rust
// tests/ansible_features/
mod blocks;           // Block construct tests
mod includes;         // Include/import directive tests
mod conditionals;     // When conditions and complex logic
mod loops;           // All loop types and patterns
mod variables;       // Variable precedence and scoping
mod handlers;        // Handler notification and execution
mod roles;           // Role inclusion and dependencies
mod templates;       // Advanced Jinja2 template features
mod error_handling;  // Error conditions and recovery
mod performance;     // Large-scale and performance tests
mod compatibility;   // Real-world playbook compatibility
```

### Test Data Management

```rust
/// Manages test fixtures and expected results
pub struct TestFixture {
    pub name: String,
    pub playbook_content: String,
    pub inventory_content: Option<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub expected_result: ExpectedResult,
    pub ansible_version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ExpectedResult {
    Success(ExpectedPlaybook),
    Error(ExpectedError),
    Warning(ExpectedWarning),
}

/// Builder for creating complex test scenarios
pub struct TestScenarioBuilder {
    name: String,
    playbook: PlaybookBuilder,
    inventory: Option<InventoryBuilder>,
    variables: HashMap<String, serde_json::Value>,
    expectations: Vec<Expectation>,
}

impl TestScenarioBuilder {
    pub fn new(name: &str) -> Self;
    pub fn with_playbook(mut self, builder: PlaybookBuilder) -> Self;
    pub fn with_inventory(mut self, builder: InventoryBuilder) -> Self;
    pub fn with_variable(mut self, key: &str, value: serde_json::Value) -> Self;
    pub fn expect_success(mut self) -> Self;
    pub fn expect_error(mut self, error_type: ExpectedErrorType) -> Self;
    pub fn expect_tasks_count(mut self, count: usize) -> Self;
    pub fn build(self) -> TestScenario;
}
```

## File and Package Structure

### Test Directory Structure
```
tests/
├── ansible_features/
│   ├── mod.rs                    # Common test utilities
│   ├── blocks/
│   │   ├── mod.rs
│   │   ├── basic_blocks.rs       # Simple block/rescue/always
│   │   ├── nested_blocks.rs      # Nested block structures
│   │   ├── error_handling.rs     # Block error scenarios
│   │   └── rescue_always.rs      # Rescue and always blocks
│   ├── includes/
│   │   ├── mod.rs
│   │   ├── include_tasks.rs      # include_tasks directive
│   │   ├── import_tasks.rs       # import_tasks directive
│   │   ├── include_playbook.rs   # include directive for playbooks
│   │   ├── conditional_includes.rs # Conditional inclusion
│   │   └── recursive_includes.rs # Nested includes
│   ├── conditionals/
│   │   ├── mod.rs
│   │   ├── basic_when.rs         # Simple when conditions
│   │   ├── complex_logic.rs      # Complex boolean logic
│   │   ├── variable_conditions.rs # Variable-based conditions
│   │   ├── fact_conditions.rs    # Fact-based conditions
│   │   └── template_conditions.rs # Template in conditions
│   ├── loops/
│   │   ├── mod.rs
│   │   ├── with_items.rs         # Basic with_items loops
│   │   ├── with_dict.rs          # Dictionary loops
│   │   ├── with_indexed_items.rs # Indexed loops
│   │   ├── with_nested.rs        # Nested loops
│   │   ├── with_subelements.rs   # Subelement loops
│   │   └── loop_control.rs       # Loop control variables
│   ├── variables/
│   │   ├── mod.rs
│   │   ├── precedence.rs         # Variable precedence rules
│   │   ├── scoping.rs            # Variable scope testing
│   │   ├── templating.rs         # Variable templating
│   │   ├── facts.rs              # Fact variables
│   │   └── defaults.rs           # Default variables
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── basic_handlers.rs     # Simple handler notification
│   │   ├── conditional_handlers.rs # Conditional handler execution
│   │   ├── handler_chains.rs     # Handler notification chains
│   │   └── meta_handlers.rs      # Meta handlers (flush, clear)
│   ├── roles/
│   │   ├── mod.rs
│   │   ├── basic_roles.rs        # Simple role inclusion
│   │   ├── role_dependencies.rs  # Role dependency resolution
│   │   ├── role_variables.rs     # Role variable handling
│   │   └── role_handlers.rs      # Role handler integration
│   ├── templates/
│   │   ├── mod.rs
│   │   ├── jinja2_features.rs    # Advanced Jinja2 syntax
│   │   ├── ansible_filters.rs    # Ansible-specific filters
│   │   ├── macros.rs             # Jinja2 macros
│   │   └── includes.rs           # Template includes
│   ├── error_handling/
│   │   ├── mod.rs
│   │   ├── syntax_errors.rs      # YAML syntax error handling
│   │   ├── template_errors.rs    # Template error scenarios
│   │   ├── undefined_vars.rs     # Undefined variable handling
│   │   └── file_errors.rs        # File access errors
│   ├── performance/
│   │   ├── mod.rs
│   │   ├── large_playbooks.rs    # Large-scale playbook tests
│   │   ├── memory_usage.rs       # Memory usage benchmarks
│   │   ├── parsing_speed.rs      # Parsing performance tests
│   │   └── concurrent_parsing.rs # Concurrent parsing tests
│   └── compatibility/
│       ├── mod.rs
│       ├── real_world.rs         # Real-world playbook tests
│       ├── ansible_versions.rs   # Version compatibility
│       └── edge_cases.rs         # Edge case scenarios
└── fixtures/
    ├── ansible_features/
    │   ├── blocks/
    │   ├── includes/
    │   ├── conditionals/
    │   ├── loops/
    │   ├── variables/
    │   ├── handlers/
    │   ├── roles/
    │   ├── templates/
    │   ├── performance/
    │   └── real_world/
    └── expected_results/
        └── [corresponding structure]
```

## Implementation Details

### Phase 1: Block Construct Testing

```rust
// tests/ansible_features/blocks/basic_blocks.rs

use crate::ansible_features::*;
use rustle_parse::{Parser, ParsedPlaybook};

#[tokio::test]
async fn test_basic_block_structure() {
    let scenario = TestScenarioBuilder::new("basic_block")
        .with_playbook(
            PlaybookBuilder::new()
                .with_play(
                    PlayBuilder::new("Test Block Play")
                        .with_block(
                            BlockBuilder::new()
                                .with_task("task1", "debug", json!({"msg": "in block"}))
                                .with_rescue(
                                    vec![TaskBuilder::new("rescue1", "debug", json!({"msg": "in rescue"}))]
                                )
                                .with_always(
                                    vec![TaskBuilder::new("always1", "debug", json!({"msg": "in always"}))]
                                )
                        )
                )
        )
        .expect_success()
        .expect_tasks_count(3) // block + rescue + always
        .build();

    let result = scenario.execute().await.unwrap();
    
    // Verify block structure
    let play = &result.plays[0];
    assert_eq!(play.tasks.len(), 1); // One block
    
    let block = &play.tasks[0];
    assert!(block.block.is_some());
    assert!(block.rescue.is_some());
    assert!(block.always.is_some());
}

#[tokio::test]
async fn test_nested_blocks() {
    let playbook_content = r#"
---
- name: Nested Blocks Test
  hosts: localhost
  tasks:
    - name: Outer block
      block:
        - name: Outer task 1
          debug:
            msg: "Outer task 1"
        
        - name: Inner block
          block:
            - name: Inner task 1
              debug:
                msg: "Inner task 1"
            - name: Inner task 2
              debug:
                msg: "Inner task 2"
          rescue:
            - name: Inner rescue
              debug:
                msg: "Inner rescue"
      rescue:
        - name: Outer rescue
          debug:
            msg: "Outer rescue"
      always:
        - name: Outer always
          debug:
            msg: "Outer always"
"#;

    let parser = Parser::new();
    let playbook = parser.parse_playbook_from_str(playbook_content).await.unwrap();
    
    // Verify nested structure is correctly parsed
    let outer_block = &playbook.plays[0].tasks[0];
    assert!(outer_block.block.is_some());
    
    let inner_tasks = outer_block.block.as_ref().unwrap();
    assert_eq!(inner_tasks.len(), 2);
    
    // Second task should be a nested block
    let inner_block = &inner_tasks[1];
    assert!(inner_block.block.is_some());
}
```

### Phase 2: Include/Import Directive Testing

```rust
// tests/ansible_features/includes/include_tasks.rs

#[tokio::test]
async fn test_include_tasks_basic() {
    let main_playbook = r#"
---
- name: Main Playbook
  hosts: localhost
  tasks:
    - name: Before include
      debug:
        msg: "Before include"
    
    - include_tasks: sub_tasks.yml
    
    - name: After include
      debug:
        msg: "After include"
"#;

    let sub_tasks = r#"
---
- name: Included task 1
  debug:
    msg: "Included task 1"

- name: Included task 2
  debug:
    msg: "Included task 2"
"#;

    let temp_dir = create_temp_directory_with_files(&[
        ("main.yml", main_playbook),
        ("sub_tasks.yml", sub_tasks),
    ]).await;

    let parser = Parser::new();
    let playbook = parser.parse_playbook(temp_dir.path().join("main.yml")).await.unwrap();
    
    // Should have 4 total tasks after include resolution
    assert_eq!(playbook.plays[0].tasks.len(), 4);
    
    // Verify task order
    assert_eq!(playbook.plays[0].tasks[0].name, "Before include");
    assert_eq!(playbook.plays[0].tasks[1].name, "Included task 1");
    assert_eq!(playbook.plays[0].tasks[2].name, "Included task 2");
    assert_eq!(playbook.plays[0].tasks[3].name, "After include");
}

#[tokio::test]
async fn test_conditional_include_tasks() {
    let playbook_content = r#"
---
- name: Conditional Include Test
  hosts: localhost
  vars:
    include_debug_tasks: true
  tasks:
    - include_tasks: debug_tasks.yml
      when: include_debug_tasks
    
    - include_tasks: prod_tasks.yml
      when: not include_debug_tasks
"#;

    let debug_tasks = r#"
---
- name: Debug task
  debug:
    msg: "Debug mode"
"#;

    let prod_tasks = r#"
---
- name: Production task
  debug:
    msg: "Production mode"
"#;

    let temp_dir = create_temp_directory_with_files(&[
        ("main.yml", playbook_content),
        ("debug_tasks.yml", debug_tasks),
        ("prod_tasks.yml", prod_tasks),
    ]).await;

    let parser = Parser::new();
    let playbook = parser.parse_playbook(temp_dir.path().join("main.yml")).await.unwrap();
    
    // Should only include debug tasks based on condition
    assert_eq!(playbook.plays[0].tasks.len(), 1);
    assert_eq!(playbook.plays[0].tasks[0].name, "Debug task");
}
```

### Phase 3: Complex Conditional Testing

```rust
// tests/ansible_features/conditionals/complex_logic.rs

#[tokio::test]
async fn test_complex_boolean_logic() {
    let test_cases = vec![
        ("simple_and", "var1 and var2", json!({"var1": true, "var2": true}), true),
        ("simple_or", "var1 or var2", json!({"var1": false, "var2": true}), true),
        ("complex_and_or", "(var1 and var2) or var3", json!({"var1": false, "var2": true, "var3": true}), true),
        ("negation", "not var1", json!({"var1": false}), true),
        ("comparison", "var1 > var2", json!({"var1": 10, "var2": 5}), true),
        ("string_comparison", "var1 == 'test'", json!({"var1": "test"}), true),
        ("in_operator", "'item' in var1", json!({"var1": ["item", "other"]}), true),
        ("defined_test", "var1 is defined", json!({"var1": "value"}), true),
        ("undefined_test", "var1 is not defined", json!({}), true),
    ];

    for (name, condition, vars, expected_included) in test_cases {
        let playbook_content = format!(r#"
---
- name: Complex Conditional Test - {}
  hosts: localhost
  vars: {}
  tasks:
    - name: Conditional task
      debug:
        msg: "Task executed"
      when: {}
"#, name, serde_json::to_string(&vars).unwrap(), condition);

        let parser = Parser::new();
        let playbook = parser.parse_playbook_from_str(&playbook_content).await.unwrap();
        
        if expected_included {
            assert_eq!(playbook.plays[0].tasks.len(), 1, 
                "Test case '{}' should include task", name);
        } else {
            assert_eq!(playbook.plays[0].tasks.len(), 0, 
                "Test case '{}' should not include task", name);
        }
    }
}
```

### Phase 4: Performance Testing

```rust
// tests/ansible_features/performance/large_playbooks.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use rustle_parse::Parser;

fn generate_large_playbook(num_tasks: usize) -> String {
    let mut playbook = String::from(r#"
---
- name: Large Playbook Performance Test
  hosts: localhost
  tasks:
"#);

    for i in 0..num_tasks {
        playbook.push_str(&format!(r#"
    - name: Task {}
      debug:
        msg: "This is task number {}"
      when: task_enabled | default(true)
      tags:
        - performance
        - task_{}
"#, i, i, i));
    }

    playbook
}

async fn parse_large_playbook(num_tasks: usize) {
    let playbook_content = generate_large_playbook(num_tasks);
    let parser = Parser::new();
    let _result = parser.parse_playbook_from_str(&playbook_content).await.unwrap();
}

fn benchmark_large_playbooks(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("large_playbook_parsing");
    
    for size in [100, 500, 1000, 2000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("tasks", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| parse_large_playbook(size));
            },
        );
    }
    
    group.finish();
}

#[tokio::test]
async fn test_memory_usage_large_playbook() {
    let initial_memory = get_memory_usage();
    
    // Parse a large playbook
    let playbook_content = generate_large_playbook(5000);
    let parser = Parser::new();
    let _result = parser.parse_playbook_from_str(&playbook_content).await.unwrap();
    
    let peak_memory = get_memory_usage();
    let memory_increase = peak_memory - initial_memory;
    
    // Should not use more than 100MB for 5000 tasks
    assert!(memory_increase < 100 * 1024 * 1024, 
        "Memory usage too high: {} bytes", memory_increase);
}

fn get_memory_usage() -> usize {
    // Platform-specific memory usage measurement
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").unwrap();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb: usize = line.split_whitespace().nth(1).unwrap().parse().unwrap();
                return kb * 1024;
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .unwrap();
        let rss_kb: usize = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        return rss_kb * 1024;
    }
    
    0 // Fallback for unsupported platforms
}

criterion_group!(benches, benchmark_large_playbooks);
criterion_main!(benches);
```

### Phase 5: Real-World Compatibility Testing

```rust
// tests/ansible_features/compatibility/real_world.rs

/// Tests with real-world playbooks from popular projects
#[tokio::test]
async fn test_kubernetes_ansible_compatibility() {
    // Example from kubernetes-ansible project
    let playbook_content = include_str!("../../../fixtures/real_world/kubernetes-setup.yml");
    
    let parser = Parser::new();
    let result = parser.parse_playbook_from_str(playbook_content).await;
    
    match result {
        Ok(playbook) => {
            assert!(!playbook.plays.is_empty());
            // Verify specific structures expected in k8s playbooks
            assert!(playbook.plays.iter().any(|play| 
                play.tasks.iter().any(|task| task.module == "package")));
        }
        Err(e) => panic!("Failed to parse kubernetes playbook: {}", e),
    }
}

#[tokio::test]
async fn test_ansible_galaxy_role_compatibility() {
    // Test with common patterns from Ansible Galaxy roles
    let role_playbook = r#"
---
- name: Example Galaxy Role Pattern
  hosts: all
  become: yes
  vars:
    app_name: "{{ role_name | default('myapp') }}"
    app_version: "{{ app_version | default('latest') }}"
  
  pre_tasks:
    - name: Update package cache
      package:
        update_cache: yes
      when: ansible_os_family in ['Debian', 'RedHat']
  
  roles:
    - role: common
      vars:
        common_packages:
          - git
          - curl
          - wget
    
    - role: "{{ app_name }}"
      vars:
        version: "{{ app_version }}"
      when: app_name is defined
  
  post_tasks:
    - name: Restart services
      service:
        name: "{{ item }}"
        state: restarted
      with_items: "{{ services_to_restart | default([]) }}"
      notify: service restarted
  
  handlers:
    - name: service restarted
      debug:
        msg: "Service {{ item }} was restarted"
"#;

    let parser = Parser::new();
    let playbook = parser.parse_playbook_from_str(role_playbook).await.unwrap();
    
    // Verify role inclusion is parsed correctly
    assert_eq!(playbook.plays[0].roles.len(), 2);
    assert_eq!(playbook.plays[0].roles[0].name, "common");
    
    // Verify conditional role inclusion
    assert!(playbook.plays[0].roles[1].when.is_some());
    
    // Verify handlers are parsed
    assert_eq!(playbook.plays[0].handlers.len(), 1);
}
```

## Testing Strategy

### Test Organization
- **Unit Tests**: Individual feature components
- **Integration Tests**: Feature combinations and interactions
- **Performance Tests**: Benchmarking with criterion
- **Property Tests**: Using proptest for edge case discovery
- **Compatibility Tests**: Real-world playbook validation

### Test Data Management
```rust
// tests/ansible_features/mod.rs

pub struct TestDataManager {
    fixture_dir: PathBuf,
}

impl TestDataManager {
    pub fn new() -> Self {
        Self {
            fixture_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ansible_features"),
        }
    }
    
    pub fn load_fixture(&self, category: &str, name: &str) -> Result<String, std::io::Error> {
        let path = self.fixture_dir.join(category).join(format!("{}.yml", name));
        std::fs::read_to_string(path)
    }
    
    pub fn create_temp_playbook(&self, content: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }
}

// Helper builders for test scenarios
pub struct PlaybookBuilder {
    plays: Vec<PlayBuilder>,
}

pub struct PlayBuilder {
    name: String,
    hosts: String,
    tasks: Vec<TaskBuilder>,
    vars: HashMap<String, serde_json::Value>,
    roles: Vec<RoleBuilder>,
    handlers: Vec<TaskBuilder>,
}

pub struct TaskBuilder {
    name: String,
    module: String,
    args: serde_json::Value,
    when: Option<String>,
    with_items: Option<serde_json::Value>,
    tags: Vec<String>,
    notify: Vec<String>,
}

// Implementation of builders...
```

### Property-Based Testing
```rust
use proptest::prelude::*;

// Property test for variable substitution
proptest! {
    #[test]
    fn test_variable_substitution_roundtrip(
        var_name in "[a-zA-Z][a-zA-Z0-9_]*",
        var_value in ".*"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        rt.block_on(async {
            let playbook_content = format!(r#"
---
- name: Property Test
  hosts: localhost
  vars:
    {}: "{}"
  tasks:
    - name: Test task
      debug:
        msg: "Value is {{{{ {} }}}}"
"#, var_name, var_value, var_name);

            let parser = Parser::new();
            let result = parser.parse_playbook_from_str(&playbook_content).await;
            
            // Should either parse successfully or fail with clear error
            match result {
                Ok(playbook) => {
                    // If successful, verify variable was substituted
                    let task = &playbook.plays[0].tasks[0];
                    if let Some(args) = task.args.as_object() {
                        if let Some(msg) = args.get("msg").and_then(|v| v.as_str()) {
                            prop_assert!(msg.contains(&var_value));
                        }
                    }
                }
                Err(_) => {
                    // If failed, should be due to invalid YAML or template syntax
                    // This is acceptable for property testing
                }
            }
        });
    }
}
```

## Edge Cases & Error Handling

### Complex Edge Cases to Test
1. **Circular Dependencies**: Include files that reference each other
2. **Deep Nesting**: Blocks nested 10+ levels deep
3. **Large Variables**: Variables with MB-sized content
4. **Unicode Content**: Playbooks with Unicode task names and content
5. **Malformed Templates**: Invalid Jinja2 syntax in various contexts
6. **Missing Files**: Include references to non-existent files
7. **Permission Issues**: Files that can't be read due to permissions
8. **Concurrent Access**: Multiple parsers accessing same files simultaneously

### Error Recovery Testing
```rust
#[tokio::test]
async fn test_graceful_error_handling() {
    let malformed_cases = vec![
        ("invalid_yaml", "---\n- name: Test\n  invalid: yaml: content"),
        ("undefined_variable", "---\n- name: Test\n  hosts: localhost\n  tasks:\n    - debug: msg={{ undefined_var }}"),
        ("invalid_when", "---\n- name: Test\n  hosts: localhost\n  tasks:\n    - debug: msg=test\n      when: invalid syntax here"),
        ("missing_include", "---\n- name: Test\n  hosts: localhost\n  tasks:\n    - include_tasks: nonexistent.yml"),
    ];

    for (case_name, content) in malformed_cases {
        let parser = Parser::new();
        let result = parser.parse_playbook_from_str(content).await;
        
        match result {
            Err(error) => {
                // Verify error provides useful information
                let error_str = error.to_string();
                assert!(!error_str.is_empty(), "Error message should not be empty for case: {}", case_name);
                // Error should contain context about what went wrong
                assert!(error_str.len() > 10, "Error message too short for case: {}", case_name);
            }
            Ok(_) => panic!("Expected error for case: {}", case_name),
        }
    }
}
```

## Dependencies

### Additional Test Dependencies
```toml
[dev-dependencies]
# Existing dependencies...

# Testing framework enhancements
proptest = "1.0"
criterion = { version = "0.6", features = ["html_reports"] }
mockall = "0.13"
tempfile = "3.0"
pretty_assertions = "1.4"

# Test data and fixtures
serde_yaml = "0.9"
serde_json = "1.0"

# Performance monitoring
memory-stats = "1.0"
```

## Configuration

### CI/CD Integration
```yaml
# .github/workflows/ansible-features.yml
name: Ansible Features Test Suite

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  ansible-features:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run Ansible feature tests
      run: |
        cargo test ansible_features --release
        
    - name: Run performance benchmarks
      run: |
        cargo bench --bench ansible_features
        
    - name: Run property tests
      run: |
        cargo test --release -- --test-threads=1 proptest
        
    - name: Generate coverage report
      run: |
        cargo tarpaulin --out Xml --output-dir coverage/
        
    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
```

### Test Configuration
```rust
// tests/ansible_features/config.rs

pub struct TestConfig {
    pub max_test_duration: Duration,
    pub memory_limit_mb: usize,
    pub enable_property_tests: bool,
    pub enable_performance_tests: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            max_test_duration: Duration::from_secs(30),
            memory_limit_mb: 1024,
            enable_property_tests: true,
            enable_performance_tests: cfg!(feature = "performance-tests"),
        }
    }
}
```

## Documentation

### Test Documentation
- Document test categories and what they cover
- Provide examples of how to add new test cases
- Document performance test interpretation
- Create troubleshooting guide for test failures

### Integration Examples
```rust
/// Example of testing a new Ansible feature
/// 
/// This example shows how to add tests for a new Ansible construct.
/// Follow this pattern when adding support for new features.
///
/// ```rust
/// #[tokio::test]
/// async fn test_new_feature() {
///     let playbook_content = r#"
///         # Your test playbook here
///     "#;
///     
///     let parser = Parser::new();
///     let result = parser.parse_playbook_from_str(playbook_content).await;
///     
///     // Add your assertions here
///     assert!(result.is_ok());
/// }
/// ```
```

## Success Metrics

1. **Coverage**: >95% of Ansible playbook features tested
2. **Performance**: All tests complete within 30 seconds
3. **Reliability**: Tests pass consistently in CI/CD (>99% success rate)
4. **Real-world Compatibility**: 100% of tested popular playbooks parse correctly
5. **Edge Case Detection**: Property tests find and help fix edge cases
6. **Documentation**: Complete documentation for adding new tests