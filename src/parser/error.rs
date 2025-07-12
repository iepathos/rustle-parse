use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("YAML syntax error at line {line}, column {column}: {message}")]
    YamlSyntax {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Template error in {file} at line {line}: {message}")]
    Template {
        file: String,
        line: usize,
        message: String,
    },

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

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("INI parsing error: {message}")]
    IniParsing { message: String },

    #[error("Template rendering error: {0}")]
    TemplateRender(#[from] minijinja::Error),

    #[error("Invalid playbook structure: {message}")]
    InvalidStructure { message: String },

    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },

    #[error("Invalid host pattern '{pattern}' at line {line}: {message}")]
    InvalidHostPattern {
        pattern: String,
        line: usize,
        message: String,
    },

    #[error("Circular group dependency: {cycle}")]
    CircularGroupDependency { cycle: String },

    #[error("Invalid variable syntax at line {line}: {message}")]
    InvalidVariableSyntax { line: usize, message: String },

    #[error("Duplicate host '{host}' in inventory")]
    DuplicateHost { host: String },

    #[error("Unknown group '{group}' referenced in children")]
    UnknownGroup { group: String },

    // Include/Import related errors
    #[error("Circular dependency detected in includes: {cycle}")]
    CircularIncludeDependency { cycle: String },

    #[error("Maximum include depth exceeded: {depth} levels deep in file '{file}'")]
    MaxIncludeDepthExceeded { depth: usize, file: String },

    #[error("Role '{role}' not found. Searched paths: {searched_paths:?}")]
    RoleNotFound {
        role: String,
        searched_paths: Vec<String>,
    },

    #[error("Security violation: {message}")]
    SecurityViolation { message: String },

    #[error("Include file '{file}' not found or not accessible")]
    IncludeFileNotFound { file: String },

    #[error("Invalid include directive: {message}")]
    InvalidIncludeDirective { message: String },

    #[error("Include variable resolution failed: {variable} in {file}")]
    IncludeVariableResolution { variable: String, file: String },
}
