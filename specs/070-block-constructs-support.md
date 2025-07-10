# Spec 070: Block Constructs Support

## Feature Summary

Implement comprehensive support for Ansible's block constructs including `block`, `rescue`, and `always` sections for error handling, task grouping, and conditional execution. This enables structured error handling, task organization, and complex control flow that are essential for robust Ansible playbooks.

**Problem it solves**: The current parser only handles simple task lists without support for block constructs. Real Ansible playbooks use blocks for error handling, conditional task grouping, and applying common properties to multiple tasks, which are currently unsupported.

**High-level approach**: Extend the task parsing system to recognize and process block constructs with proper error handling semantics, task inheritance, and conditional evaluation. Implement nested block support and ensure proper variable scoping.

## Goals & Requirements

### Functional Requirements
- Support `block`, `rescue`, and `always` sections
- Handle nested blocks and complex block structures
- Implement proper error handling flow (block → rescue → always)
- Support block-level properties (when, tags, become, etc.)
- Apply block properties to all contained tasks
- Handle block-level variables and scoping
- Support conditional block execution
- Implement task tagging and filtering for blocks
- Handle block-level delegation and connection properties

### Non-functional Requirements
- **Performance**: Block processing should not significantly impact parsing speed
- **Memory**: Efficient memory usage for deeply nested blocks
- **Compatibility**: 100% compatible with Ansible block behavior
- **Error Handling**: Clear error messages for block-related issues
- **Maintainability**: Clean separation between block and task logic

### Success Criteria
- All block constructs parse and execute correctly
- Error handling flows work as expected
- Block properties properly inherit to contained tasks
- Nested blocks work correctly
- Real-world playbooks with complex blocks parse successfully

## API/Interface Design

### Block Structure Types
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBlock {
    pub id: String,
    pub name: Option<String>,
    pub block_tasks: Vec<TaskOrBlock>,
    pub rescue_tasks: Option<Vec<TaskOrBlock>>,
    pub always_tasks: Option<Vec<TaskOrBlock>>,
    pub vars: HashMap<String, serde_json::Value>,
    pub when: Option<String>,
    pub tags: Vec<String>,
    pub become: Option<bool>,
    pub become_user: Option<String>,
    pub become_method: Option<String>,
    pub delegate_to: Option<String>,
    pub delegate_facts: Option<bool>,
    pub run_once: Option<bool>,
    pub ignore_errors: Option<bool>,
    pub any_errors_fatal: Option<bool>,
    pub max_fail_percentage: Option<f32>,
    pub throttle: Option<u32>,
    pub timeout: Option<u32>,
    pub check_mode: Option<bool>,
    pub diff: Option<bool>,
    pub environment: Option<HashMap<String, String>>,
    pub no_log: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskOrBlock {
    Task(ParsedTask),
    Block(ParsedBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockExecutionContext {
    pub block_vars: HashMap<String, serde_json::Value>,
    pub inherited_properties: BlockProperties,
    pub error_state: BlockErrorState,
    pub execution_phase: BlockPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockProperties {
    pub when: Option<String>,
    pub tags: Vec<String>,
    pub become: Option<bool>,
    pub become_user: Option<String>,
    pub become_method: Option<String>,
    pub delegate_to: Option<String>,
    pub delegate_facts: Option<bool>,
    pub run_once: Option<bool>,
    pub environment: Option<HashMap<String, String>>,
    pub no_log: Option<bool>,
    pub check_mode: Option<bool>,
    pub diff: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockPhase {
    Block,
    Rescue,
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockErrorState {
    pub has_errors: bool,
    pub fatal_error: bool,
    pub ignored_errors: bool,
    pub error_details: Option<String>,
}
```

### Block Parser Interface
```rust
pub struct BlockParser<'a> {
    template_engine: &'a TemplateEngine,
    task_parser: &'a TaskParser<'a>,
}

impl<'a> BlockParser<'a> {
    pub fn new(template_engine: &'a TemplateEngine, task_parser: &'a TaskParser<'a>) -> Self;
    
    /// Parse a block construct from raw YAML
    pub async fn parse_block(
        &self,
        raw_block: RawBlock,
        context: &BlockParsingContext,
        index: usize,
    ) -> Result<ParsedBlock, ParseError>;
    
    /// Parse a list of tasks or blocks
    pub async fn parse_task_or_block_list(
        &self,
        raw_items: Vec<RawTaskOrBlock>,
        context: &BlockParsingContext,
    ) -> Result<Vec<TaskOrBlock>, ParseError>;
    
    /// Apply block properties to a task
    pub fn apply_block_properties(
        &self,
        task: &mut ParsedTask,
        block_properties: &BlockProperties,
    );
    
    /// Validate block structure and semantics
    pub fn validate_block(&self, block: &ParsedBlock) -> Result<(), ParseError>;
    
    /// Resolve block-level variables and merge with context
    pub fn resolve_block_variables(
        &self,
        block: &ParsedBlock,
        context: &BlockParsingContext,
    ) -> Result<HashMap<String, serde_json::Value>, ParseError>;
}

#[derive(Debug, Clone)]
pub struct BlockParsingContext {
    pub variables: HashMap<String, serde_json::Value>,
    pub parent_block_properties: Option<BlockProperties>,
    pub block_depth: usize,
    pub max_block_depth: usize,
}

impl BlockParsingContext {
    pub fn new(variables: HashMap<String, serde_json::Value>) -> Self;
    pub fn with_parent_properties(mut self, properties: BlockProperties) -> Self;
    pub fn push_block_scope(&mut self, block_vars: HashMap<String, serde_json::Value>);
    pub fn pop_block_scope(&mut self);
}
```

### Raw Block Deserialization
```rust
#[derive(Debug, Deserialize)]
pub struct RawBlock {
    pub name: Option<String>,
    pub block: Vec<RawTaskOrBlock>,
    pub rescue: Option<Vec<RawTaskOrBlock>>,
    pub always: Option<Vec<RawTaskOrBlock>>,
    
    // Block-level properties
    pub vars: Option<HashMap<String, serde_json::Value>>,
    pub when: Option<String>,
    pub tags: Option<Vec<String>>,
    pub become: Option<bool>,
    pub become_user: Option<String>,
    pub become_method: Option<String>,
    pub delegate_to: Option<String>,
    pub delegate_facts: Option<bool>,
    pub run_once: Option<bool>,
    pub ignore_errors: Option<bool>,
    pub any_errors_fatal: Option<bool>,
    pub max_fail_percentage: Option<f32>,
    pub throttle: Option<u32>,
    pub timeout: Option<u32>,
    pub check_mode: Option<bool>,
    pub diff: Option<bool>,
    pub environment: Option<HashMap<String, String>>,
    pub no_log: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawTaskOrBlock {
    Task(RawTask),
    Block(RawBlock),
}
```

### Integration with Existing Parser
```rust
impl<'a> PlaybookParser<'a> {
    async fn parse_task_or_block(
        &self,
        raw_item: RawTaskOrBlock,
        context: &BlockParsingContext,
        index: usize,
    ) -> Result<TaskOrBlock, ParseError> {
        match raw_item {
            RawTaskOrBlock::Task(raw_task) => {
                let task = self.parse_task(raw_task, &context.variables, index).await?;
                
                // Apply parent block properties if any
                let mut enhanced_task = task;
                if let Some(parent_props) = &context.parent_block_properties {
                    self.block_parser.apply_block_properties(&mut enhanced_task, parent_props);
                }
                
                Ok(TaskOrBlock::Task(enhanced_task))
            }
            RawTaskOrBlock::Block(raw_block) => {
                let block = self.block_parser.parse_block(raw_block, context, index).await?;
                Ok(TaskOrBlock::Block(block))
            }
        }
    }
}
```

## File and Package Structure

### Block Processing Module Structure
```
src/
├── parser/
│   ├── block/
│   │   ├── mod.rs                 # Block module exports
│   │   ├── parser.rs              # Main block parsing logic
│   │   ├── properties.rs          # Block property inheritance
│   │   ├── execution.rs           # Block execution semantics
│   │   ├── validation.rs          # Block structure validation
│   │   └── error_handling.rs      # Block error handling logic
│   ├── playbook.rs                # Enhanced with block support
│   ├── task.rs                    # Enhanced task parsing
│   └── error.rs                   # Add block-related errors
├── types/
│   ├── block.rs                   # Block-related types
│   └── parsed.rs                  # Enhanced with block support
└── ...

tests/
├── fixtures/
│   ├── blocks/
│   │   ├── basic/                 # Basic block examples
│   │   │   ├── simple_block.yml
│   │   │   ├── rescue_block.yml
│   │   │   └── always_block.yml
│   │   ├── nested/                # Nested block scenarios
│   │   ├── properties/            # Block property inheritance
│   │   ├── error_handling/        # Error handling scenarios
│   │   └── edge_cases/            # Complex and edge cases
│   └── expected/
│       └── blocks/                # Expected parsing results
└── parser/
    ├── block_parser_tests.rs      # Core block parser tests
    ├── block_properties_tests.rs  # Property inheritance tests
    ├── block_execution_tests.rs   # Execution semantics tests
    └── block_integration_tests.rs # Integration tests
```

## Implementation Details

### Phase 1: Basic Block Parsing
```rust
// src/parser/block/parser.rs
impl<'a> BlockParser<'a> {
    pub async fn parse_block(
        &self,
        raw_block: RawBlock,
        context: &BlockParsingContext,
        index: usize,
    ) -> Result<ParsedBlock, ParseError> {
        // Validate block depth
        if context.block_depth >= context.max_block_depth {
            return Err(ParseError::MaxBlockDepthExceeded {
                depth: context.max_block_depth,
                block_name: raw_block.name.clone(),
            });
        }
        
        // Generate block ID
        let id = raw_block.name.clone()
            .unwrap_or_else(|| format!("block_{}", index));
        
        // Create block properties from raw block
        let block_properties = self.extract_block_properties(&raw_block);
        
        // Resolve block variables
        let block_vars = self.resolve_block_variables_from_raw(&raw_block, context)?;
        
        // Create new context for nested tasks/blocks
        let mut nested_context = context.clone();
        nested_context.block_depth += 1;
        nested_context.push_block_scope(block_vars.clone());
        nested_context = nested_context.with_parent_properties(block_properties.clone());
        
        // Parse block tasks
        let block_tasks = self.parse_task_or_block_list(raw_block.block, &nested_context).await?;
        
        // Parse rescue tasks if present
        let rescue_tasks = if let Some(rescue_items) = raw_block.rescue {
            Some(self.parse_task_or_block_list(rescue_items, &nested_context).await?)
        } else {
            None
        };
        
        // Parse always tasks if present
        let always_tasks = if let Some(always_items) = raw_block.always {
            Some(self.parse_task_or_block_list(always_items, &nested_context).await?)
        } else {
            None
        };
        
        let parsed_block = ParsedBlock {
            id,
            name: raw_block.name,
            block_tasks,
            rescue_tasks,
            always_tasks,
            vars: block_vars,
            when: raw_block.when,
            tags: raw_block.tags.unwrap_or_default(),
            become: raw_block.become,
            become_user: raw_block.become_user,
            become_method: raw_block.become_method,
            delegate_to: raw_block.delegate_to,
            delegate_facts: raw_block.delegate_facts,
            run_once: raw_block.run_once,
            ignore_errors: raw_block.ignore_errors,
            any_errors_fatal: raw_block.any_errors_fatal,
            max_fail_percentage: raw_block.max_fail_percentage,
            throttle: raw_block.throttle,
            timeout: raw_block.timeout,
            check_mode: raw_block.check_mode,
            diff: raw_block.diff,
            environment: raw_block.environment,
            no_log: raw_block.no_log,
        };
        
        // Validate the parsed block
        self.validate_block(&parsed_block)?;
        
        Ok(parsed_block)
    }
    
    fn extract_block_properties(&self, raw_block: &RawBlock) -> BlockProperties {
        BlockProperties {
            when: raw_block.when.clone(),
            tags: raw_block.tags.clone().unwrap_or_default(),
            become: raw_block.become,
            become_user: raw_block.become_user.clone(),
            become_method: raw_block.become_method.clone(),
            delegate_to: raw_block.delegate_to.clone(),
            delegate_facts: raw_block.delegate_facts,
            run_once: raw_block.run_once,
            environment: raw_block.environment.clone(),
            no_log: raw_block.no_log,
            check_mode: raw_block.check_mode,
            diff: raw_block.diff,
        }
    }
}
```

### Phase 2: Property Inheritance
```rust
// src/parser/block/properties.rs
impl<'a> BlockParser<'a> {
    pub fn apply_block_properties(
        &self,
        task: &mut ParsedTask,
        block_properties: &BlockProperties,
    ) {
        // Merge when conditions (block AND task)
        if let Some(block_when) = &block_properties.when {
            if let Some(task_when) = &task.when {
                task.when = Some(format!("({}) and ({})", block_when, task_when));
            } else {
                task.when = Some(block_when.clone());
            }
        }
        
        // Merge tags (block tags + task tags)
        task.tags.extend(block_properties.tags.clone());
        
        // Apply privilege escalation (task overrides block)
        if task.become.is_none() {
            task.become = block_properties.become;
        }
        if task.become_user.is_none() {
            task.become_user = block_properties.become_user.clone();
        }
        if task.become_method.is_none() {
            task.become_method = block_properties.become_method.clone();
        }
        
        // Apply delegation (task overrides block)
        if task.delegate_to.is_none() {
            task.delegate_to = block_properties.delegate_to.clone();
        }
        if task.delegate_facts.is_none() {
            task.delegate_facts = block_properties.delegate_facts;
        }
        
        // Apply run_once (task overrides block)
        if task.run_once.is_none() {
            task.run_once = block_properties.run_once;
        }
        
        // Merge environment variables (block + task, task overrides block)
        if let Some(block_env) = &block_properties.environment {
            let mut merged_env = block_env.clone();
            if let Some(task_env) = &task.environment {
                merged_env.extend(task_env.clone());
            }
            task.environment = Some(merged_env);
        }
        
        // Apply logging settings (task overrides block)
        if task.no_log.is_none() {
            task.no_log = block_properties.no_log;
        }
        
        // Apply check mode settings (task overrides block)
        if task.check_mode.is_none() {
            task.check_mode = block_properties.check_mode;
        }
        if task.diff.is_none() {
            task.diff = block_properties.diff;
        }
    }
    
    pub fn merge_block_properties(
        &self,
        parent: &BlockProperties,
        child: &BlockProperties,
    ) -> BlockProperties {
        BlockProperties {
            // When conditions are ANDed
            when: match (&parent.when, &child.when) {
                (Some(p), Some(c)) => Some(format!("({}) and ({})", p, c)),
                (Some(p), None) => Some(p.clone()),
                (None, Some(c)) => Some(c.clone()),
                (None, None) => None,
            },
            
            // Tags are merged
            tags: {
                let mut merged = parent.tags.clone();
                merged.extend(child.tags.clone());
                merged
            },
            
            // Child properties override parent properties
            become: child.become.or(parent.become),
            become_user: child.become_user.clone().or_else(|| parent.become_user.clone()),
            become_method: child.become_method.clone().or_else(|| parent.become_method.clone()),
            delegate_to: child.delegate_to.clone().or_else(|| parent.delegate_to.clone()),
            delegate_facts: child.delegate_facts.or(parent.delegate_facts),
            run_once: child.run_once.or(parent.run_once),
            no_log: child.no_log.or(parent.no_log),
            check_mode: child.check_mode.or(parent.check_mode),
            diff: child.diff.or(parent.diff),
            
            // Environment variables are merged (child overrides parent)
            environment: match (&parent.environment, &child.environment) {
                (Some(p), Some(c)) => {
                    let mut merged = p.clone();
                    merged.extend(c.clone());
                    Some(merged)
                }
                (Some(p), None) => Some(p.clone()),
                (None, Some(c)) => Some(c.clone()),
                (None, None) => None,
            },
        }
    }
}
```

### Phase 3: Block Execution Semantics
```rust
// src/parser/block/execution.rs
impl<'a> BlockParser<'a> {
    /// Simulate block execution logic for validation and dependency analysis
    pub fn analyze_block_execution(
        &self,
        block: &ParsedBlock,
        context: &BlockExecutionContext,
    ) -> Result<BlockExecutionResult, ParseError> {
        let mut result = BlockExecutionResult::new();
        
        // Phase 1: Execute block tasks
        result.block_phase = BlockPhase::Block;
        for task_or_block in &block.block_tasks {
            match task_or_block {
                TaskOrBlock::Task(task) => {
                    let task_result = self.analyze_task_execution(task, context)?;
                    result.task_results.push(task_result);
                    
                    // Check for task failures
                    if task_result.would_fail && !task.ignore_errors {
                        result.block_failed = true;
                        if block.any_errors_fatal.unwrap_or(false) {
                            result.fatal_error = true;
                        }
                        break; // Exit block on failure
                    }
                }
                TaskOrBlock::Block(nested_block) => {
                    let nested_context = self.create_nested_execution_context(context, nested_block)?;
                    let nested_result = self.analyze_block_execution(nested_block, &nested_context)?;
                    result.nested_results.push(nested_result.clone());
                    
                    if nested_result.block_failed {
                        result.block_failed = true;
                        if nested_result.fatal_error {
                            result.fatal_error = true;
                            break;
                        }
                    }
                }
            }
        }
        
        // Phase 2: Execute rescue tasks if block failed
        if result.block_failed && !result.fatal_error {
            if let Some(rescue_tasks) = &block.rescue_tasks {
                result.block_phase = BlockPhase::Rescue;
                result.rescue_executed = true;
                
                for task_or_block in rescue_tasks {
                    match task_or_block {
                        TaskOrBlock::Task(task) => {
                            let task_result = self.analyze_task_execution(task, context)?;
                            result.rescue_results.push(task_result);
                        }
                        TaskOrBlock::Block(nested_block) => {
                            let nested_context = self.create_nested_execution_context(context, nested_block)?;
                            let nested_result = self.analyze_block_execution(nested_block, &nested_context)?;
                            result.rescue_nested_results.push(nested_result);
                        }
                    }
                }
            }
        }
        
        // Phase 3: Always execute always tasks
        if let Some(always_tasks) = &block.always_tasks {
            result.block_phase = BlockPhase::Always;
            result.always_executed = true;
            
            for task_or_block in always_tasks {
                match task_or_block {
                    TaskOrBlock::Task(task) => {
                        let task_result = self.analyze_task_execution(task, context)?;
                        result.always_results.push(task_result);
                    }
                    TaskOrBlock::Block(nested_block) => {
                        let nested_context = self.create_nested_execution_context(context, nested_block)?;
                        let nested_result = self.analyze_block_execution(nested_block, &nested_context)?;
                        result.always_nested_results.push(nested_result);
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    fn analyze_task_execution(
        &self,
        task: &ParsedTask,
        context: &BlockExecutionContext,
    ) -> Result<TaskExecutionResult, ParseError> {
        // Analyze task execution without actually running it
        let mut result = TaskExecutionResult {
            task_name: task.name.clone(),
            would_execute: true,
            would_fail: false,
            skip_reason: None,
        };
        
        // Check if task would be skipped due to when condition
        if let Some(when_condition) = &task.when {
            let would_run = self.evaluate_when_condition(when_condition, &context.block_vars)?;
            if !would_run {
                result.would_execute = false;
                result.skip_reason = Some("when condition false".to_string());
            }
        }
        
        // Analyze potential failure scenarios based on module
        result.would_fail = self.analyze_task_failure_potential(task)?;
        
        Ok(result)
    }
    
    fn evaluate_when_condition(
        &self,
        condition: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<bool, ParseError> {
        // Use template engine to evaluate the condition
        let template_str = format!("{{{{ {} }}}}", condition);
        let result = self.template_engine.render_string(&template_str, variables)?;
        
        // Parse as boolean
        match result.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(true),
            "false" | "no" | "0" => Ok(false),
            "" => Ok(false),
            _ => {
                // Try to parse as truthy/falsy
                Ok(!result.trim().is_empty())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockExecutionResult {
    pub block_phase: BlockPhase,
    pub block_failed: bool,
    pub fatal_error: bool,
    pub rescue_executed: bool,
    pub always_executed: bool,
    pub task_results: Vec<TaskExecutionResult>,
    pub rescue_results: Vec<TaskExecutionResult>,
    pub always_results: Vec<TaskExecutionResult>,
    pub nested_results: Vec<BlockExecutionResult>,
    pub rescue_nested_results: Vec<BlockExecutionResult>,
    pub always_nested_results: Vec<BlockExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct TaskExecutionResult {
    pub task_name: String,
    pub would_execute: bool,
    pub would_fail: bool,
    pub skip_reason: Option<String>,
}
```

### Phase 4: Block Validation
```rust
// src/parser/block/validation.rs
impl<'a> BlockParser<'a> {
    pub fn validate_block(&self, block: &ParsedBlock) -> Result<(), ParseError> {
        // Validate block has at least one section
        if block.block_tasks.is_empty() && block.rescue_tasks.is_none() && block.always_tasks.is_none() {
            return Err(ParseError::InvalidBlockStructure {
                message: "Block must have at least one of 'block', 'rescue', or 'always' sections".to_string(),
                block_name: block.name.clone(),
            });
        }
        
        // Validate rescue/always sections only exist with block section
        if !block.block_tasks.is_empty() {
            // Valid - has block section
        } else if block.rescue_tasks.is_some() || block.always_tasks.is_some() {
            return Err(ParseError::InvalidBlockStructure {
                message: "rescue and always sections require a block section".to_string(),
                block_name: block.name.clone(),
            });
        }
        
        // Validate nested structures
        self.validate_task_or_block_list(&block.block_tasks)?;
        
        if let Some(rescue_tasks) = &block.rescue_tasks {
            self.validate_task_or_block_list(rescue_tasks)?;
        }
        
        if let Some(always_tasks) = &block.always_tasks {
            self.validate_task_or_block_list(always_tasks)?;
        }
        
        // Validate block properties
        self.validate_block_properties(block)?;
        
        Ok(())
    }
    
    fn validate_task_or_block_list(&self, items: &[TaskOrBlock]) -> Result<(), ParseError> {
        for item in items {
            match item {
                TaskOrBlock::Task(task) => {
                    self.validate_task(task)?;
                }
                TaskOrBlock::Block(block) => {
                    self.validate_block(block)?;
                }
            }
        }
        Ok(())
    }
    
    fn validate_block_properties(&self, block: &ParsedBlock) -> Result<(), ParseError> {
        // Validate when condition syntax
        if let Some(when_condition) = &block.when {
            self.validate_when_condition(when_condition)?;
        }
        
        // Validate tag names
        for tag in &block.tags {
            if tag.is_empty() {
                return Err(ParseError::InvalidBlockStructure {
                    message: "Block tags cannot be empty".to_string(),
                    block_name: block.name.clone(),
                });
            }
        }
        
        // Validate privilege escalation settings
        if block.become == Some(true) && block.become_user.is_none() {
            // Warning: become without become_user (uses default)
        }
        
        // Validate percentage values
        if let Some(max_fail_pct) = block.max_fail_percentage {
            if !(0.0..=100.0).contains(&max_fail_pct) {
                return Err(ParseError::InvalidBlockStructure {
                    message: format!("max_fail_percentage must be between 0 and 100, got {}", max_fail_pct),
                    block_name: block.name.clone(),
                });
            }
        }
        
        // Validate timeout value
        if let Some(timeout) = block.timeout {
            if timeout == 0 {
                return Err(ParseError::InvalidBlockStructure {
                    message: "timeout must be greater than 0".to_string(),
                    block_name: block.name.clone(),
                });
            }
        }
        
        Ok(())
    }
    
    fn validate_when_condition(&self, condition: &str) -> Result<(), ParseError> {
        // Basic validation - check for common syntax errors
        if condition.trim().is_empty() {
            return Err(ParseError::InvalidWhenCondition {
                condition: condition.to_string(),
                message: "when condition cannot be empty".to_string(),
            });
        }
        
        // Check for unbalanced parentheses
        let open_parens = condition.matches('(').count();
        let close_parens = condition.matches(')').count();
        if open_parens != close_parens {
            return Err(ParseError::InvalidWhenCondition {
                condition: condition.to_string(),
                message: "unbalanced parentheses in when condition".to_string(),
            });
        }
        
        // Check for unbalanced quotes
        let single_quotes = condition.matches('\'').count();
        let double_quotes = condition.matches('"').count();
        if single_quotes % 2 != 0 || double_quotes % 2 != 0 {
            return Err(ParseError::InvalidWhenCondition {
                condition: condition.to_string(),
                message: "unbalanced quotes in when condition".to_string(),
            });
        }
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Testing Requirements
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_block_parsing() {
        let raw_block = RawBlock {
            name: Some("Test block".to_string()),
            block: vec![
                RawTaskOrBlock::Task(RawTask {
                    name: Some("Task 1".to_string()),
                    module_args: {
                        let mut args = HashMap::new();
                        args.insert("debug".to_string(), serde_json::json!({"msg": "hello"}));
                        args
                    },
                    ..Default::default()
                }),
                RawTaskOrBlock::Task(RawTask {
                    name: Some("Task 2".to_string()),
                    module_args: {
                        let mut args = HashMap::new();
                        args.insert("command".to_string(), serde_json::json!("echo test"));
                        args
                    },
                    ..Default::default()
                }),
            ],
            rescue: None,
            always: None,
            vars: None,
            when: None,
            tags: None,
            become: None,
            become_user: None,
            become_method: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: None,
            ignore_errors: None,
            any_errors_fatal: None,
            max_fail_percentage: None,
            throttle: None,
            timeout: None,
            check_mode: None,
            diff: None,
            environment: None,
            no_log: None,
        };
        
        let template_engine = TemplateEngine::new();
        let task_parser = TaskParser::new(&template_engine, &HashMap::new());
        let block_parser = BlockParser::new(&template_engine, &task_parser);
        
        let context = BlockParsingContext::new(HashMap::new());
        
        let parsed_block = block_parser.parse_block(raw_block, &context, 0).await.unwrap();
        
        assert_eq!(parsed_block.name, Some("Test block".to_string()));
        assert_eq!(parsed_block.block_tasks.len(), 2);
        assert!(parsed_block.rescue_tasks.is_none());
        assert!(parsed_block.always_tasks.is_none());
    }
    
    #[tokio::test]
    async fn test_block_with_rescue_and_always() {
        let raw_block = RawBlock {
            name: Some("Error handling block".to_string()),
            block: vec![
                RawTaskOrBlock::Task(create_failing_task()),
            ],
            rescue: Some(vec![
                RawTaskOrBlock::Task(create_recovery_task()),
            ]),
            always: Some(vec![
                RawTaskOrBlock::Task(create_cleanup_task()),
            ]),
            ..Default::default()
        };
        
        let template_engine = TemplateEngine::new();
        let task_parser = TaskParser::new(&template_engine, &HashMap::new());
        let block_parser = BlockParser::new(&template_engine, &task_parser);
        
        let context = BlockParsingContext::new(HashMap::new());
        
        let parsed_block = block_parser.parse_block(raw_block, &context, 0).await.unwrap();
        
        assert_eq!(parsed_block.block_tasks.len(), 1);
        assert_eq!(parsed_block.rescue_tasks.as_ref().unwrap().len(), 1);
        assert_eq!(parsed_block.always_tasks.as_ref().unwrap().len(), 1);
    }
    
    #[test]
    fn test_block_property_inheritance() {
        let template_engine = TemplateEngine::new();
        let task_parser = TaskParser::new(&template_engine, &HashMap::new());
        let block_parser = BlockParser::new(&template_engine, &task_parser);
        
        let block_properties = BlockProperties {
            when: Some("condition1".to_string()),
            tags: vec!["block_tag".to_string()],
            become: Some(true),
            become_user: Some("root".to_string()),
            become_method: None,
            delegate_to: None,
            delegate_facts: None,
            run_once: Some(true),
            environment: Some({
                let mut env = HashMap::new();
                env.insert("BLOCK_VAR".to_string(), "block_value".to_string());
                env
            }),
            no_log: Some(false),
            check_mode: None,
            diff: None,
        };
        
        let mut task = ParsedTask {
            name: "Test task".to_string(),
            when: Some("condition2".to_string()),
            tags: vec!["task_tag".to_string()],
            become: None,
            become_user: None,
            become_method: None,
            environment: Some({
                let mut env = HashMap::new();
                env.insert("TASK_VAR".to_string(), "task_value".to_string());
                env
            }),
            ..Default::default()
        };
        
        block_parser.apply_block_properties(&mut task, &block_properties);
        
        // Check when condition is combined
        assert_eq!(task.when, Some("(condition1) and (condition2)".to_string()));
        
        // Check tags are merged
        assert!(task.tags.contains(&"block_tag".to_string()));
        assert!(task.tags.contains(&"task_tag".to_string()));
        
        // Check become properties are inherited
        assert_eq!(task.become, Some(true));
        assert_eq!(task.become_user, Some("root".to_string()));
        
        // Check environment variables are merged
        let env = task.environment.unwrap();
        assert_eq!(env.get("BLOCK_VAR"), Some(&"block_value".to_string()));
        assert_eq!(env.get("TASK_VAR"), Some(&"task_value".to_string()));
    }
    
    #[test]
    fn test_nested_block_properties() {
        let template_engine = TemplateEngine::new();
        let task_parser = TaskParser::new(&template_engine, &HashMap::new());
        let block_parser = BlockParser::new(&template_engine, &task_parser);
        
        let parent_properties = BlockProperties {
            when: Some("parent_condition".to_string()),
            tags: vec!["parent_tag".to_string()],
            become: Some(true),
            ..Default::default()
        };
        
        let child_properties = BlockProperties {
            when: Some("child_condition".to_string()),
            tags: vec!["child_tag".to_string()],
            become_user: Some("child_user".to_string()),
            ..Default::default()
        };
        
        let merged = block_parser.merge_block_properties(&parent_properties, &child_properties);
        
        // When conditions should be ANDed
        assert_eq!(merged.when, Some("(parent_condition) and (child_condition)".to_string()));
        
        // Tags should be merged
        assert!(merged.tags.contains(&"parent_tag".to_string()));
        assert!(merged.tags.contains(&"child_tag".to_string()));
        
        // Child properties should override parent
        assert_eq!(merged.become, Some(true)); // from parent
        assert_eq!(merged.become_user, Some("child_user".to_string())); // from child
    }
}
```

### Integration Testing
```rust
// tests/parser/block_integration_tests.rs
#[tokio::test]
async fn test_complex_nested_blocks() {
    let playbook_content = r#"
---
- hosts: all
  tasks:
    - name: Main block
      block:
        - name: Nested block 1
          block:
            - name: Deep task 1
              debug:
                msg: "Deep task 1"
          rescue:
            - name: Deep rescue 1
              debug:
                msg: "Deep rescue 1"
          always:
            - name: Deep always 1
              debug:
                msg: "Deep always 1"
        
        - name: Nested block 2
          block:
            - name: Deep task 2
              debug:
                msg: "Deep task 2"
              when: false  # This will be skipped
      rescue:
        - name: Main rescue
          debug:
            msg: "Main rescue"
      always:
        - name: Main always
          debug:
            msg: "Main always"
      tags:
        - main
      become: yes
"#;
    
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), playbook_content).unwrap();
    
    let parser = PlaybookParser::new(&TemplateEngine::new(), &HashMap::new());
    let playbook = parser.parse(temp_file.path()).await.unwrap();
    
    assert_eq!(playbook.plays.len(), 1);
    let play = &playbook.plays[0];
    assert_eq!(play.tasks.len(), 1);
    
    if let TaskOrBlock::Block(main_block) = &play.tasks[0] {
        assert_eq!(main_block.name, Some("Main block".to_string()));
        assert_eq!(main_block.block_tasks.len(), 2);
        assert!(main_block.rescue_tasks.is_some());
        assert!(main_block.always_tasks.is_some());
        assert!(main_block.tags.contains(&"main".to_string()));
        assert_eq!(main_block.become, Some(true));
        
        // Check nested blocks
        if let TaskOrBlock::Block(nested_block) = &main_block.block_tasks[0] {
            assert_eq!(nested_block.name, Some("Nested block 1".to_string()));
            assert_eq!(nested_block.block_tasks.len(), 1);
            assert!(nested_block.rescue_tasks.is_some());
            assert!(nested_block.always_tasks.is_some());
        } else {
            panic!("Expected nested block");
        }
    } else {
        panic!("Expected main block");
    }
}

#[tokio::test]
async fn test_block_property_inheritance_integration() {
    let playbook_content = r#"
---
- hosts: all
  tasks:
    - name: Parent block
      block:
        - name: Child task
          debug:
            msg: "Child task"
          tags:
            - child
        - name: Child block
          block:
            - name: Grandchild task
              debug:
                msg: "Grandchild task"
          tags:
            - grandchild
      tags:
        - parent
      become: yes
      become_user: admin
      when: parent_condition
"#;
    
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), playbook_content).unwrap();
    
    let mut variables = HashMap::new();
    variables.insert("parent_condition".to_string(), serde_json::Value::Bool(true));
    
    let parser = PlaybookParser::new(&TemplateEngine::new(), &variables);
    let playbook = parser.parse(temp_file.path()).await.unwrap();
    
    let play = &playbook.plays[0];
    if let TaskOrBlock::Block(parent_block) = &play.tasks[0] {
        // Verify parent block properties
        assert!(parent_block.tags.contains(&"parent".to_string()));
        assert_eq!(parent_block.become, Some(true));
        assert_eq!(parent_block.become_user, Some("admin".to_string()));
        assert_eq!(parent_block.when, Some("parent_condition".to_string()));
        
        // Check that properties are inherited by child tasks
        if let TaskOrBlock::Task(child_task) = &parent_block.block_tasks[0] {
            assert!(child_task.tags.contains(&"parent".to_string())); // inherited
            assert!(child_task.tags.contains(&"child".to_string()));  // own
            assert_eq!(child_task.become, Some(true));               // inherited
            assert_eq!(child_task.become_user, Some("admin".to_string())); // inherited
            assert_eq!(child_task.when, Some("parent_condition".to_string())); // inherited
        }
        
        // Check nested block inheritance
        if let TaskOrBlock::Block(child_block) = &parent_block.block_tasks[1] {
            assert!(child_block.tags.contains(&"grandchild".to_string()));
            
            if let TaskOrBlock::Task(grandchild_task) = &child_block.block_tasks[0] {
                // Should inherit from both parent and child blocks
                assert!(grandchild_task.tags.contains(&"parent".to_string()));
                assert!(grandchild_task.tags.contains(&"grandchild".to_string()));
                assert_eq!(grandchild_task.become, Some(true));
                assert_eq!(grandchild_task.become_user, Some("admin".to_string()));
            }
        }
    }
}
```

## Edge Cases & Error Handling

### Block-Specific Error Types
```rust
#[derive(Debug, Error)]
pub enum ParseError {
    // ... existing errors ...
    
    #[error("Invalid block structure: {message}")]
    InvalidBlockStructure { 
        message: String,
        block_name: Option<String>,
    },
    
    #[error("Maximum block depth exceeded: {depth} levels deep")]
    MaxBlockDepthExceeded { 
        depth: usize,
        block_name: Option<String>,
    },
    
    #[error("Invalid when condition '{condition}': {message}")]
    InvalidWhenCondition { 
        condition: String,
        message: String,
    },
    
    #[error("Block property conflict: {message}")]
    BlockPropertyConflict { message: String },
    
    #[error("Circular block reference detected: {cycle}")]
    CircularBlockReference { cycle: String },
}
```

### Edge Case Handling
```rust
impl<'a> BlockParser<'a> {
    /// Handle edge case: empty block sections
    fn handle_empty_sections(&self, raw_block: &RawBlock) -> Result<(), ParseError> {
        if raw_block.block.is_empty() && 
           raw_block.rescue.as_ref().map_or(true, |r| r.is_empty()) &&
           raw_block.always.as_ref().map_or(true, |a| a.is_empty()) {
            return Err(ParseError::InvalidBlockStructure {
                message: "Block cannot have all empty sections".to_string(),
                block_name: raw_block.name.clone(),
            });
        }
        Ok(())
    }
    
    /// Handle edge case: conflicting properties
    fn validate_property_conflicts(&self, block: &ParsedBlock) -> Result<(), ParseError> {
        // Check for conflicting become settings
        if block.become == Some(false) && block.become_user.is_some() {
            return Err(ParseError::BlockPropertyConflict {
                message: "Cannot specify become_user when become is false".to_string(),
            });
        }
        
        // Check for conflicting delegation settings
        if block.delegate_to.is_some() && block.run_once == Some(true) {
            // This is a warning in Ansible, but we'll allow it
        }
        
        Ok(())
    }
    
    /// Handle deep nesting limits
    fn check_nesting_limits(&self, context: &BlockParsingContext) -> Result<(), ParseError> {
        if context.block_depth >= context.max_block_depth {
            return Err(ParseError::MaxBlockDepthExceeded {
                depth: context.max_block_depth,
                block_name: None,
            });
        }
        Ok(())
    }
}
```

## Dependencies

### No New Dependencies Required
Block constructs can be implemented using existing dependencies:
- `serde` and `serde_yaml` for block deserialization
- Existing template engine for property processing
- Current error handling infrastructure

### Enhanced Error Types
The existing `ParseError` enum will be extended with block-specific variants.

## Performance Considerations

### Memory Management
```rust
impl<'a> BlockParser<'a> {
    /// Optimize memory usage for deeply nested blocks
    fn optimize_block_memory(&self, block: &mut ParsedBlock) {
        // Use Box for large nested structures
        if self.should_box_nested_content(block) {
            // Convert to boxed storage for deeply nested blocks
            // Implementation would depend on memory pressure
        }
        
        // Deduplicate repeated property values
        self.deduplicate_properties(block);
    }
    
    fn should_box_nested_content(&self, block: &ParsedBlock) -> bool {
        let total_tasks = self.count_total_tasks(block);
        total_tasks > 100 // Threshold for boxing
    }
    
    fn count_total_tasks(&self, block: &ParsedBlock) -> usize {
        let mut count = 0;
        
        for item in &block.block_tasks {
            count += match item {
                TaskOrBlock::Task(_) => 1,
                TaskOrBlock::Block(nested) => self.count_total_tasks(nested),
            };
        }
        
        if let Some(rescue) = &block.rescue_tasks {
            for item in rescue {
                count += match item {
                    TaskOrBlock::Task(_) => 1,
                    TaskOrBlock::Block(nested) => self.count_total_tasks(nested),
                };
            }
        }
        
        if let Some(always) = &block.always_tasks {
            for item in always {
                count += match item {
                    TaskOrBlock::Task(_) => 1,
                    TaskOrBlock::Block(nested) => self.count_total_tasks(nested),
                };
            }
        }
        
        count
    }
}
```

### Parse Performance
- Cache block property inheritance calculations
- Use efficient data structures for tag merging
- Minimize string allocations in property merging

## Configuration

### Block Processing Configuration
```rust
#[derive(Debug, Clone)]
pub struct BlockConfig {
    pub max_block_depth: usize,
    pub enable_property_inheritance: bool,
    pub strict_validation: bool,
    pub optimize_memory: bool,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            max_block_depth: 50,
            enable_property_inheritance: true,
            strict_validation: true,
            optimize_memory: true,
        }
    }
}
```

## Implementation Phases

### Phase 1: Basic Block Support (Week 1)
- [ ] Implement basic block, rescue, always parsing
- [ ] Add block structure validation
- [ ] Basic property inheritance for simple cases
- [ ] Unit tests for core functionality

### Phase 2: Property Inheritance (Week 2)
- [ ] Complete property inheritance system
- [ ] Handle nested block property merging
- [ ] Add comprehensive property conflict detection
- [ ] Property inheritance integration tests

### Phase 3: Advanced Features (Week 3)
- [ ] Block execution analysis and validation
- [ ] Complex when condition handling
- [ ] Memory optimization for deep nesting
- [ ] Performance optimization

### Phase 4: Integration and Testing (Week 4)
- [ ] Full integration with playbook parser
- [ ] Comprehensive edge case testing
- [ ] Real-world block scenario testing
- [ ] Documentation and examples

## Success Metrics

### Functional Metrics
- All block constructs parse correctly
- Property inheritance works exactly like Ansible
- Error handling flows function properly
- Nested blocks work to reasonable depths

### Performance Metrics
- Block parsing adds <20% overhead to playbook parsing
- Memory usage scales linearly with block complexity
- Property inheritance processing <1ms per block
- Deep nesting (20+ levels) handled gracefully

### Compatibility Metrics
- 100% compatibility with Ansible block syntax
- All Ansible block test cases pass
- Real-world playbooks with complex blocks work
- Error behavior matches Ansible exactly