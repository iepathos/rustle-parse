# Spec 040: Vault Integration for Modular Architecture

## Feature Summary

**UPDATED**: This spec has been revised to reflect the modular architecture design. Vault functionality will be implemented as a separate `rustle-vault` tool, with rustle-parse providing integration markers and basic vault detection.

Implement vault integration markers in rustle-parse to identify vault-encrypted content and defer decryption to the specialized `rustle-vault` tool. This maintains separation of concerns while enabling seamless vault support in the parsing pipeline.

**Problem it solves**: The current vault implementation is a placeholder. Real Ansible projects use vault encryption, but cryptographic operations should be isolated in a dedicated tool for security and modularity.

**High-level approach**: Implement vault content detection and marker generation in rustle-parse, with pipeline integration to `rustle-vault` for actual decryption operations.

## Goals & Requirements

### Functional Requirements (rustle-parse)
- Detect Ansible vault-encrypted content in YAML
- Generate vault markers with location and metadata
- Support vault format identification (1.1, 1.2, 2.0)
- Extract vault IDs from encrypted content
- Preserve vault content in parsed output for pipeline processing
- Handle vault content in templates, variables, and files

### Functional Requirements (rustle-vault - separate tool)
- Decrypt all Ansible vault formats
- Support multiple vault IDs and password sources
- Handle password files and interactive prompts
- Validate vault data integrity with HMAC
- Process vault markers from rustle-parse output

### Non-functional Requirements
- **Security**: Cryptographic operations isolated in rustle-vault
- **Performance**: Streaming integration between tools
- **Compatibility**: 100% compatible with Ansible vault format
- **Modularity**: Clean separation of parsing and cryptographic concerns
- **Error Handling**: Clear errors with fallback strategies

### Success Criteria
- Vault content properly detected and marked in rustle-parse
- Seamless integration with rustle-vault tool
- Pipeline processing works correctly
- Error handling and fallback to SSH execution
- Performance acceptable with tool composition

## API/Interface Design

### Vault Detection Interface (rustle-parse)
```rust
use serde::{Deserialize, Serialize};

/// Vault content detector for rustle-parse
pub struct VaultDetector;

impl VaultDetector {
    /// Check if content is vault-encrypted
    pub fn is_vault_encrypted(content: &str) -> bool;
    
    /// Extract vault metadata without decryption
    pub fn extract_metadata(content: &str) -> Result<VaultMetadata, ParseError>;
    
    /// Create vault marker for pipeline processing
    pub fn create_marker(content: &str, location: String) -> Result<VaultMarker, ParseError>;
}

/// Vault marker for inter-tool communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMarker {
    pub location: String,              // JSONPath to vault content
    pub vault_id: Option<String>,      // Extracted vault ID
    pub format_version: VaultFormatVersion,
    pub encrypted_data: String,        // Original encrypted content
}

/// Vault content metadata (parsing only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultMetadata {
    pub vault_id: Option<String>,
    pub format_version: VaultFormatVersion,
    pub cipher: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VaultFormatVersion {
    V1_1,
    V1_2, 
    V2_0,
}

/// Enhanced parsed output with vault markers
#[derive(Serialize, Deserialize)]
pub struct ParsedOutput {
    pub playbook: ParsedPlaybook,
    pub vault_markers: Vec<VaultMarker>,
    pub template_markers: Vec<TemplateMarker>,
    pub metadata: ParseMetadata,
}
```

### rustle-vault Tool Interface (separate tool)
```bash
# CLI interface for rustle-vault tool
rustle-vault decrypt [OPTIONS] [INPUT]
rustle-vault encrypt [OPTIONS] [INPUT] 
rustle-vault resolve-markers [OPTIONS] < parsed.json > resolved.json
rustle-vault scan-and-decrypt [OPTIONS] playbook.yml

# Integration with rustle-parse
rustle-parse playbook.yml --defer-vault | rustle-vault resolve-markers --password-file vault-pass
```

### Integration with Template Engine
```rust
impl TemplateEngine {
    /// Create template marker for vault content in templates
    pub fn create_template_marker_for_vault(
        &self,
        template_expr: &str,
        location: String,
    ) -> Result<TemplateMarker, ParseError>;
    
    /// Check if template contains vault variables
    fn contains_vault_variables(&self, template: &str) -> bool;
    
    /// Extract vault-related variable dependencies
    fn extract_vault_dependencies(&self, template: &str) -> Vec<String>;
}
```

### Error Types (rustle-parse integration)
```rust
#[derive(Debug, Error)]
pub enum VaultDetectionError {
    #[error("Invalid vault format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Unsupported vault format version: {version}")]
    UnsupportedVersion { version: String },
    
    #[error("Base64 decode error in vault header: {0}")]
    Base64Error(#[from] base64::DecodeError),
    
    #[error("UTF-8 decode error in vault header: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

// Integrate into existing ParseError
impl From<VaultDetectionError> for ParseError {
    fn from(err: VaultDetectionError) -> Self {
        ParseError::VaultDetection {
            message: err.to_string(),
        }
    }
}
```

## File and Package Structure

### Vault Detection Module Structure (rustle-parse only)
```
src/
├── parser/
│   ├── vault.rs                   # Vault detection and marker creation
│   ├── template.rs                # Template marker creation
│   └── error.rs                   # Add VaultDetectionError integration
├── types/
│   ├── parsed.rs                  # Enhanced with vault/template markers
│   └── vault.rs                   # Vault marker types
└── ...

tests/
├── fixtures/
│   ├── vault/
│   │   ├── encrypted_strings.txt  # Various encrypted string examples
│   │   ├── encrypted_files/       # Full encrypted YAML files
│   │   └── multi_vault/           # Multi-vault-ID scenarios
│   └── playbooks/
│       └── with_vault.yml         # Playbooks using vault variables
└── parser/
    ├── vault_detection_tests.rs   # Vault detection tests
    └── vault_integration_tests.rs # Integration marker tests
```

### Enhanced Existing Files
- `src/parser/mod.rs`: Export vault detection functionality
- `src/parser/template.rs`: Add vault marker support
- `src/parser/error.rs`: Integrate VaultDetectionError
- `src/types/parsed.rs`: Add vault and template markers
- `src/bin/rustle-parse.rs`: Add CLI flags for vault integration

## Implementation Details

### Phase 1: Vault Detection (rustle-parse)
```rust
// src/parser/vault.rs - Detection only, no decryption
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

impl VaultDetector {
    pub fn is_vault_encrypted(content: &str) -> bool {
        content.trim().starts_with("$ANSIBLE_VAULT;")
    }
    
    pub fn extract_metadata(content: &str) -> Result<VaultMetadata, ParseError> {
        let content = content.trim();
        
        // Check for vault marker
        if !Self::is_vault_encrypted(content) {
            return Err(ParseError::VaultDetection {
                message: "Content is not vault-encrypted".to_string(),
            });
        }
        
        // Parse header line only (no decryption)
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Err(ParseError::VaultDetection {
                message: "Empty vault content".to_string(),
            });
        }
        
        let header = lines[0];
        let (format_version, vault_id) = Self::parse_header(header)?;
        
        Ok(VaultMetadata {
            vault_id,
            format_version,
            cipher: "AES256".to_string(), // Standard for all versions
        })
    }
    
    pub fn create_marker(content: &str, location: String) -> Result<VaultMarker, ParseError> {
        let metadata = Self::extract_metadata(content)?;
        
        Ok(VaultMarker {
            location,
            vault_id: metadata.vault_id,
            format_version: metadata.format_version,
            encrypted_data: content.to_string(),
        })
    }
    
    fn parse_header(header: &str) -> Result<(VaultFormatVersion, Option<String>), ParseError> {
        // Format: $ANSIBLE_VAULT;1.1;AES256
        // Or: $ANSIBLE_VAULT;1.2;AES256;vault_id
        let parts: Vec<&str> = header.split(';').collect();
        
        if parts.len() < 3 {
            return Err(ParseError::VaultDetection {
                message: "Invalid vault header format".to_string(),
            });
        }
        
        let version = match parts[1] {
            "1.1" => VaultFormatVersion::V1_1,
            "1.2" => VaultFormatVersion::V1_2,
            "2.0" => VaultFormatVersion::V2_0,
            v => return Err(ParseError::VaultDetection {
                message: format!("Unsupported vault format version: {}", v),
            }),
        };
        
        let vault_id = if parts.len() > 3 && !parts[3].is_empty() {
            Some(parts[3].to_string())
        } else {
            None
        };
        
        Ok((version, vault_id))
    }
}
```

### Phase 2: Cryptographic Operations
```rust
// src/parser/vault/crypto.rs
use ring::{aead, digest, hkdf, pbkdf2};
use ring::rand::{SecureRandom, SystemRandom};

pub struct VaultCrypto;

impl VaultCrypto {
    /// Derive encryption key from password and salt using PBKDF2
    pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], VaultError> {
        let mut key = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(10000).unwrap(), // Ansible default iterations
            salt,
            password.as_bytes(),
            &mut key,
        );
        Ok(key)
    }
    
    /// Decrypt AES-256-CTR ciphertext
    pub fn decrypt_aes256_ctr(
        key: &[u8; 32],
        iv: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        use aes::Aes256;
        use ctr::Ctr128BE;
        use cipher::{KeyIvInit, StreamCipher};
        
        type Aes256Ctr = Ctr128BE<Aes256>;
        
        let mut cipher = Aes256Ctr::new(key.into(), iv.into());
        let mut plaintext = ciphertext.to_vec();
        cipher.apply_keystream(&mut plaintext);
        
        Ok(plaintext)
    }
    
    /// Verify HMAC-SHA256 integrity
    pub fn verify_hmac(
        key: &[u8],
        data: &[u8],
        expected_hmac: &[u8],
    ) -> Result<(), VaultError> {
        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, key);
        match hmac::verify(&signing_key, data, expected_hmac) {
            Ok(_) => Ok(()),
            Err(_) => Err(VaultError::IntegrityCheckFailed),
        }
    }
    
    /// Remove PKCS7 padding from decrypted content
    pub fn remove_pkcs7_padding(data: &[u8]) -> Result<Vec<u8>, VaultError> {
        if data.is_empty() {
            return Err(VaultError::DecryptionFailed {
                message: "Empty data for padding removal".to_string(),
            });
        }
        
        let padding_length = data[data.len() - 1] as usize;
        
        if padding_length == 0 || padding_length > 16 || padding_length > data.len() {
            return Err(VaultError::DecryptionFailed {
                message: "Invalid PKCS7 padding".to_string(),
            });
        }
        
        // Verify padding bytes
        for &byte in &data[data.len() - padding_length..] {
            if byte != padding_length as u8 {
                return Err(VaultError::DecryptionFailed {
                    message: "Invalid PKCS7 padding bytes".to_string(),
                });
            }
        }
        
        Ok(data[..data.len() - padding_length].to_vec())
    }
}
```

### Phase 3: Vault Decryptor Implementation
```rust
// src/parser/vault/decryptor.rs
use lru::LruCache;
use std::num::NonZeroUsize;

impl VaultDecryptor {
    pub fn new() -> Self {
        Self {
            passwords: HashMap::new(),
            cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
        }
    }
    
    pub fn decrypt(&self, encrypted_content: &str) -> Result<String, VaultError> {
        // Parse vault content
        let vault_content = VaultContent::parse(encrypted_content)?;
        
        // Check cache first
        let cache_key = self.generate_cache_key(encrypted_content);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        // Get password for vault ID
        let password = self.get_password(&vault_content.vault_id)?;
        
        // Decrypt content
        let decrypted = self.decrypt_content(&vault_content, &password)?;
        
        // Cache result
        self.cache.put(cache_key, decrypted.clone());
        
        Ok(decrypted)
    }
    
    fn decrypt_content(&self, vault_content: &VaultContent, password: &str) -> Result<String, VaultError> {
        match vault_content.format_version {
            VaultFormatVersion::V1_1 | VaultFormatVersion::V1_2 => {
                self.decrypt_v1(vault_content, password)
            }
            VaultFormatVersion::V2_0 => {
                self.decrypt_v2(vault_content, password)
            }
        }
    }
    
    fn decrypt_v1(&self, vault_content: &VaultContent, password: &str) -> Result<String, VaultError> {
        // Derive keys
        let encryption_key = VaultCrypto::derive_key(password, &vault_content.salt)?;
        let hmac_key = VaultCrypto::derive_key(
            &format!("{}\n", password), // Ansible adds newline for HMAC key
            &vault_content.salt,
        )?;
        
        // Verify HMAC
        VaultCrypto::verify_hmac(&hmac_key, &vault_content.ciphertext, &vault_content.hmac)?;
        
        // Decrypt (first 16 bytes are IV for AES)
        if vault_content.ciphertext.len() < 16 {
            return Err(VaultError::DecryptionFailed {
                message: "Ciphertext too short for IV".to_string(),
            });
        }
        
        let iv = &vault_content.ciphertext[0..16];
        let ciphertext = &vault_content.ciphertext[16..];
        
        let decrypted = VaultCrypto::decrypt_aes256_ctr(&encryption_key, iv, ciphertext)?;
        
        // Remove padding
        let unpadded = VaultCrypto::remove_pkcs7_padding(&decrypted)?;
        
        // Convert to string
        let result = String::from_utf8(unpadded)
            .map_err(|e| VaultError::DecryptionFailed {
                message: format!("Invalid UTF-8 in decrypted content: {}", e),
            })?;
        
        Ok(result)
    }
    
    fn get_password(&self, vault_id: &Option<String>) -> Result<String, VaultError> {
        match self.passwords.get(vault_id) {
            Some(password) => Ok(password.clone()),
            None => {
                let id_str = vault_id.as_ref().map(|s| s.as_str()).unwrap_or("default");
                Err(VaultError::NoPassword {
                    vault_id: id_str.to_string(),
                })
            }
        }
    }
    
    fn generate_cache_key(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
```

### Phase 4: Template Integration
```rust
// Enhanced src/parser/template.rs
impl TemplateEngine {
    pub fn render_value_with_vault(
        &self,
        value: &serde_json::Value,
        vars: &HashMap<String, serde_json::Value>,
        vault_decryptor: Option<&VaultDecryptor>,
    ) -> Result<serde_json::Value, ParseError> {
        match value {
            serde_json::Value::String(s) => {
                let mut processed = s.clone();
                
                // First decrypt any vault content
                if let Some(decryptor) = vault_decryptor {
                    processed = self.decrypt_vault_in_string(&processed, decryptor)?;
                }
                
                // Then render templates
                if processed.contains("{{") && processed.contains("}}") {
                    let rendered = self.render_string(&processed, vars)?;
                    Ok(serde_json::Value::String(rendered))
                } else {
                    Ok(serde_json::Value::String(processed))
                }
            }
            serde_json::Value::Object(obj) => {
                let mut rendered_obj = serde_json::Map::new();
                for (k, v) in obj {
                    let rendered_value = self.render_value_with_vault(v, vars, vault_decryptor)?;
                    rendered_obj.insert(k.clone(), rendered_value);
                }
                Ok(serde_json::Value::Object(rendered_obj))
            }
            serde_json::Value::Array(arr) => {
                let mut rendered_arr = Vec::new();
                for item in arr {
                    let rendered_item = self.render_value_with_vault(item, vars, vault_decryptor)?;
                    rendered_arr.push(rendered_item);
                }
                Ok(serde_json::Value::Array(rendered_arr))
            }
            _ => Ok(value.clone()),
        }
    }
    
    fn decrypt_vault_in_string(&self, content: &str, decryptor: &VaultDecryptor) -> Result<String, ParseError> {
        if !VaultDecryptor::is_vault_encrypted(content) {
            return Ok(content.to_string());
        }
        
        decryptor.decrypt(content)
            .map_err(|e| ParseError::VaultDecryption {
                message: e.to_string(),
            })
    }
}
```

## Testing Strategy

### Unit Testing Requirements
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vault_format_parsing() {
        let vault_content = r#"$ANSIBLE_VAULT;1.1;AES256
33363965326261303234396464623732373034373366353462363763636163393464386565316333
3438626638653333373066386464373236656363366235650a396130306533633562623533656165
35653934373338373836313739626638643461336137323163366565333737623039353064653361
6133386361616464330a3137313332623134343135306164626561616432383936346466353439"#;
        
        let parsed = VaultContent::parse(vault_content).unwrap();
        assert_eq!(parsed.format_version, VaultFormatVersion::V1_1);
        assert!(parsed.vault_id.is_none());
        assert_eq!(parsed.cipher, "AES256");
    }
    
    #[test]
    fn test_vault_decryption() {
        let password = "test_password";
        let mut decryptor = VaultDecryptor::new();
        decryptor.add_password(password.to_string());
        
        // This is "hello world" encrypted with test_password
        let encrypted = r#"$ANSIBLE_VAULT;1.1;AES256
33363965326261303234396464623732373034373366353462363763636163393464386565316333
3438626638653333373066386464373236656363366235650a396130306533633562623533656165
35653934373338373836313739626638643461336137323163366565333737623039353064653361
6133386361616464330a3137313332623134343135306164626561616432383936346466353439"#;
        
        let decrypted = decryptor.decrypt(encrypted).unwrap();
        assert_eq!(decrypted, "hello world");
    }
    
    #[test]
    fn test_vault_id_support() {
        let mut decryptor = VaultDecryptor::new();
        decryptor.add_vault_password("prod".to_string(), "prod_password".to_string());
        decryptor.add_vault_password("dev".to_string(), "dev_password".to_string());
        
        let prod_encrypted = r#"$ANSIBLE_VAULT;1.2;AES256;prod
..."#;
        
        let decrypted = decryptor.decrypt(prod_encrypted).unwrap();
        // Verify correct password was used
    }
    
    #[test]
    fn test_template_vault_integration() {
        let mut decryptor = VaultDecryptor::new();
        decryptor.add_password("secret123".to_string());
        
        let template_with_vault = serde_json::json!({
            "password": "$ANSIBLE_VAULT;1.1;AES256\n...",
            "message": "The password is {{ password }}"
        });
        
        let vars = HashMap::new();
        let engine = TemplateEngine::new();
        
        let result = engine.render_value_with_vault(&template_with_vault, &vars, Some(&decryptor)).unwrap();
        
        // Verify vault was decrypted and template was rendered
    }
}
```

### Integration Testing Requirements
```rust
// tests/parser/vault_integration_tests.rs
#[tokio::test]
async fn test_playbook_with_vault_vars() {
    let playbook_content = r#"
---
- hosts: all
  vars:
    secret_password: !vault |
      $ANSIBLE_VAULT;1.1;AES256
      33363965326261303234396464623732373034373366353462363763636163393464386565316333
      3438626638653333373066386464373236656363366235650a396130306533633562623533656165
      35653934373338373836313739626638643461336137323163366565333737623039353064653361
      6133386361616464330a3137313332623134343135306164626561616432383936346466353439
  tasks:
    - name: Use secret
      debug:
        msg: "Secret is {{ secret_password }}"
"#;
    
    let mut parser = Parser::new();
    parser = parser.with_vault_password("test_password".to_string());
    
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), playbook_content).unwrap();
    
    let parsed = parser.parse_playbook(temp_file.path()).await.unwrap();
    
    // Verify vault variable was decrypted
    assert!(parsed.variables.contains_key("secret_password"));
    let decrypted = parsed.variables.get("secret_password").unwrap();
    assert_eq!(decrypted.as_str().unwrap(), "hello world");
}

#[tokio::test]
async fn test_inventory_with_vault_vars() {
    let inventory_content = r#"
[webservers]
web1 ansible_password=!vault |
  $ANSIBLE_VAULT;1.1;AES256
  ...

[webservers:vars]
db_password=!vault |
  $ANSIBLE_VAULT;1.1;AES256
  ...
"#;
    
    let mut parser = Parser::new();
    parser = parser.with_vault_password("vault_pass".to_string());
    
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), inventory_content).unwrap();
    
    let parsed = parser.parse_inventory(temp_file.path()).await.unwrap();
    
    // Verify vault variables were decrypted
    let web1 = parsed.hosts.get("web1").unwrap();
    assert!(web1.vars.contains_key("ansible_password"));
}
```

## Edge Cases & Error Handling

### Critical Edge Cases
1. **Invalid Vault Formats**
   - Corrupted base64 encoding
   - Wrong header format
   - Truncated vault content
   - Invalid padding

2. **Password Management**
   - Missing passwords for vault IDs
   - Wrong passwords causing decryption failures
   - Password file permissions and access
   - Interactive password prompts

3. **Security Concerns**
   - Memory leaks of decrypted content
   - Cache poisoning attacks
   - Timing attacks on password verification
   - Secure deletion of sensitive data

4. **Integration Issues**
   - Vault content in templates
   - Circular references with vault variables
   - Performance with large vault files
   - Error propagation in parsing pipeline

### Error Recovery Strategies
```rust
impl VaultDecryptor {
    /// Attempt to decrypt with all available passwords if vault ID is missing
    pub fn decrypt_with_fallback(&self, encrypted_content: &str) -> Result<String, VaultError> {
        let vault_content = VaultContent::parse(encrypted_content)?;
        
        if vault_content.vault_id.is_some() {
            // Use specific vault ID
            return self.decrypt(encrypted_content);
        }
        
        // Try all passwords if no vault ID specified
        let mut last_error = None;
        for (_, password) in &self.passwords {
            match self.decrypt_content(&vault_content, password) {
                Ok(result) => return Ok(result),
                Err(e) => last_error = Some(e),
            }
        }
        
        Err(last_error.unwrap_or(VaultError::NoPassword {
            vault_id: "any".to_string(),
        }))
    }
    
    /// Clear sensitive data from memory
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        // TODO: Secure memory clearing
    }
}
```

## Dependencies

### New Cryptographic Dependencies
```toml
[dependencies]
# Cryptography
ring = "0.17"           # Cryptographic primitives
aes = "0.8"            # AES encryption
ctr = "0.9"            # CTR mode
sha2 = "0.10"          # SHA-256 hashing (already present)
base64 = "0.22"        # Base64 encoding (already present)

# Caching
lru = "0.12"           # LRU cache for decrypted content

# Password handling
rpassword = "7.3"      # Secure password input
```

### Security Dependencies
```toml
[dependencies]
# Secure memory handling
zeroize = "1.8"        # Secure memory clearing
```

## Configuration

### Vault Configuration Options
```rust
pub struct VaultConfig {
    pub cache_size: usize,              // Number of cached decryptions
    pub cache_ttl: Duration,            // Cache time-to-live
    pub max_file_size: usize,           // Maximum vault file size
    pub password_timeout: Duration,     // Interactive password timeout
    pub secure_memory: bool,            // Use secure memory clearing
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            cache_size: 100,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            max_file_size: 10 * 1024 * 1024,     // 10MB
            password_timeout: Duration::from_secs(30),
            secure_memory: true,
        }
    }
}
```

### Environment Variables
```bash
ANSIBLE_VAULT_PASSWORD_FILE=/path/to/password/file
ANSIBLE_VAULT_IDENTITY_LIST=dev@/dev/password,prod@/prod/password
RUSTLE_VAULT_CACHE_SIZE=200
RUSTLE_VAULT_SECURE_MEMORY=true
```

## Documentation

### Vault Usage Documentation
```rust
/// Ansible Vault decryption support for rustle-parse.
/// 
/// This module provides complete Ansible Vault compatibility including:
/// - Multiple vault format versions (1.1, 1.2, 2.0)
/// - Multi-vault-ID support
/// - Integration with template engine
/// - Secure password handling
/// 
/// # Examples
/// 
/// ## Basic vault decryption
/// ```rust
/// use rustle_parse::parser::VaultDecryptor;
/// 
/// let mut decryptor = VaultDecryptor::new();
/// decryptor.add_password("my_vault_password".to_string());
/// 
/// let encrypted = "$ANSIBLE_VAULT;1.1;AES256\n...";
/// let decrypted = decryptor.decrypt(encrypted)?;
/// ```
/// 
/// ## Multi-vault-ID support
/// ```rust
/// let mut decryptor = VaultDecryptor::new();
/// decryptor.add_vault_password("prod".to_string(), "prod_password".to_string());
/// decryptor.add_vault_password("dev".to_string(), "dev_password".to_string());
/// 
/// let prod_secret = "$ANSIBLE_VAULT;1.2;AES256;prod\n...";
/// let decrypted = decryptor.decrypt(prod_secret)?;
/// ```
/// 
/// ## Integration with parser
/// ```rust
/// let mut parser = Parser::new();
/// parser = parser.with_vault_password("secret123".to_string());
/// 
/// let playbook = parser.parse_playbook("playbook_with_vault.yml").await?;
/// // Vault variables are automatically decrypted
/// ```
pub struct VaultDecryptor { /* ... */ }
```

## Performance Considerations

### Cryptographic Performance
- Use hardware-accelerated AES when available
- Cache derived keys to avoid repeated PBKDF2 operations
- Implement lazy decryption - only decrypt when needed
- Use secure memory pools for sensitive data

### Memory Management
```rust
// Use zeroize for secure memory clearing
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
struct SensitiveData {
    password: String,
    decrypted_content: String,
}

impl Drop for VaultDecryptor {
    fn drop(&mut self) {
        // Secure cleanup of sensitive data
        for (_, mut password) in self.passwords.drain() {
            password.zeroize();
        }
        self.cache.clear();
    }
}
```

## Security Considerations

### Cryptographic Security
- Use constant-time operations for password verification
- Implement secure random number generation
- Follow OWASP guidelines for key derivation
- Regular security audits of cryptographic code

### Memory Security
```rust
// Secure string handling
use secstr::SecStr;

pub struct SecureVaultDecryptor {
    passwords: HashMap<Option<String>, SecStr>,
    // ... other fields
}

impl SecureVaultDecryptor {
    pub fn add_password(&mut self, password: SecStr) {
        self.passwords.insert(None, password);
    }
    
    pub fn decrypt_secure(&self, encrypted_content: &str) -> Result<SecStr, VaultError> {
        // Decrypt to secure string to avoid plaintext in memory
        // ... implementation
    }
}
```

## Implementation Phases

### Phase 1: Core Vault Support (Week 1)
- [ ] Implement vault format parsing
- [ ] Add basic AES-256 decryption
- [ ] Support vault format versions 1.1 and 1.2
- [ ] Basic password management
- [ ] Unit tests for core functionality

### Phase 2: Integration (Week 2)
- [ ] Integrate with template engine
- [ ] Add vault support to playbook parser
- [ ] Add vault support to inventory parser
- [ ] Error handling and validation
- [ ] Integration tests

### Phase 3: Advanced Features (Week 3)
- [ ] Multi-vault-ID support
- [ ] Password file support
- [ ] Caching and performance optimization
- [ ] Security enhancements
- [ ] Comprehensive testing

### Phase 4: Production Readiness (Week 4)
- [ ] Security audit and testing
- [ ] Performance benchmarking
- [ ] Documentation and examples
- [ ] Real-world vault file testing
- [ ] Memory leak testing

## Success Metrics

### Functional Metrics
- All Ansible vault formats decrypt correctly
- Multi-vault-ID scenarios work properly
- Template integration seamless
- Error messages clear and actionable

### Security Metrics
- No sensitive data leaks in memory
- Secure password handling verified
- Cryptographic implementation audited
- Timing attack resistance verified

### Performance Metrics
- Vault decryption <100ms for typical files
- Cache hit ratio >90% for repeated access
- Memory usage proportional to active vaults
- No memory leaks in long-running processes