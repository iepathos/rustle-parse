# Claude Code Generation Guidelines for rustle-parse

## Project Overview

rustle-parse is a specialized YAML and inventory parser for Ansible-compatible playbooks built in Rust. This tool converts playbooks and inventory files into structured JSON/binary format and serves as a core component of the Rustle automation ecosystem. The project emphasizes performance, memory safety, and comprehensive error handling while maintaining full compatibility with Ansible syntax.

## Core Architecture Principles

### 1. Error Handling & Resource Management
- **Use Result types**: Prefer `Result<T, E>` over panics for recoverable errors
- **Explicit error handling**: Use `?` operator and proper error propagation
- **RAII pattern**: Rust's ownership system handles resource cleanup automatically
- **Custom error types**: Create domain-specific error types using `thiserror` or `anyhow`

```rust
// Good example
use anyhow::{Context, Result};

fn process_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;
    
    // Process content...
    Ok(content)
}
```

### 2. Concurrency & Thread Safety
- **Ownership model**: Leverage Rust's ownership system for thread safety
- **Async/await**: Use `tokio` for asynchronous programming
- **Channel communication**: Use `mpsc` channels for thread communication
- **Mutex/RwLock**: Use for shared mutable state when necessary

```rust
// Async example
use tokio::time::{sleep, Duration};

async fn fetch_data(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}
```

### 3. Configuration & Dependency Injection
- **Serde configuration**: Use `serde` for serialization/deserialization
- **Environment variables**: Use `dotenvy` for environment configuration
- **Dependency injection**: Pass dependencies explicitly through constructors
- **Feature flags**: Use Cargo features for conditional compilation

## File and Directory Structure

### Current Project Layout
```
rustle-parse/
├── src/                              # Source code
│   ├── bin/
│   │   └── rustle-parse.rs          # CLI binary entry point
│   ├── parser/                      # Core parsing modules
│   │   ├── mod.rs                   # Parser module exports
│   │   ├── playbook.rs              # Playbook YAML parsing
│   │   ├── inventory.rs             # Inventory file parsing
│   │   ├── template.rs              # Jinja2 template engine
│   │   ├── error.rs                 # Error types and handling
│   │   ├── vault.rs                 # Vault decryption support
│   │   ├── cache.rs                 # Parse result caching
│   │   ├── validator.rs             # Syntax validation
│   │   └── dependency.rs            # Task dependency resolution
│   ├── types/                       # Data structures
│   │   ├── mod.rs                   # Type module exports
│   │   ├── parsed.rs                # Parsed playbook/inventory structures
│   │   └── output.rs                # Output format definitions
│   └── lib.rs                       # Library entry point
├── tests/                           # Integration tests
│   ├── fixtures/                    # Test data
│   │   ├── playbooks/              # Sample playbooks
│   │   ├── inventories/            # Sample inventories
│   │   └── expected/               # Expected output
│   └── parser/                     # Test modules
├── specs/                          # Specification documents
├── target/                         # Build artifacts (gitignored)
├── Cargo.toml                      # Project manifest
├── Cargo.lock                      # Dependency lock file
├── CLAUDE.md                       # This file
└── README.md                       # Project documentation
```

### File Naming Conventions
- **Rust files**: Use snake_case (e.g., `user_service.rs`, `auth_handler.rs`)
- **Test files**: Integration tests in `tests/` directory
- **Module files**: `mod.rs` for module declarations
- **Binary targets**: Place in `src/bin/` for additional executables

## Code Style & Standards

### Documentation
- **Rustdoc comments**: Use `///` for public API documentation
- **Module documentation**: Document modules with `//!` at the top
- **Examples**: Include code examples in documentation
- **Cargo.toml metadata**: Include proper project metadata

```rust
/// Processes user authentication requests.
///
/// # Arguments
///
/// * `username` - The user's username
/// * `password` - The user's password
///
/// # Returns
///
/// Returns `Ok(User)` if authentication succeeds, or `Err(AuthError)` if it fails.
///
/// # Examples
///
/// ```
/// let user = authenticate("alice", "secret123")?;
/// println!("Welcome, {}!", user.name);
/// ```
pub fn authenticate(username: &str, password: &str) -> Result<User, AuthError> {
    // Implementation...
}
```

### Logging Standards
- **Structured logging**: Use `tracing` for structured logging
- **Log levels**: Use appropriate levels (trace, debug, info, warn, error)
- **Contextual logging**: Include relevant context with spans
- **Performance**: Use logging guards for expensive operations

```rust
use tracing::{info, debug, error, instrument};

#[instrument]
async fn process_request(request_id: u64) -> Result<Response> {
    debug!("Processing request {}", request_id);
    
    match handle_request(request_id).await {
        Ok(response) => {
            info!("Request {} processed successfully", request_id);
            Ok(response)
        }
        Err(e) => {
            error!("Failed to process request {}: {}", request_id, e);
            Err(e)
        }
    }
}
```

### Testing Requirements
- **Unit tests**: Include `#[cfg(test)]` modules in source files
- **Integration tests**: Place in `tests/` directory
- **Property testing**: Use `proptest` for property-based testing
- **Mocking**: Use `mockall` for mocking dependencies
- **Coverage**: Use `cargo tarpaulin` for code coverage

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_basic_functionality() {
        let result = process_data("test input");
        assert!(result.is_ok());
    }

    proptest! {
        #[test]
        fn test_property_based(input in ".*") {
            let result = validate_input(&input);
            prop_assert!(result.is_ok() || result.is_err());
        }
    }
}
```

## Platform-Specific Considerations

### Cross-Platform Compatibility
- **Conditional compilation**: Use `cfg` attributes for platform-specific code
- **Path handling**: Use `std::path::Path` for cross-platform path operations
- **Feature detection**: Use `cfg!` macro for runtime feature detection

```rust
#[cfg(target_os = "windows")]
fn platform_specific_function() {
    // Windows-specific implementation
}

#[cfg(unix)]
fn platform_specific_function() {
    // Unix-specific implementation
}
```

## Common Patterns & Anti-Patterns

### Do's
- ✅ Use `Result<T, E>` for error handling
- ✅ Leverage ownership and borrowing for memory safety
- ✅ Use iterators instead of manual loops
- ✅ Implement `Display` and `Debug` traits appropriately
- ✅ Use `clippy` for code quality checks
- ✅ Write comprehensive tests and documentation
- ✅ Use `serde` for serialization needs
- ✅ Follow Rust naming conventions

### Don'ts
- ❌ Don't use `unwrap()` in production code
- ❌ Don't use `panic!` for normal error flow
- ❌ Don't ignore compiler warnings
- ❌ Don't use `unsafe` without careful consideration
- ❌ Don't create unnecessary allocations
- ❌ Don't write untested code
- ❌ Don't use global mutable state

## Development Workflow

### Feature Development
1. **Design API**: Define public interfaces and types first
2. **Write tests**: Write failing tests before implementation
3. **Implement incrementally**: Build in small, testable increments
4. **Document thoroughly**: Include examples and edge cases
5. **Commit atomically**: Make small, focused commits

### Code Review Checklist
- [ ] Follows Rust idioms and conventions
- [ ] Proper error handling with `Result` types
- [ ] Comprehensive test coverage
- [ ] Clear documentation and examples
- [ ] No compiler warnings or clippy lints
- [ ] Appropriate use of lifetimes and borrowing
- [ ] Performance considerations addressed
- [ ] Security best practices followed

## Performance Considerations

### Memory Management
- **Zero-cost abstractions**: Leverage Rust's zero-cost abstractions
- **Avoid unnecessary allocations**: Use string slices over owned strings when possible
- **Iterator chains**: Use iterator adaptors for efficient data processing
- **Profiling**: Use `perf` and `flamegraph` for performance analysis

### Async Performance
- **Async runtime**: Choose appropriate async runtime (tokio, async-std)
- **Concurrent operations**: Use `join!` and `select!` for concurrency
- **Buffering**: Use appropriate buffer sizes for I/O operations
- **Connection pooling**: Implement connection pooling for database/network operations

## Security & Privacy

### Data Handling
- **Input validation**: Validate all external inputs
- **Sanitization**: Sanitize data before processing
- **Secure defaults**: Use secure defaults for configurations
- **Secrets management**: Never hardcode secrets in source code

### Memory Safety
- **Ownership system**: Rust's ownership prevents many security issues
- **Bounds checking**: Array bounds are checked at runtime
- **Type safety**: Use strong typing to prevent logic errors
- **Unsafe code**: Minimize and carefully review any `unsafe` blocks

## Tooling & Development Environment

### Essential Tools
- **Rustfmt**: Code formatting with `cargo fmt`
- **Clippy**: Linting with `cargo clippy`
- **Cargo**: Build system and package manager
- **Rust analyzer**: IDE integration for better development experience

### Code Search & Analysis
- **Ripgrep**: Fast text search with `rg`
  - `rg "pattern"` for basic search
  - `rg -t rust "pattern"` to search only Rust files
  - `rg -A 5 -B 5 "pattern"` for context lines
- **IDE integration**: Configure your editor for Rust development

### Testing Tools
- **Cargo test**: Built-in test runner
- **Tarpaulin**: Code coverage analysis
- **Criterion**: Benchmarking framework
- **Proptest**: Property-based testing

## Project Dependencies

### Core Dependencies (Already Configured)
- **serde** & **serde_yaml**: YAML parsing and serialization
- **serde_json**: JSON output format support
- **minijinja**: Jinja2 template engine with Ansible filters
- **clap**: Command-line argument parsing with derive macros
- **tokio**: Async runtime for file I/O operations
- **anyhow** & **thiserror**: Comprehensive error handling
- **tracing** & **tracing-subscriber**: Structured logging
- **chrono**: Date/time handling for metadata
- **sha2**: Cryptographic hashing for checksums
- **base64**: Base64 encoding/decoding for vault support
- **regex**: Regular expression support for filters
- **petgraph**: Graph algorithms for dependency resolution
- **configparser**: INI file parsing for inventories

### Development Dependencies
- **proptest**: Property-based testing framework
- **mockall**: Mocking framework for unit tests
- **criterion**: Benchmarking framework
- **tempfile**: Temporary file handling in tests
- **pretty_assertions**: Enhanced assertion output

## Example Prompts for Claude

### Implementing New Features
```
Implement complete INI inventory parsing in src/parser/inventory.rs. 
The current implementation is simplified - extend it to fully parse 
Ansible INI inventory format including groups, variables, and host patterns. 
Ensure compatibility with existing tests and add comprehensive test coverage.
```

### Fixing Issues
```
The template engine in src/parser/template.rs needs to handle complex 
Ansible template expressions. Add support for additional filters like 
'to_json', 'from_yaml', and 'b64encode'. Ensure all template errors 
include proper context with line numbers from the original YAML.
```

### Refactoring
```
Refactor the playbook parser to handle include_tasks and import_tasks directives.
The current parser in src/parser/playbook.rs only handles basic task structure.
Add support for task inclusion, role dependencies, and block constructs
while maintaining the existing ParsedPlaybook structure.
```

### Performance Optimization
```
Optimize the YAML parsing performance for large playbooks (>1000 tasks).
Profile the current implementation in src/parser/playbook.rs and identify
bottlenecks in template resolution and data structure conversion. Consider
parallel processing for independent tasks and implement result caching.
```

### Error Handling Enhancement
```
Enhance error reporting in src/parser/error.rs to include more context
from the original YAML files. Add line and column information for template
errors, undefined variables, and syntax issues. Ensure error messages
are actionable for users debugging their playbooks.
```

### CLI Feature Addition
```
Add support for reading playbooks from stdin in src/bin/rustle-parse.rs.
The current implementation has a placeholder for '-' input. Implement
proper stdin handling with temporary file creation for the YAML parser.
Include tests for this functionality.
```

## Project-Specific Guidelines

### Ansible Compatibility
- **Maintain 100% compatibility** with Ansible YAML syntax and semantics
- **Follow Ansible conventions** for variable precedence, template evaluation, and module arguments
- **Support standard Ansible features** like conditionals (`when`), loops (`with_items`), tags, and handlers
- **Preserve original behavior** for edge cases and error conditions

### Parser Implementation Patterns
- **Use serde for YAML parsing** with custom deserializers for complex structures
- **Implement template resolution** after YAML parsing but before final output
- **Handle circular dependencies** in task and role relationships
- **Validate module arguments** against known Ansible module schemas where possible

### Error Handling Specifics
- **Include file paths and line numbers** in all error messages
- **Provide actionable suggestions** for common syntax errors
- **Distinguish between parsing errors and validation errors**
- **Handle missing files gracefully** with clear error messages

### Testing Strategy
- **Use fixtures from real Ansible projects** to ensure compatibility
- **Test edge cases** like empty files, malformed YAML, and circular dependencies
- **Include performance tests** for large playbooks (>1000 tasks)
- **Mock external dependencies** like vault password files and dynamic inventory scripts

### Development Workflow
1. **Read the specification** (specs/030-rustle-parse.md) before making changes
2. **Write tests first** for new functionality
3. **Ensure all tests pass** before submitting changes
4. **Update documentation** to reflect new features or changes
5. **Run performance benchmarks** for parser-related changes

### Command Line Interface
- **Follow Ansible CLI conventions** where applicable
- **Provide clear help text** and usage examples
- **Support multiple output formats** with consistent behavior
- **Handle stdin input** for pipeline integration
- **Implement proper exit codes** for different error conditions

---

This guidance ensures Claude generates idiomatic, safe, and performant Rust code that follows community best practices and modern Rust development patterns while maintaining full compatibility with the Ansible ecosystem.