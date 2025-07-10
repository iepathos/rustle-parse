# Spec 100: Complete Stub Implementations

## Feature Summary

Complete the implementation of stub modules that currently exist but are not fully functional: cache, validator, and vault. These modules are referenced throughout the codebase but contain minimal or placeholder implementations. This feature brings them to production-ready status with full functionality, comprehensive error handling, and thorough test coverage.

## Goals & Requirements

### Functional Requirements
- Implement full cache functionality for parsed results
- Complete validator module with comprehensive Ansible syntax validation
- Implement vault decryption support for encrypted variables and files
- Ensure all modules integrate seamlessly with existing parser workflow
- Maintain compatibility with Ansible ecosystem standards

### Non-functional Requirements
- Performance: Cache operations should be sub-millisecond
- Security: Vault operations must be cryptographically secure
- Reliability: All modules must handle edge cases gracefully
- Memory efficiency: Cache should have configurable size limits
- Thread safety: All operations must be safe for concurrent use

### Success Criteria
- All TODOs in stub modules are resolved
- Full test coverage for all implemented functionality
- Integration tests demonstrate end-to-end workflows
- Performance benchmarks meet established targets
- Security audit passes for vault implementation

## API/Interface Design

### Cache Module API

```rust
/// Manages cached parse results to improve performance.
///
/// The cache stores serialized parse results keyed by file path and
/// modification time. It automatically invalidates entries when
/// source files change.
pub struct Cache {
    storage: Box<dyn CacheStorage>,
    config: CacheConfig,
}

/// Configuration for cache behavior.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries to store
    pub max_entries: usize,
    /// Maximum total size in bytes
    pub max_size_bytes: usize,
    /// Time-to-live for entries in seconds
    pub ttl_seconds: u64,
    /// Whether to enable cache compression
    pub enable_compression: bool,
}

impl Cache {
    /// Creates a new cache with the specified storage backend.
    pub fn new(storage: Box<dyn CacheStorage>, config: CacheConfig) -> Self;
    
    /// Retrieves a cached parse result if valid.
    pub async fn get(&self, key: &CacheKey) -> Result<Option<CachedResult>, CacheError>;
    
    /// Stores a parse result in the cache.
    pub async fn put(&self, key: CacheKey, result: CachedResult) -> Result<(), CacheError>;
    
    /// Invalidates cache entries for the given path.
    pub async fn invalidate(&self, path: &Path) -> Result<(), CacheError>;
    
    /// Clears all cache entries.
    pub async fn clear(&self) -> Result<(), CacheError>;
    
    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats;
}

/// Unique identifier for cached items.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub path: PathBuf,
    pub modified_time: SystemTime,
    pub content_hash: String,
    pub vars_hash: String,
}

/// Cached parse result with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub data: Vec<u8>,
    pub created_at: SystemTime,
    pub content_type: ContentType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Playbook,
    Inventory,
    Template,
}
```

### Validator Module API

```rust
/// Validates Ansible playbook and inventory syntax.
///
/// Provides comprehensive validation of Ansible constructs including
/// module arguments, variable references, task structure, and more.
pub struct Validator {
    config: ValidatorConfig,
    module_registry: ModuleRegistry,
}

/// Configuration for validation behavior.
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Whether to enforce strict mode validation
    pub strict_mode: bool,
    /// Ansible version to validate against
    pub ansible_version: AnsibleVersion,
    /// Custom module definitions
    pub custom_modules: Vec<ModuleDefinition>,
    /// Whether to validate variable references
    pub check_undefined_vars: bool,
}

impl Validator {
    /// Creates a new validator with default configuration.
    pub fn new() -> Self;
    
    /// Creates a validator with custom configuration.
    pub fn with_config(config: ValidatorConfig) -> Self;
    
    /// Validates a parsed playbook structure.
    pub fn validate_playbook(&self, playbook: &ParsedPlaybook) -> Result<ValidationReport, ValidationError>;
    
    /// Validates a parsed inventory structure.
    pub fn validate_inventory(&self, inventory: &ParsedInventory) -> Result<ValidationReport, ValidationError>;
    
    /// Validates a single task definition.
    pub fn validate_task(&self, task: &ParsedTask) -> Result<ValidationReport, ValidationError>;
    
    /// Validates variable references in templates.
    pub fn validate_template(&self, template: &str, vars: &HashMap<String, serde_json::Value>) -> Result<ValidationReport, ValidationError>;
}

/// Validation results with warnings and errors.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub is_valid: bool,
}

/// Individual validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Source location for validation issues.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}
```

### Vault Module API

```rust
/// Handles Ansible Vault encryption and decryption.
///
/// Supports all Ansible Vault formats and encryption methods,
/// providing secure access to encrypted variables and files.
pub struct Vault {
    passwords: HashMap<String, String>,
    config: VaultConfig,
}

/// Configuration for vault operations.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Default vault ID to use
    pub default_vault_id: Option<String>,
    /// Whether to prompt for passwords if not provided
    pub prompt_for_passwords: bool,
    /// Maximum time to cache decrypted content
    pub cache_ttl_seconds: u64,
}

impl Vault {
    /// Creates a new vault handler.
    pub fn new() -> Self;
    
    /// Adds a vault password for the specified vault ID.
    pub fn add_password(&mut self, vault_id: String, password: String);
    
    /// Loads vault passwords from a file.
    pub async fn load_password_file(&mut self, path: &Path) -> Result<(), VaultError>;
    
    /// Decrypts an encrypted string.
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String, VaultError>;
    
    /// Encrypts a string with the specified vault ID.
    pub fn encrypt_string(&self, plaintext: &str, vault_id: Option<&str>) -> Result<String, VaultError>;
    
    /// Decrypts an entire file.
    pub async fn decrypt_file(&self, path: &Path) -> Result<String, VaultError>;
    
    /// Checks if a string is vault-encrypted.
    pub fn is_encrypted(data: &str) -> bool;
    
    /// Extracts vault ID from encrypted data.
    pub fn extract_vault_id(data: &str) -> Option<String>;
}

/// Errors that can occur during vault operations.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Invalid vault format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Decryption failed: vault ID '{vault_id}' password not available")]
    PasswordNotAvailable { vault_id: String },
    
    #[error("Decryption failed: invalid password or corrupted data")]
    DecryptionFailed,
    
    #[error("Unsupported vault version: {version}")]
    UnsupportedVersion { version: String },
}
```

## File and Package Structure

### Cache Implementation
```
src/parser/cache/
├── mod.rs              # Public API and main Cache struct
├── storage.rs          # Storage backend trait and implementations
├── config.rs           # Configuration types
├── key.rs              # Cache key generation and hashing
├── compression.rs      # Optional compression support
└── stats.rs            # Cache statistics tracking
```

### Validator Implementation
```
src/parser/validator/
├── mod.rs              # Public API and main Validator struct
├── playbook.rs         # Playbook-specific validation rules
├── inventory.rs        # Inventory validation rules
├── task.rs             # Task validation logic
├── template.rs         # Template validation
├── modules/            # Module-specific validation
│   ├── mod.rs
│   ├── builtin.rs      # Built-in Ansible modules
│   └── registry.rs     # Module registry
└── rules.rs            # Validation rule definitions
```

### Vault Implementation
```
src/parser/vault/
├── mod.rs              # Public API and main Vault struct
├── encryption.rs       # Encryption/decryption algorithms
├── format.rs           # Vault format parsing
├── password.rs         # Password management
└── cache.rs            # Decrypted content caching
```

## Implementation Details

### Phase 1: Cache Implementation

1. **Storage Backend Trait**
```rust
#[async_trait]
pub trait CacheStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<(), CacheError>;
    async fn remove(&self, key: &str) -> Result<(), CacheError>;
    async fn clear(&self) -> Result<(), CacheError>;
    async fn size(&self) -> Result<usize, CacheError>;
}

// Implementations:
// - FileSystemStorage: Stores cache files in a directory
// - MemoryStorage: In-memory LRU cache
// - RedisStorage: Redis backend for distributed caching
```

2. **Cache Key Generation**
```rust
impl CacheKey {
    pub async fn from_file(path: &Path, vars: &HashMap<String, serde_json::Value>) -> Result<Self, CacheError> {
        let metadata = tokio::fs::metadata(path).await?;
        let modified_time = metadata.modified()?;
        
        // Hash file contents for integrity
        let content = tokio::fs::read(path).await?;
        let content_hash = format!("{:x}", sha2::Sha256::digest(&content));
        
        // Hash variables that affect parsing
        let vars_json = serde_json::to_string(vars)?;
        let vars_hash = format!("{:x}", sha2::Sha256::digest(vars_json.as_bytes()));
        
        Ok(CacheKey {
            path: path.to_path_buf(),
            modified_time,
            content_hash,
            vars_hash,
        })
    }
}
```

### Phase 2: Validator Implementation

1. **Module Registry**
```rust
pub struct ModuleRegistry {
    modules: HashMap<String, ModuleDefinition>,
}

impl ModuleRegistry {
    pub fn load_builtin_modules() -> Self {
        let mut registry = Self { modules: HashMap::new() };
        
        // Load built-in Ansible modules
        registry.add_module("copy", ModuleDefinition {
            name: "copy".to_string(),
            required_args: vec!["dest".to_string()],
            optional_args: vec!["src".to_string(), "content".to_string(), "mode".to_string()],
            mutually_exclusive: vec![vec!["src".to_string(), "content".to_string()]],
        });
        
        // ... more modules
        registry
    }
    
    pub fn validate_module_args(&self, module: &str, args: &serde_json::Value) -> Result<(), ValidationError> {
        let definition = self.modules.get(module)
            .ok_or_else(|| ValidationError::UnknownModule { name: module.to_string() })?;
            
        // Validate required arguments are present
        // Validate mutually exclusive arguments
        // Check for unknown arguments
        Ok(())
    }
}
```

2. **Playbook Validation Rules**
```rust
impl Validator {
    fn validate_playbook_structure(&self, playbook: &ParsedPlaybook) -> Result<Vec<ValidationIssue>, ValidationError> {
        let mut issues = Vec::new();
        
        // Validate play structure
        for (play_idx, play) in playbook.plays.iter().enumerate() {
            // Check required fields
            if play.name.is_empty() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: "Play name is empty".to_string(),
                    location: Some(SourceLocation {
                        file: playbook.path.clone(),
                        line: play_idx + 1, // Approximate
                        column: 1,
                    }),
                    suggestion: Some("Consider adding a descriptive name for this play".to_string()),
                });
            }
            
            // Validate tasks
            for task in &play.tasks {
                issues.extend(self.validate_task_structure(task)?);
            }
        }
        
        Ok(issues)
    }
}
```

### Phase 3: Vault Implementation

1. **Vault Format Detection**
```rust
impl Vault {
    pub fn parse_vault_header(data: &str) -> Result<VaultHeader, VaultError> {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() || !lines[0].starts_with("$ANSIBLE_VAULT;") {
            return Err(VaultError::InvalidFormat {
                message: "Missing vault header".to_string(),
            });
        }
        
        let header_parts: Vec<&str> = lines[0].split(';').collect();
        if header_parts.len() < 4 {
            return Err(VaultError::InvalidFormat {
                message: "Invalid header format".to_string(),
            });
        }
        
        Ok(VaultHeader {
            version: header_parts[1].to_string(),
            cipher: header_parts[2].to_string(),
            vault_id: if header_parts.len() > 4 {
                Some(header_parts[3].to_string())
            } else {
                None
            },
        })
    }
}
```

2. **AES256 Decryption Implementation**
```rust
use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

impl Vault {
    fn decrypt_aes256_cbc(&self, encrypted_data: &[u8], password: &str, salt: &[u8]) -> Result<Vec<u8>, VaultError> {
        // Derive key using PBKDF2
        let mut key = [0u8; 32];
        let mut iv = [0u8; 16];
        
        pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha256>>(
            password.as_bytes(),
            salt,
            10000, // iterations
            &mut key,
        );
        
        // Use first 16 bytes of key as IV (Ansible-compatible)
        iv.copy_from_slice(&key[..16]);
        
        let cipher = Aes256Cbc::new_from_slices(&key, &iv)
            .map_err(|_| VaultError::DecryptionFailed)?;
            
        cipher.decrypt_vec(encrypted_data)
            .map_err(|_| VaultError::DecryptionFailed)
    }
}
```

## Testing Strategy

### Cache Testing
- Unit tests for all cache operations
- Integration tests with real parser workflows
- Performance tests with large datasets
- Concurrency tests for thread safety
- Storage backend tests for each implementation

### Validator Testing
- Test validation of all Ansible constructs
- Error and warning generation tests
- Module argument validation tests
- Template validation tests
- Edge case and malformed input tests

### Vault Testing
- Test all supported vault formats (1.1, 1.2, 2.0)
- Encryption/decryption round-trip tests
- Password management tests
- Integration tests with real vault files
- Security tests for timing attacks

### Test Data
```
tests/fixtures/
├── cache/
│   ├── valid_cache_entries/
│   └── corrupted_cache_entries/
├── validator/
│   ├── valid_playbooks/
│   ├── invalid_playbooks/
│   └── module_args/
└── vault/
    ├── vault_1_1_files/
    ├── vault_1_2_files/
    ├── vault_2_0_files/
    └── password_files/
```

## Edge Cases & Error Handling

### Cache Edge Cases
- Disk space exhaustion during cache writes
- Corrupted cache files
- Concurrent access to same cache entries
- Cache size limit enforcement
- File system permission issues

### Validator Edge Cases
- Deeply nested variable references
- Circular task dependencies
- Unknown module detection
- Malformed YAML edge cases
- Unicode in task names and variables

### Vault Edge Cases
- Missing vault passwords
- Corrupted encrypted data
- Unsupported vault versions
- Invalid base64 encoding
- Wrong passwords (security)

## Dependencies

### Required Crates
```toml
[dependencies]
# Existing dependencies...

# Cache dependencies
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["fs"] }
sha2 = "0.10"
lru = "0.12"

# Validator dependencies
regex = "1.0"
semver = "1.0"

# Vault dependencies
aes = "0.8"
block-modes = "0.9"
pbkdf2 = "0.12"
hmac = "0.12"
base64 = "0.22"

[dev-dependencies]
tempfile = "3.0"
criterion = "0.6"
```

### Optional Dependencies
- `redis` - For Redis cache backend
- `zstd` - For cache compression

## Configuration

### Cache Configuration
```rust
// Default configuration
impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            ttl_seconds: 3600, // 1 hour
            enable_compression: true,
        }
    }
}
```

### Integration with Parser
```rust
impl Parser {
    pub fn with_cache<P: AsRef<Path>>(mut self, cache_dir: P) -> Self {
        let storage = Box::new(FileSystemStorage::new(cache_dir));
        let config = CacheConfig::default();
        self.cache = Some(Cache::new(storage, config));
        self
    }
    
    pub fn with_validator(mut self, validator: Validator) -> Self {
        self.validator = Some(validator);
        self
    }
}
```

## Documentation

### Module Documentation
- Each module needs comprehensive rustdoc
- Usage examples for all major functions
- Security considerations for vault operations
- Performance characteristics for cache operations

### Integration Examples
```rust
// Complete example in documentation
use rustle_parse::{Parser, Validator, Vault, Cache};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up vault with password
    let mut vault = Vault::new();
    vault.load_password_file("vault-password.txt").await?;
    
    // Create validator with strict mode
    let validator = Validator::new()
        .with_strict_mode(true)
        .with_ansible_version("2.15");
    
    // Create parser with all features
    let parser = Parser::new()
        .with_cache("/tmp/rustle-cache")
        .with_vault(vault)
        .with_validator(validator);
    
    // Parse with full validation and caching
    let playbook = parser.parse_playbook("site.yml").await?;
    println!("Successfully parsed {} plays", playbook.plays.len());
    
    Ok(())
}
```

## Success Metrics

1. **Functionality**: All stub methods fully implemented
2. **Performance**: Cache hit rates > 90% for repeated parses
3. **Security**: Vault operations pass security audit
4. **Reliability**: Zero panics in error conditions
5. **Testing**: > 95% test coverage for all modules
6. **Documentation**: Complete rustdoc for all public APIs