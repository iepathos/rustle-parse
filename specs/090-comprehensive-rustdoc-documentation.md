# Spec 090: Comprehensive Rustdoc Documentation

## Feature Summary

Add comprehensive rustdoc documentation to all public APIs in the rustle-parse codebase. This feature addresses the critical lack of documentation identified in the code quality evaluation, ensuring that all public structs, functions, methods, and modules have proper documentation comments with examples and clear descriptions of behavior, parameters, return values, and error conditions.

## Goals & Requirements

### Functional Requirements
- Document all public APIs with `///` rustdoc comments
- Add module-level documentation with `//!` comments
- Include code examples in documentation where appropriate
- Document all error conditions and edge cases
- Provide clear descriptions of parameters and return values
- Add usage examples for complex functionality

### Non-functional Requirements
- Documentation must pass `cargo doc` without warnings
- Examples in documentation must be compilable and testable
- Documentation should follow Rust community conventions
- Clear, concise writing that aids understanding
- Consistent formatting and terminology

### Success Criteria
- 100% documentation coverage for public APIs
- All documentation examples compile and run successfully
- No rustdoc warnings when building documentation
- Documentation is searchable and navigable via `cargo doc`
- Examples demonstrate real-world usage patterns

## API/Interface Design

### Documentation Format Standards

```rust
/// Brief one-line description of the item.
///
/// More detailed explanation of what this does, when to use it,
/// and any important considerations.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// Description of return value and possible states.
///
/// # Errors
///
/// Returns `ErrorType` when:
/// - Condition that causes error
/// - Another error condition
///
/// # Examples
///
/// ```
/// use rustle_parse::Parser;
/// 
/// let parser = Parser::new();
/// let result = parser.parse_playbook("playbook.yml").await?;
/// ```
///
/// # Panics
///
/// Panics if invariant is violated (if applicable).
pub fn example_function(param1: &str, param2: u32) -> Result<String, Error> {
    // Implementation
}
```

### Module Documentation Format

```rust
//! # Module Name
//!
//! Brief description of what this module provides.
//!
//! ## Overview
//!
//! Detailed explanation of the module's purpose and how it fits
//! into the larger system.
//!
//! ## Examples
//!
//! ```
//! use rustle_parse::parser::playbook;
//! 
//! // Example usage
//! ```
```

## File and Package Structure

### Files to Document

1. **Public API Entry Points**
   - `src/lib.rs` - Main library documentation
   - `src/bin/rustle-parse.rs` - CLI documentation

2. **Parser Module**
   - `src/parser/mod.rs` - Parser module overview
   - `src/parser/playbook.rs` - Playbook parsing APIs
   - `src/parser/inventory/*.rs` - Inventory parsing APIs
   - `src/parser/template.rs` - Template engine APIs
   - `src/parser/error.rs` - Error types documentation
   - `src/parser/validator.rs` - Validation APIs
   - `src/parser/cache.rs` - Cache functionality
   - `src/parser/dependency.rs` - Dependency resolution

3. **Types Module**
   - `src/types/mod.rs` - Types module overview
   - `src/types/parsed.rs` - Data structure documentation
   - `src/types/output.rs` - Output format documentation

## Implementation Details

### Step 1: Document Core Library API (src/lib.rs)

```rust
//! # rustle-parse
//!
//! A specialized YAML and inventory parser for Ansible-compatible playbooks.
//!
//! This crate provides high-performance parsing of Ansible playbooks and inventory
//! files with full compatibility for Ansible syntax including Jinja2 templates,
//! variable precedence, and complex data structures.
//!
//! ## Quick Start
//!
//! ```no_run
//! use rustle_parse::{Parser, OutputFormat};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let parser = Parser::new();
//!     let playbook = parser.parse_playbook("site.yml").await?;
//!     
//!     // Output as JSON
//!     let json = playbook.to_format(OutputFormat::Json)?;
//!     println!("{}", json);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - Full Ansible playbook YAML parsing
//! - Jinja2 template engine with Ansible filters
//! - INI and YAML inventory file support
//! - Variable precedence resolution
//! - Task dependency analysis
//! - Multiple output formats (JSON, YAML, Binary)

/// Re-export commonly used types
pub mod parser;
pub mod types;

pub use parser::{ParseError, Parser};
pub use types::output::OutputFormat;
pub use types::parsed::*;
```

### Step 2: Document Parser Struct and Methods

```rust
/// High-level parser for Ansible playbooks and inventory files.
///
/// The `Parser` struct provides the main interface for parsing Ansible
/// content. It handles template resolution, variable substitution, and
/// structural validation.
///
/// # Examples
///
/// ```no_run
/// use rustle_parse::Parser;
/// use std::collections::HashMap;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut parser = Parser::new()
///     .with_extra_vars(HashMap::from([
///         ("env".to_string(), "production".into()),
///     ]))
///     .with_cache("/tmp/rustle-cache");
///
/// let playbook = parser.parse_playbook("deploy.yml").await?;
/// # Ok(())
/// # }
/// ```
pub struct Parser {
    // fields
}

impl Parser {
    /// Creates a new parser with default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustle_parse::Parser;
    /// 
    /// let parser = Parser::new();
    /// ```
    pub fn new() -> Self {
        // implementation
    }

    /// Parses an Ansible playbook from the specified path.
    ///
    /// This method reads the YAML file, resolves all templates and includes,
    /// validates the structure, and returns a parsed representation.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the playbook YAML file
    ///
    /// # Returns
    ///
    /// Returns a `ParsedPlaybook` containing the structured representation
    /// of the playbook with all templates resolved.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` when:
    /// - File cannot be read (`ParseError::FileNotFound`)
    /// - YAML syntax is invalid (`ParseError::YamlError`)
    /// - Template syntax is invalid (`ParseError::TemplateError`)
    /// - Required variables are undefined (`ParseError::UndefinedVariable`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use rustle_parse::Parser;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let parser = Parser::new();
    /// let playbook = parser.parse_playbook("site.yml").await?;
    /// 
    /// for play in &playbook.plays {
    ///     println!("Play: {}", play.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn parse_playbook(&self, path: impl AsRef<Path>) -> Result<ParsedPlaybook, ParseError> {
        // implementation
    }
}
```

### Step 3: Document Error Types

```rust
/// Errors that can occur during parsing operations.
///
/// This enum represents all possible error conditions that can arise
/// when parsing Ansible playbooks and inventory files. Each variant
/// includes relevant context to help diagnose the issue.
///
/// # Examples
///
/// ```
/// use rustle_parse::{Parser, ParseError};
/// 
/// # async fn example() {
/// let parser = Parser::new();
/// match parser.parse_playbook("missing.yml").await {
///     Ok(playbook) => println!("Parsed successfully"),
///     Err(ParseError::FileNotFound { path, .. }) => {
///         eprintln!("File not found: {}", path);
///     }
///     Err(e) => eprintln!("Parse error: {}", e),
/// }
/// # }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// File could not be found or accessed.
    ///
    /// This typically occurs when the specified playbook or inventory
    /// file doesn't exist or permissions prevent reading.
    #[error("File not found: {path}")]
    FileNotFound {
        /// The path that could not be accessed
        path: String,
        /// The underlying IO error
        #[source]
        source: std::io::Error,
    },
    
    // ... other variants with documentation
}
```

### Step 4: Document Data Structures

```rust
/// Represents a parsed Ansible playbook.
///
/// This structure contains the complete parsed representation of an
/// Ansible playbook including all plays, variables, and metadata.
/// All templates have been resolved and validated.
///
/// # Examples
///
/// ```
/// use rustle_parse::ParsedPlaybook;
/// 
/// let playbook = ParsedPlaybook {
///     path: "/path/to/playbook.yml".into(),
///     plays: vec![],
///     global_vars: Default::default(),
/// };
/// 
/// // Convert to JSON
/// let json = serde_json::to_string_pretty(&playbook)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlaybook {
    /// Path to the source playbook file
    pub path: PathBuf,
    
    /// List of plays in execution order
    pub plays: Vec<ParsedPlay>,
    
    /// Global variables defined at playbook level
    pub global_vars: HashMap<String, serde_json::Value>,
}
```

## Testing Strategy

### Documentation Tests
- All code examples in documentation must be tested
- Use `cargo test --doc` to verify examples compile and run
- Examples should demonstrate realistic usage patterns

### Documentation Coverage
- Use `cargo doc --no-deps --document-private-items` to check coverage
- Verify all public items have documentation
- Check for broken links and references

### Review Process
1. Generate documentation with `cargo doc --open`
2. Review for clarity and completeness
3. Verify examples are helpful and correct
4. Check cross-references between modules

## Edge Cases & Error Handling

### Special Documentation Cases
- Generic types need clear type parameter documentation
- Trait implementations should document deviations from expected behavior
- Unsafe code blocks must document safety invariants
- Deprecated items need migration guidance

### Error Documentation
- Each error variant needs usage examples
- Document recovery strategies where applicable
- Include common causes and solutions

## Dependencies

### Documentation Tools
- `rustdoc` (built into Rust toolchain)
- Optional: `cargo-doc-coverage` for coverage metrics

### No Runtime Dependencies
- Documentation is compile-time only
- No impact on binary size or runtime performance

## Configuration

### Cargo.toml Documentation Settings
```toml
[package]
# ... existing fields ...

# Documentation metadata
documentation = "https://docs.rs/rustle-parse"
readme = "README.md"
keywords = ["ansible", "parser", "yaml", "playbook"]
categories = ["parser-implementations", "command-line-utilities"]

[package.metadata.docs.rs]
# Enable all features when building docs
all-features = true
# Generate docs for all targets
targets = ["x86_64-unknown-linux-gnu"]
```

### Documentation Build Configuration
```rust
// In lib.rs
#![doc(html_root_url = "https://docs.rs/rustle-parse/0.1.0")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
```

## Documentation

### README Updates
- Add "Documentation" section with link to docs.rs
- Include basic usage examples
- Reference the generated documentation

### Example Documentation Structure
```
rustle-parse/
├── src/
│   ├── lib.rs          (crate-level docs)
│   ├── parser/
│   │   ├── mod.rs      (module-level docs)
│   │   └── *.rs        (item-level docs)
│   └── types/
│       ├── mod.rs      (module-level docs)
│       └── *.rs        (item-level docs)
└── examples/
    ├── basic_usage.rs  (runnable example)
    └── advanced.rs     (complex example)
```

### Documentation Standards
- Use active voice and present tense
- Start with a brief one-line summary
- Provide context before diving into details
- Include "why" not just "what"
- Cross-reference related items with `[`links`]`

## Success Metrics

1. **Coverage**: 100% of public APIs documented
2. **Quality**: No rustdoc warnings or errors
3. **Usability**: Examples run without modification
4. **Clarity**: Documentation is clear to newcomers
5. **Completeness**: All parameters, returns, and errors documented