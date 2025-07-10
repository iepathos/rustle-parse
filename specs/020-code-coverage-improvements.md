# Spec 020: Code Coverage Improvements

## Feature Summary

Implement comprehensive code coverage measurement, reporting, and quality improvements for the rustle-parse project. This will establish baseline coverage metrics, identify uncovered code paths, and implement automated coverage tracking to ensure high code quality and maintainability.

**Problem it solves**: Currently, the project lacks systematic code coverage measurement and enforcement, making it difficult to identify untested code paths and ensure comprehensive test coverage across all modules.

**High-level approach**: Set up cargo-tarpaulin for coverage measurement, establish coverage targets, identify and fill coverage gaps, and integrate coverage reporting into the development workflow.

## Goals & Requirements

### Functional Requirements
- Measure current code coverage across all source modules
- Generate HTML and text coverage reports
- Identify specific uncovered code paths and functions
- Add missing unit tests for core functionality
- Implement property-based testing for parser components
- Add integration tests for end-to-end scenarios
- Set up coverage tracking in CI/CD pipeline
- Establish minimum coverage thresholds

### Non-functional Requirements
- **Coverage Target**: Achieve minimum 85% line coverage across all modules
- **Performance**: Coverage measurement should not add more than 30% to test execution time
- **Automation**: Coverage reports should be generated automatically on each test run
- **Documentation**: All new tests must include clear documentation
- **Maintainability**: Test code should follow the same quality standards as production code

### Success Criteria
- All source modules have minimum 85% line coverage
- All public APIs have comprehensive test coverage
- Error handling paths are thoroughly tested
- Property-based tests validate parser correctness
- Coverage reports are generated on every CI run
- Coverage trends are tracked over time

## API/Interface Design

### Coverage Configuration
```toml
# Addition to Cargo.toml [dev-dependencies]
cargo-tarpaulin = "0.27"

# New workspace configuration
[workspace.metadata.tarpaulin]
coverage-reports = ["Html", "Lcov", "Json"]
output-dir = "coverage/"
exclude-files = ["target/*", "tests/*", "benches/*"]
fail-under = 85
timeout = 300
```

### Test Organization Structure
```rust
// Standard test module pattern for each source file
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use mockall::predicate::*;

    // Unit tests for core functionality
    #[test]
    fn test_basic_functionality() { /* ... */ }

    // Property-based tests for parsers
    proptest! {
        #[test]
        fn test_parser_properties(input in ".*") {
            // Test parser invariants
        }
    }

    // Error handling tests
    #[test]
    fn test_error_conditions() { /* ... */ }
}
```

### Coverage Reporting Interface
```rust
// New module: src/testing/coverage.rs
pub struct CoverageReport {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub uncovered_lines: Vec<(String, u32)>,
}

impl CoverageReport {
    pub fn generate() -> Result<Self, CoverageError>;
    pub fn meets_threshold(&self, threshold: f64) -> bool;
    pub fn export_html(&self, path: &Path) -> Result<(), CoverageError>;
}
```

## File and Package Structure

### New Files to Create
```
src/
├── testing/                    # New testing utilities module
│   ├── mod.rs                 # Testing module exports
│   ├── coverage.rs            # Coverage reporting utilities
│   ├── fixtures.rs            # Shared test fixtures
│   └── helpers.rs             # Test helper functions
├── parser/
│   ├── playbook.rs            # Enhanced with comprehensive tests
│   ├── inventory.rs           # Enhanced with comprehensive tests
│   ├── template.rs            # Enhanced with comprehensive tests
│   ├── vault.rs               # Enhanced with comprehensive tests
│   └── ...                    # All other modules with enhanced tests
└── ...

tests/
├── coverage/                  # Coverage-specific integration tests
│   ├── mod.rs
│   ├── edge_cases.rs          # Tests for edge cases and error conditions
│   └── property_based.rs      # Cross-module property-based tests
├── fixtures/
│   ├── coverage/              # Coverage test fixtures
│   │   ├── playbooks/         # Additional test playbooks
│   │   └── inventories/       # Additional test inventories
│   └── ...
└── ...

coverage/                      # Coverage output directory (gitignored)
├── html/                      # HTML coverage reports
├── tarpaulin-report.html      # Main coverage report
└── coverage.json              # Machine-readable coverage data
```

### Enhanced Justfile Commands
```justfile
# Coverage-related commands
coverage:
    cargo tarpaulin --out Html --out Lcov --out Json --output-dir coverage

coverage-open:
    cargo tarpaulin --out Html --output-dir coverage --open

coverage-check:
    cargo tarpaulin --fail-under 85 --output-dir coverage

coverage-ci:
    cargo tarpaulin --out Lcov --output-dir coverage --fail-under 85 --timeout 300
```

## Implementation Details

### Phase 1: Setup and Baseline Measurement
1. **Install and configure cargo-tarpaulin**
   - Add to development dependencies
   - Configure workspace metadata for tarpaulin
   - Set up coverage output directories

2. **Establish baseline coverage**
   - Run initial coverage measurement
   - Generate comprehensive report
   - Document current coverage by module

3. **Analyze coverage gaps**
   - Identify uncovered functions and code paths
   - Prioritize critical areas needing coverage
   - Create coverage improvement roadmap

### Phase 2: Core Module Coverage Enhancement
1. **Parser module improvements**
   ```rust
   // Example enhanced test for playbook.rs
   #[cfg(test)]
   mod tests {
       use super::*;
       use tempfile::NamedTempFile;
       use std::io::Write;

       #[test]
       fn test_parse_simple_playbook() {
           let yaml = r#"
           ---
           - hosts: all
             tasks:
               - name: test task
                 debug:
                   msg: "hello world"
           "#;
           
           let result = parse_playbook_content(yaml);
           assert!(result.is_ok());
           let playbook = result.unwrap();
           assert_eq!(playbook.plays.len(), 1);
       }

       #[test]
       fn test_parse_invalid_yaml() {
           let invalid_yaml = "invalid: yaml: content: [";
           let result = parse_playbook_content(invalid_yaml);
           assert!(result.is_err());
           assert!(matches!(result.unwrap_err(), ParseError::YamlError(_)));
       }

       proptest! {
           #[test]
           fn test_playbook_parse_doesnt_panic(content in ".*") {
               let _ = parse_playbook_content(&content);
               // Should never panic, only return errors
           }
       }
   }
   ```

2. **Template engine testing**
   - Test all Jinja2 filter implementations
   - Test variable resolution edge cases
   - Test template syntax error handling

3. **Inventory parser testing**
   - Test INI format parsing comprehensively
   - Test YAML inventory format
   - Test dynamic inventory handling
   - Test group and host variable resolution

### Phase 3: Error Handling and Edge Cases
1. **Comprehensive error path testing**
   ```rust
   #[test]
   fn test_file_not_found_error() {
       let result = parse_playbook_file("/nonexistent/file.yml");
       assert!(matches!(result.unwrap_err(), ParseError::IoError(_)));
   }

   #[test]
   fn test_vault_decryption_failure() {
       let encrypted_content = "!vault |\ninvalid_encrypted_content";
       let result = decrypt_vault_content(encrypted_content, "wrong_password");
       assert!(matches!(result.unwrap_err(), VaultError::DecryptionFailed));
   }
   ```

2. **Resource exhaustion testing**
   - Test behavior with very large files
   - Test memory usage under stress
   - Test timeout handling for long operations

### Phase 4: Integration and Property-Based Testing
1. **End-to-end integration tests**
   ```rust
   // tests/integration_tests.rs enhancement
   #[test]
   fn test_complete_parsing_pipeline() {
       let playbook_path = "tests/fixtures/playbooks/complex.yml";
       let inventory_path = "tests/fixtures/inventories/production.ini";
       
       let result = parse_complete_environment(playbook_path, inventory_path);
       assert!(result.is_ok());
       
       let parsed = result.unwrap();
       assert!(!parsed.playbooks.is_empty());
       assert!(!parsed.inventory.hosts.is_empty());
   }
   ```

2. **Property-based testing for parsers**
   ```rust
   proptest! {
       #[test]
       fn test_yaml_round_trip(playbook in arbitrary_valid_playbook()) {
           let serialized = serde_yaml::to_string(&playbook).unwrap();
           let parsed = parse_playbook_content(&serialized).unwrap();
           assert_eq!(playbook, parsed);
       }
   }
   ```

## Testing Strategy

### Unit Testing Requirements
- Every public function must have unit tests
- All error conditions must be tested
- Edge cases and boundary conditions covered
- Mock external dependencies (file system, network)

### Integration Testing Requirements
- End-to-end parsing workflows
- Multi-file scenarios (includes, imports)
- Real-world playbook examples
- Performance regression tests

### Property-Based Testing Requirements
- Parser correctness invariants
- Serialization round-trip testing
- Input validation behavior
- Template resolution properties

### Test Organization
```rust
// Standard test module structure
#[cfg(test)]
mod tests {
    use super::*;
    
    mod unit {
        use super::*;
        // Unit tests here
    }
    
    mod integration {
        use super::*;
        // Integration tests here
    }
    
    mod properties {
        use super::*;
        use proptest::prelude::*;
        // Property-based tests here
    }
}
```

## Edge Cases & Error Handling

### Critical Edge Cases to Test
1. **File System Edge Cases**
   - Empty files
   - Files with only whitespace
   - Files with invalid UTF-8 encoding
   - Symlinks and permissions issues

2. **YAML Parsing Edge Cases**
   - Deeply nested structures
   - Very long strings
   - Special characters and unicode
   - Malformed YAML syntax

3. **Template Processing Edge Cases**
   - Circular variable references
   - Undefined variables
   - Complex nested templates
   - Filter chain errors

4. **Vault Handling Edge Cases**
   - Corrupted vault data
   - Wrong password scenarios
   - Mixed encrypted/unencrypted content

### Error Recovery Patterns
```rust
// Comprehensive error handling test pattern
#[test]
fn test_graceful_error_recovery() {
    let scenarios = vec![
        ("empty_file", ""),
        ("invalid_yaml", "invalid: yaml: ["),
        ("missing_key", "hosts: missing"),
        ("circular_ref", "vars:\n  a: '{{ b }}'\n  b: '{{ a }}'"),
    ];
    
    for (name, content) in scenarios {
        let result = parse_playbook_content(content);
        assert!(result.is_err(), "Expected error for scenario: {}", name);
        
        // Verify error contains useful information
        let error = result.unwrap_err();
        assert!(!error.to_string().is_empty());
        assert!(error.line_number().is_some());
    }
}
```

## Dependencies

### New Development Dependencies
```toml
[dev-dependencies]
# Coverage measurement
cargo-tarpaulin = "0.27"

# Enhanced testing (already present but ensure latest versions)
proptest = "1.4"
mockall = "0.13"
tempfile = "3.8"
pretty_assertions = "1.4"

# Property testing generators
arbitrary = { version = "1.3", features = ["derive"] }
```

### Tool Dependencies
- **cargo-tarpaulin**: Primary coverage measurement tool
- **cargo-llvm-cov**: Alternative coverage tool (optional)
- **grcov**: Additional coverage analysis (optional)

## Configuration

### Workspace Configuration
```toml
# Cargo.toml additions
[workspace.metadata.tarpaulin]
coverage-reports = ["Html", "Lcov", "Json"]
output-dir = "coverage/"
exclude-files = [
    "target/*",
    "tests/*",
    "benches/*",
    "examples/*",
    "src/bin/*",  # Exclude CLI binary from coverage
]
timeout = 300
fail-under = 85
features = "default"
```

### Environment Variables
```bash
# Coverage configuration
export TARPAULIN_TIMEOUT=300
export TARPAULIN_FAIL_UNDER=85
export RUST_LOG=debug  # For coverage debugging
```

## Documentation

### Coverage Reporting Documentation
- Document how to run coverage analysis
- Explain coverage metrics and thresholds
- Provide guidelines for writing testable code
- Document CI/CD integration steps

### Test Writing Guidelines
```rust
/// Example of well-documented test
/// 
/// This test verifies that the playbook parser correctly handles
/// malformed YAML input by returning appropriate error types.
/// 
/// # Test Coverage
/// - YamlError variant of ParseError
/// - Error message contains line information
/// - Parser doesn't panic on invalid input
#[test]
fn test_malformed_yaml_handling() {
    // Given: Invalid YAML content
    let malformed_yaml = "key: value\ninvalid: [unclosed";
    
    // When: Parsing the content
    let result = parse_playbook_content(malformed_yaml);
    
    // Then: Should return YamlError
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, ParseError::YamlError(_)));
    assert!(error.to_string().contains("line"));
}
```

## Performance Considerations

### Coverage Measurement Impact
- Tarpaulin adds instrumentation overhead (~20-30% slower tests)
- Use cached coverage results during development
- Run full coverage analysis only on CI and before releases

### Test Performance Optimization
```rust
// Use test attributes for organization
#[test]
#[ignore = "slow"]  // Mark slow tests
fn test_large_file_parsing() {
    // Large file test that takes significant time
}

// Use conditional compilation for expensive tests
#[cfg(feature = "integration-tests")]
#[test] 
fn test_full_integration() {
    // Expensive integration test
}
```

## Security & Privacy

### Test Data Security
- No real credentials in test fixtures
- Use mock vault passwords for testing
- Sanitize any real-world examples used as fixtures

### Coverage Data Handling
- Coverage reports may contain source code snippets
- Ensure coverage artifacts are not committed to repository
- Configure .gitignore for coverage output directories

## Tooling & Development Environment

### Required Tools Installation
```bash
# Install coverage tools
cargo install cargo-tarpaulin

# Optional tools for enhanced coverage analysis
cargo install cargo-llvm-cov
cargo install grcov
```

### IDE Integration
- Configure VS Code/IntelliJ for coverage visualization
- Set up coverage gutters to show line coverage
- Configure test runners to include coverage

### CI/CD Integration
```yaml
# Example GitHub Actions workflow step
- name: Run tests with coverage
  run: |
    cargo tarpaulin --fail-under 85 --out Lcov --output-dir coverage
    
- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    file: coverage/lcov.info
```

## Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Install and configure cargo-tarpaulin
- [ ] Generate baseline coverage report
- [ ] Analyze current coverage gaps
- [ ] Set up coverage reporting infrastructure

### Phase 2: Core Coverage (Week 2-3)
- [ ] Add comprehensive parser module tests
- [ ] Implement error handling tests
- [ ] Add template engine test coverage
- [ ] Enhance inventory parser tests

### Phase 3: Advanced Testing (Week 4)
- [ ] Implement property-based tests
- [ ] Add integration test scenarios
- [ ] Test edge cases and error conditions
- [ ] Performance and stress testing

### Phase 4: Automation (Week 5)
- [ ] Integrate coverage into CI pipeline
- [ ] Set up coverage trend tracking
- [ ] Document testing procedures
- [ ] Establish coverage maintenance process

## Success Metrics

### Quantitative Metrics
- Line coverage ≥ 85% across all modules
- Branch coverage ≥ 80% for critical paths
- Function coverage ≥ 90% for public APIs
- Zero uncovered error handling paths

### Qualitative Metrics
- All tests have clear documentation
- Test code follows project style guidelines
- Coverage reports are actionable and clear
- Development workflow includes coverage checks

## Migration Strategy

### Gradual Implementation
1. Start with highest-priority modules (parser core)
2. Add tests incrementally to avoid overwhelming changes
3. Maintain backward compatibility with existing tests
4. Document coverage improvements in commit messages

### Risk Mitigation
- Run existing tests to ensure no regressions
- Use feature flags for experimental test coverage
- Maintain separate coverage thresholds during transition
- Regular coverage trend review to catch decreases early