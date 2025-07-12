# Spec 120: Modular Tool Integration

## Feature Summary

Implement the integration patterns and data flow mechanisms needed for rustle-parse to work seamlessly with other tools in the Rustle ecosystem: rustle-vault, rustle-template, rustle-plan, and the unified rustle wrapper.

**Problem it solves**: The modular architecture requires standardized communication between tools, proper data flow management, error handling across tool boundaries, and seamless pipeline composition while maintaining the ability to use tools independently.

**High-level approach**: Implement marker-based deferred processing, standardized JSON schemas, streaming integration, and comprehensive error handling with fallback strategies.

## Goals & Requirements

### Functional Requirements
- Generate vault and template markers during parsing
- Output standardized JSON format for tool composition
- Support streaming and non-streaming pipeline modes
- Handle tool integration via CLI flags and environment
- Implement error propagation and fallback strategies
- Support both automatic and manual tool orchestration

### Non-functional Requirements
- **Performance**: Streaming integration to prevent blocking
- **Reliability**: Comprehensive error handling with fallbacks
- **Compatibility**: Maintain backward compatibility with standalone operation
- **Flexibility**: Support both pipeline and individual tool usage
- **Security**: Secure data handling across tool boundaries

### Success Criteria
- Tools compose seamlessly in pipelines
- Error handling prevents cascade failures
- Performance remains optimal in pipeline mode
- Users can access individual tools for advanced workflows
- Backward compatibility maintained

## API/Interface Design

### Standard Data Exchange Format
```rust
use serde::{Deserialize, Serialize};

/// Standard format for inter-tool communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustleData {
    pub schema_version: String,
    pub tool_chain: Vec<String>,
    pub playbook: Option<ParsedPlaybook>,
    pub inventory: Option<ParsedInventory>,
    pub vault_markers: Vec<VaultMarker>,
    pub template_markers: Vec<TemplateMarker>,
    pub resolved_content: Option<ResolvedContent>,
    pub metadata: ToolMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub processing_timestamp: String,
    pub tool_versions: HashMap<String, String>,
    pub performance_metrics: Option<HashMap<String, f64>>,
    pub error_context: Option<ErrorContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedContent {
    pub vault_resolutions: HashMap<String, String>,  // location -> decrypted content
    pub template_resolutions: HashMap<String, String>, // location -> rendered content
}
```

### Enhanced CLI Interface
```rust
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "rustle-parse")]
pub struct ParseArgs {
    /// Input playbook file
    #[arg(value_name = "PLAYBOOK")]
    pub playbook: PathBuf,
    
    /// Inventory file
    #[arg(short, long)]
    pub inventory: Option<PathBuf>,
    
    /// Output format
    #[arg(long, default_value = "json")]
    pub output_format: OutputFormat,
    
    /// Defer vault decryption to pipeline
    #[arg(long)]
    pub defer_vault: bool,
    
    /// Defer complex templating to pipeline  
    #[arg(long)]
    pub defer_complex_templates: bool,
    
    /// Template complexity threshold (0-10)
    #[arg(long, default_value = "5")]
    pub template_complexity_threshold: u8,
    
    /// Include tool integration metadata
    #[arg(long)]
    pub include_metadata: bool,
    
    /// Read additional data from stdin
    #[arg(long)]
    pub stdin_data: bool,
    
    /// Vault password file for immediate processing
    #[arg(long)]
    pub vault_password_file: Option<PathBuf>,
    
    /// Template variables file
    #[arg(long)]
    pub template_vars: Option<PathBuf>,
}

#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    MessagePack,
    Yaml,
}
```

### Pipeline Integration Interface
```rust
pub struct PipelineIntegration;

impl PipelineIntegration {
    /// Read data from previous tool in pipeline
    pub async fn read_stdin_data() -> Result<Option<RustleData>, IntegrationError>;
    
    /// Merge data from previous tool with current parsing results
    pub fn merge_pipeline_data(
        current: ParsedOutput,
        pipeline_data: Option<RustleData>,
    ) -> Result<RustleData, IntegrationError>;
    
    /// Output data for next tool in pipeline
    pub fn output_pipeline_data(
        data: &RustleData,
        format: OutputFormat,
    ) -> Result<String, IntegrationError>;
    
    /// Detect when vault processing is needed
    pub fn requires_vault_processing(data: &RustleData) -> bool;
    
    /// Detect when template processing is needed
    pub fn requires_template_processing(data: &RustleData) -> bool;
    
    /// Generate shell command for automatic pipeline
    pub fn generate_pipeline_command(
        data: &RustleData,
        options: &PipelineOptions,
    ) -> Result<String, IntegrationError>;
}
```

### Tool Detection and Complexity Analysis
```rust
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Analyze template expression complexity
    pub fn analyze_template_complexity(expression: &str) -> ComplexityScore;
    
    /// Determine if vault processing should be deferred
    pub fn should_defer_vault(vault_content: &str, options: &IntegrationOptions) -> bool;
    
    /// Determine if template processing should be deferred
    pub fn should_defer_template(
        template_expr: &str, 
        threshold: u8,
    ) -> bool;
    
    /// Generate processing recommendations
    pub fn generate_processing_plan(data: &ParsedOutput) -> ProcessingPlan;
}

#[derive(Debug, Clone)]
pub struct ComplexityScore {
    pub score: u8,  // 0-10
    pub factors: Vec<ComplexityFactor>,
}

#[derive(Debug, Clone)]
pub enum ComplexityFactor {
    MultipleFilters,
    NestedExpressions,
    ConditionalLogic,
    LoopConstruct,
    CustomFunctions,
    LargeDataStructures,
}

#[derive(Debug, Clone)]
pub struct ProcessingPlan {
    pub vault_processing: ProcessingMode,
    pub template_processing: ProcessingMode,
    pub estimated_tools: Vec<String>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum ProcessingMode {
    Immediate,
    Deferred,
    Optional,
}
```

## Implementation Details

### Phase 1: Marker Generation
```rust
// src/parser/integration.rs
use jsonpath_lib as jsonpath;

impl IntegrationMarkers {
    /// Create vault marker with JSONPath location
    pub fn create_vault_marker(
        content: &str,
        location: &JsonPath,
        source_file: &str,
        line: Option<usize>,
    ) -> Result<VaultMarker, ParseError> {
        let metadata = VaultDetector::extract_metadata(content)?;
        
        Ok(VaultMarker {
            location: location.to_string(),
            vault_id: metadata.vault_id,
            format_version: metadata.format_version,
            encrypted_data: content.to_string(),
            source_context: SourceContext {
                file: source_file.to_string(),
                line,
                column: None,
            },
        })
    }
    
    /// Create template marker with complexity analysis
    pub fn create_template_marker(
        expression: &str,
        location: &JsonPath,
        context: &TemplateContext,
    ) -> Result<TemplateMarker, ParseError> {
        let complexity = ComplexityAnalyzer::analyze_template_complexity(expression);
        let dependencies = TemplateAnalyzer::extract_dependencies(expression)?;
        
        Ok(TemplateMarker {
            location: location.to_string(),
            expression: expression.to_string(),
            complexity_score: complexity.score,
            required_vars: dependencies,
            processing_hints: complexity.factors,
            source_context: context.error_context.clone(),
        })
    }
    
    /// Track markers during parsing recursively
    pub fn track_markers_in_value(
        value: &mut serde_json::Value,
        path: &JsonPath,
        markers: &mut IntegrationMarkers,
    ) -> Result<(), ParseError> {
        match value {
            serde_json::Value::String(s) => {
                if VaultDetector::is_vault_encrypted(s) {
                    let marker = Self::create_vault_marker(s, path, "playbook.yml", None)?;
                    markers.vault_markers.push(marker);
                } else if TemplateDetector::has_complex_expressions(s) {
                    let marker = Self::create_template_marker(s, path, &TemplateContext::default())?;
                    markers.template_markers.push(marker);
                }
            }
            serde_json::Value::Object(obj) => {
                for (key, val) in obj.iter_mut() {
                    let new_path = path.extend(key);
                    Self::track_markers_in_value(val, &new_path, markers)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for (index, val) in arr.iter_mut().enumerate() {
                    let new_path = path.extend_index(index);
                    Self::track_markers_in_value(val, &new_path, markers)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

### Phase 2: Pipeline Data Flow
```rust
// src/integration/pipeline.rs
impl PipelineIntegration {
    pub async fn read_stdin_data() -> Result<Option<RustleData>, IntegrationError> {
        let mut stdin = tokio::io::stdin();
        let mut buffer = String::new();
        
        // Try to read from stdin with timeout
        let read_result = tokio::time::timeout(
            Duration::from_millis(100),
            stdin.read_to_string(&mut buffer)
        ).await;
        
        match read_result {
            Ok(Ok(_)) if !buffer.trim().is_empty() => {
                // Parse JSON data from previous tool
                let data: RustleData = serde_json::from_str(&buffer)
                    .map_err(|e| IntegrationError::InvalidPipelineData {
                        message: format!("Failed to parse pipeline data: {}", e),
                    })?;
                Ok(Some(data))
            }
            _ => Ok(None), // No pipeline data
        }
    }
    
    pub fn merge_pipeline_data(
        current: ParsedOutput,
        pipeline_data: Option<RustleData>,
    ) -> Result<RustleData, IntegrationError> {
        let mut result = RustleData {
            schema_version: "1.0".to_string(),
            tool_chain: vec!["rustle-parse".to_string()],
            playbook: Some(current.playbook),
            inventory: current.inventory,
            vault_markers: current.vault_markers,
            template_markers: current.template_markers,
            resolved_content: None,
            metadata: ToolMetadata {
                processing_timestamp: chrono::Utc::now().to_rfc3339(),
                tool_versions: Self::collect_tool_versions(),
                performance_metrics: current.performance_metrics,
                error_context: None,
            },
        };
        
        if let Some(pipeline) = pipeline_data {
            // Merge data from previous tools
            result.tool_chain.extend(pipeline.tool_chain);
            result.resolved_content = pipeline.resolved_content;
            
            // Apply any resolved content to current data
            if let Some(resolved) = &pipeline.resolved_content {
                Self::apply_resolutions(&mut result, resolved)?;
            }
        }
        
        Ok(result)
    }
    
    fn apply_resolutions(
        data: &mut RustleData,
        resolved: &ResolvedContent,
    ) -> Result<(), IntegrationError> {
        // Apply vault resolutions
        for (location, decrypted_content) in &resolved.vault_resolutions {
            if let Some(playbook) = &mut data.playbook {
                Self::update_value_at_path(
                    &mut playbook.content,
                    location,
                    serde_json::Value::String(decrypted_content.clone()),
                )?;
            }
        }
        
        // Apply template resolutions
        for (location, rendered_content) in &resolved.template_resolutions {
            if let Some(playbook) = &mut data.playbook {
                Self::update_value_at_path(
                    &mut playbook.content,
                    location,
                    serde_json::Value::String(rendered_content.clone()),
                )?;
            }
        }
        
        Ok(())
    }
}
```

### Phase 3: Error Handling and Fallbacks
```rust
// src/integration/error.rs
#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("Invalid pipeline data: {message}")]
    InvalidPipelineData { message: String },
    
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },
    
    #[error("Pipeline communication failed: {message}")]
    PipelineCommunicationFailed { message: String },
    
    #[error("Tool execution failed: {tool}: {message}")]
    ToolExecutionFailed { tool: String, message: String },
    
    #[error("Data format incompatible: {message}")]
    IncompatibleDataFormat { message: String },
}

impl IntegrationErrorHandler {
    /// Handle vault processing errors with fallback
    pub fn handle_vault_error(
        error: VaultError,
        context: &ErrorContext,
    ) -> Result<FallbackAction, IntegrationError> {
        match error {
            VaultError::NoPassword { .. } => {
                // Fallback: defer to pipeline or fail gracefully
                Ok(FallbackAction::DeferToMarker)
            }
            VaultError::DecryptionFailed { .. } => {
                // Fallback: preserve encrypted content with warning
                Ok(FallbackAction::PreserveEncrypted)
            }
            VaultError::InvalidFormat { .. } => {
                // Fail: invalid format cannot be recovered
                Err(IntegrationError::ToolExecutionFailed {
                    tool: "rustle-vault".to_string(),
                    message: error.to_string(),
                })
            }
        }
    }
    
    /// Handle template processing errors with fallback
    pub fn handle_template_error(
        error: TemplateError,
        template: &str,
        context: &ErrorContext,
    ) -> Result<FallbackAction, IntegrationError> {
        if template.len() < 100 && !template.contains("{{") {
            // Simple string, no templating needed
            Ok(FallbackAction::UseAsLiteral)
        } else if Self::is_basic_template(template) {
            // Try processing with basic engine
            Ok(FallbackAction::ProcessWithBasicEngine)
        } else {
            // Defer to rustle-template
            Ok(FallbackAction::DeferToMarker)
        }
    }
}

#[derive(Debug, Clone)]
pub enum FallbackAction {
    DeferToMarker,
    PreserveEncrypted,
    UseAsLiteral,
    ProcessWithBasicEngine,
    FailGracefully,
}
```

### Phase 4: Command Generation for Pipeline
```rust
// src/integration/pipeline_generator.rs
impl PipelineGenerator {
    /// Generate complete pipeline command
    pub fn generate_pipeline_command(
        data: &RustleData,
        options: &PipelineOptions,
    ) -> Result<String, IntegrationError> {
        let mut commands = vec!["rustle-parse".to_string()];
        
        // Add source arguments
        commands.push(format!("\"{}\"", options.playbook_path.display()));
        
        if let Some(inventory) = &options.inventory_path {
            commands.push(format!("-i \"{}\"", inventory.display()));
        }
        
        // Configure processing modes
        if Self::should_defer_vault(data, options) {
            commands.push("--defer-vault".to_string());
        }
        
        if Self::should_defer_templates(data, options) {
            commands.push("--defer-complex-templates".to_string());
        }
        
        // Add pipeline tools
        let mut pipeline = vec![commands.join(" ")];
        
        if Self::requires_vault_processing(data) {
            let vault_cmd = Self::generate_vault_command(data, options)?;
            pipeline.push(vault_cmd);
        }
        
        if Self::requires_template_processing(data) {
            let template_cmd = Self::generate_template_command(data, options)?;
            pipeline.push(template_cmd);
        }
        
        // Add final destination
        if let Some(output_tool) = &options.output_tool {
            pipeline.push(output_tool.clone());
        }
        
        Ok(pipeline.join(" | "))
    }
    
    fn generate_vault_command(
        data: &RustleData,
        options: &PipelineOptions,
    ) -> Result<String, IntegrationError> {
        let mut cmd = vec!["rustle-vault", "resolve-markers"];
        
        if let Some(password_file) = &options.vault_password_file {
            cmd.push("--password-file");
            cmd.push(&format!("\"{}\"", password_file.display()));
        }
        
        if options.vault_ask_pass {
            cmd.push("--ask-vault-pass");
        }
        
        Ok(cmd.join(" "))
    }
    
    fn generate_template_command(
        data: &RustleData,
        options: &PipelineOptions,
    ) -> Result<String, IntegrationError> {
        let mut cmd = vec!["rustle-template", "resolve-markers"];
        
        if let Some(vars_file) = &options.extra_vars_file {
            cmd.push("--vars");
            cmd.push(&format!("\"{}\"", vars_file.display()));
        }
        
        if !options.extra_vars.is_empty() {
            cmd.push("--extra-vars");
            cmd.push(&format!("'{}'", serde_json::to_string(&options.extra_vars)?));
        }
        
        Ok(cmd.join(" "))
    }
}
```

## File and Package Structure

### Integration Module Structure
```
src/
├── integration/
│   ├── mod.rs                  # Integration module exports
│   ├── pipeline.rs             # Pipeline data flow
│   ├── markers.rs              # Marker generation and tracking
│   ├── complexity.rs           # Template/vault complexity analysis
│   ├── error.rs                # Integration error handling
│   ├── format.rs               # Data format conversion
│   └── generator.rs            # Pipeline command generation
├── parser/
│   ├── vault.rs                # Enhanced with marker support
│   ├── template.rs             # Enhanced with marker support
│   └── mod.rs                  # Export integration functionality
├── types/
│   ├── integration.rs          # Integration types and schemas
│   └── pipeline.rs             # Pipeline data structures
└── bin/
    └── rustle-parse.rs         # Enhanced CLI with integration flags

tests/
├── integration/
│   ├── pipeline_tests.rs       # Pipeline integration tests
│   ├── marker_tests.rs         # Marker generation tests
│   ├── error_handling_tests.rs # Error handling tests
│   └── command_generation_tests.rs # Command generation tests
└── fixtures/
    ├── pipeline/               # Pipeline test data
    └── integration/            # Integration test scenarios
```

## Testing Strategy

### Integration Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_vault_pipeline_integration() {
        let playbook_content = r#"
---
- hosts: all
  vars:
    secret: !vault |
      $ANSIBLE_VAULT;1.1;AES256
      66386439653762336464663732373034373366353462363763636163393464386565316333
      3438626638653333373066386464373236656363366235650a396130306533633562623533656165
      35653934373338373836313739626638643461336137323163366565333737623039353064653361
      6133386361616464330a3137313332623134343135306164626561616432383936346466353439
  tasks:
    - debug: msg="{{ secret }}"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), playbook_content).unwrap();
        
        // Test parsing with vault deferral
        let args = ParseArgs {
            playbook: temp_file.path().to_path_buf(),
            defer_vault: true,
            include_metadata: true,
            ..Default::default()
        };
        
        let result = parse_with_args(args).await.unwrap();
        
        // Verify vault markers were created
        assert!(!result.vault_markers.is_empty());
        assert_eq!(result.vault_markers[0].location, "plays[0].vars.secret");
        
        // Verify pipeline data structure
        assert_eq!(result.metadata.tool_chain, vec!["rustle-parse"]);
        assert!(PipelineIntegration::requires_vault_processing(&result));
    }
    
    #[tokio::test]
    async fn test_template_complexity_analysis() {
        let templates = vec![
            ("{{ simple_var }}", 1), // Simple
            ("{{ var | default('test') }}", 2), // Basic filter
            ("{{ items | select('defined') | list }}", 6), // Complex filter chain
            ("{% for item in items %}{{ item | upper }}{% endfor %}", 8), // Loop + filter
        ];
        
        for (template, expected_complexity) in templates {
            let complexity = ComplexityAnalyzer::analyze_template_complexity(template);
            assert_eq!(complexity.score, expected_complexity);
        }
    }
    
    #[tokio::test]
    async fn test_pipeline_command_generation() {
        let data = RustleData {
            vault_markers: vec![VaultMarker { /* test data */ }],
            template_markers: vec![TemplateMarker { /* test data */ }],
            ..Default::default()
        };
        
        let options = PipelineOptions {
            playbook_path: PathBuf::from("test.yml"),
            vault_password_file: Some(PathBuf::from("vault-pass")),
            extra_vars_file: Some(PathBuf::from("vars.yml")),
            ..Default::default()
        };
        
        let command = PipelineGenerator::generate_pipeline_command(&data, &options).unwrap();
        
        assert!(command.contains("rustle-parse"));
        assert!(command.contains("rustle-vault"));
        assert!(command.contains("rustle-template"));
        assert!(command.contains("--password-file"));
    }
    
    #[tokio::test]
    async fn test_error_fallback_strategies() {
        // Test vault error fallback
        let vault_error = VaultError::NoPassword { vault_id: "test".to_string() };
        let action = IntegrationErrorHandler::handle_vault_error(
            vault_error,
            &ErrorContext::default(),
        ).unwrap();
        
        assert!(matches!(action, FallbackAction::DeferToMarker));
        
        // Test template error fallback
        let template_error = TemplateError::new(ErrorKind::UndefinedError, "undefined var");
        let action = IntegrationErrorHandler::handle_template_error(
            template_error,
            "{{ undefined_var }}",
            &ErrorContext::default(),
        ).unwrap();
        
        assert!(matches!(action, FallbackAction::ProcessWithBasicEngine));
    }
}
```

## Configuration

### Integration Configuration
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationConfig {
    pub enable_pipeline_mode: bool,
    pub default_defer_vault: bool,
    pub default_defer_templates: bool,
    pub template_complexity_threshold: u8,
    pub tool_paths: ToolPaths,
    pub timeout_settings: TimeoutSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPaths {
    pub rustle_vault: Option<PathBuf>,
    pub rustle_template: Option<PathBuf>,
    pub rustle_plan: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeoutSettings {
    pub tool_execution: Duration,
    pub pipeline_communication: Duration,
    pub vault_operations: Duration,
    pub template_rendering: Duration,
}
```

## Dependencies

### Integration Dependencies
```toml
[dependencies]
# Pipeline communication
tokio = { version = "1.0", features = ["io-util", "time"] }
serde_json = "1.0"
serde_yaml = "0.9"

# JSONPath for marker locations
jsonpath-lib = "0.3"

# Command execution
tokio-process = "0.2"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Time handling for metadata
chrono = { version = "0.4", features = ["serde"] }
```

## Implementation Phases

### Phase 1: Basic Integration (Week 1)
- [ ] Implement marker generation during parsing
- [ ] Create standardized data exchange format
- [ ] Add CLI flags for integration modes
- [ ] Basic pipeline data flow

### Phase 2: Tool Communication (Week 2)
- [ ] Implement stdin/stdout pipeline communication
- [ ] Add complexity analysis for vault/template content
- [ ] Error handling and fallback strategies
- [ ] Pipeline command generation

### Phase 3: Advanced Features (Week 3)
- [ ] Streaming integration for large datasets
- [ ] Performance metrics collection
- [ ] Tool detection and validation
- [ ] Configuration file support

### Phase 4: Production Readiness (Week 4)
- [ ] Comprehensive testing with real scenarios
- [ ] Documentation and examples
- [ ] Performance optimization
- [ ] Error message improvements

## Success Metrics

### Integration Metrics
- Tools compose correctly in 100% of test scenarios
- Error handling prevents cascade failures
- Pipeline performance within 10% of individual tool performance
- Backward compatibility maintained

### Usability Metrics
- Simple workflows require no additional configuration
- Advanced users can access individual tools easily
- Error messages provide clear guidance
- Documentation covers all integration patterns