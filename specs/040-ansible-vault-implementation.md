# Spec 040: Ansible Vault Implementation

## Feature Summary

Implement complete Ansible Vault support for encrypting and decrypting sensitive data in playbooks and inventory files. This includes support for vault passwords, multiple vault IDs, encrypted strings, encrypted files, and integration with the template engine for runtime decryption.

**Problem it solves**: The current vault implementation is a placeholder that returns an error. Real Ansible projects use vault encryption for passwords, API keys, and other sensitive data that must be decrypted during playbook parsing and execution.

**High-level approach**: Implement AES-256 encryption/decryption compatible with Ansible's vault format, support multiple vault IDs, integrate with password sources, and provide seamless decryption during template resolution.

## Goals & Requirements

### Functional Requirements
- Decrypt Ansible vault-encrypted strings and files
- Support multiple vault IDs and password sources
- Handle vault format versions (1.1, 1.2, 2.0)
- Integrate with template engine for runtime decryption
- Support vault password files and interactive prompts
- Validate vault data integrity with HMAC
- Cache decrypted content for performance
- Support vault variables in inventories and playbooks

### Non-functional Requirements
- **Security**: Use secure cryptographic libraries and practices
- **Performance**: Cache decrypted content, decrypt on-demand
- **Compatibility**: 100% compatible with Ansible vault format
- **Memory Safety**: Secure memory handling for passwords and decrypted data
- **Error Handling**: Clear errors for decryption failures

### Success Criteria
- All Ansible vault formats decrypt correctly
- Multi-vault-ID scenarios work properly
- Integration with templates and variables
- Secure password handling and memory management
- Performance acceptable for large vaults

## API/Interface Design

### Vault Decryption Interface
```rust
use ring::aead;
use ring::pbkdf2;
use ring::hmac;

pub struct VaultDecryptor {
    passwords: HashMap<Option<String>, String>, // vault_id -> password
    cache: LruCache<String, String>,            // content_hash -> decrypted_content
}

impl VaultDecryptor {
    /// Create new vault decryptor with passwords
    pub fn new() -> Self;
    
    /// Add password for default vault ID
    pub fn add_password(&mut self, password: String);
    
    /// Add password for specific vault ID
    pub fn add_vault_password(&mut self, vault_id: String, password: String);
    
    /// Load password from file
    pub async fn load_password_file(&mut self, vault_id: Option<String>, path: &Path) -> Result<(), VaultError>;
    
    /// Decrypt vault-encrypted content
    pub fn decrypt(&self, encrypted_content: &str) -> Result<String, VaultError>;
    
    /// Decrypt vault-encrypted content with specific vault ID
    pub fn decrypt_with_id(&self, encrypted_content: &str, vault_id: &str) -> Result<String, VaultError>;
    
    /// Check if content is vault-encrypted
    pub fn is_vault_encrypted(content: &str) -> bool;
    
    /// Extract vault ID from encrypted content
    pub fn extract_vault_id(content: &str) -> Option<String>;
    
    /// Validate vault format and integrity
    pub fn validate_vault_content(content: &str) -> Result<VaultMetadata, VaultError>;
}

/// Vault content metadata
#[derive(Debug, Clone)]
pub struct VaultMetadata {
    pub vault_id: Option<String>,
    pub format_version: VaultFormatVersion,
    pub cipher: String,
    pub key_length: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VaultFormatVersion {
    V1_1,
    V1_2, 
    V2_0,
}
```

### Integration with Template Engine
```rust
impl TemplateEngine {
    /// Render value with vault decryption support
    pub fn render_value_with_vault(
        &self,
        value: &serde_json::Value,
        vars: &HashMap<String, serde_json::Value>,
        vault_decryptor: Option<&VaultDecryptor>,
    ) -> Result<serde_json::Value, ParseError>;
    
    /// Check if string contains vault-encrypted content
    fn contains_vault_content(&self, content: &str) -> bool;
    
    /// Decrypt vault content in template context
    fn decrypt_vault_content(&self, content: &str, vault_decryptor: &VaultDecryptor) -> Result<String, ParseError>;
}
```

### Vault Error Types
```rust
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Invalid vault format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Vault decryption failed: {message}")]
    DecryptionFailed { message: String },
    
    #[error("No password provided for vault ID '{vault_id}'")]
    NoPassword { vault_id: String },
    
    #[error("Invalid vault password for vault ID '{vault_id}'")]
    InvalidPassword { vault_id: String },
    
    #[error("Unsupported vault format version: {version}")]
    UnsupportedVersion { version: String },
    
    #[error("Vault integrity check failed")]
    IntegrityCheckFailed,
    
    #[error("IO error reading vault password file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    
    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}
```

## File and Package Structure

### New Vault Module Structure
```
src/
├── parser/
│   ├── vault/
│   │   ├── mod.rs                 # Vault module exports
│   │   ├── decryptor.rs           # Main vault decryption logic
│   │   ├── crypto.rs              # Cryptographic primitives
│   │   ├── format.rs              # Vault format parsing
│   │   ├── password.rs            # Password source handling
│   │   └── cache.rs               # Decryption result caching
│   ├── vault.rs                   # Main vault interface (enhanced)
│   ├── template.rs                # Enhanced with vault support
│   └── error.rs                   # Add VaultError integration
├── types/
│   └── vault.rs                   # Vault-related types
└── ...

tests/
├── fixtures/
│   ├── vault/
│   │   ├── encrypted_strings.txt  # Various encrypted string examples
│   │   ├── encrypted_files/       # Full encrypted YAML files
│   │   ├── passwords/             # Test password files
│   │   └── multi_vault/           # Multi-vault-ID scenarios
│   └── playbooks/
│       └── with_vault.yml         # Playbooks using vault variables
└── parser/
    ├── vault_tests.rs             # Comprehensive vault tests
    └── vault_integration_tests.rs # Integration with parsing
```

### Enhanced Existing Files
- `src/parser/mod.rs`: Export vault functionality
- `src/parser/template.rs`: Add vault decryption support
- `src/parser/error.rs`: Integrate VaultError
- `src/types/parsed.rs`: Add vault metadata to parsed structures

## Implementation Details

### Phase 1: Vault Format Parsing
```rust
// src/parser/vault/format.rs
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug)]
pub struct VaultContent {
    pub vault_id: Option<String>,
    pub format_version: VaultFormatVersion,
    pub cipher: String,
    pub salt: Vec<u8>,
    pub hmac: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

impl VaultContent {
    pub fn parse(content: &str) -> Result<Self, VaultError> {
        let content = content.trim();
        
        // Check for vault marker
        if !content.starts_with("$ANSIBLE_VAULT;") {
            return Err(VaultError::InvalidFormat {
                message: "Content does not start with $ANSIBLE_VAULT marker".to_string(),
            });
        }
        
        // Parse header line
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Err(VaultError::InvalidFormat {
                message: "Empty vault content".to_string(),
            });
        }
        
        let header = lines[0];
        let (format_version, vault_id) = Self::parse_header(header)?;
        
        // Parse vault body (base64 encoded)
        let body_lines: Vec<&str> = lines[1..].iter().cloned().collect();
        let body = body_lines.join("");
        let decoded = BASE64.decode(body)?;
        
        match format_version {
            VaultFormatVersion::V1_1 => Self::parse_v1_1(decoded, vault_id),
            VaultFormatVersion::V1_2 => Self::parse_v1_2(decoded, vault_id),
            VaultFormatVersion::V2_0 => Self::parse_v2_0(decoded, vault_id),
        }
    }
    
    fn parse_header(header: &str) -> Result<(VaultFormatVersion, Option<String>), VaultError> {
        // Format: $ANSIBLE_VAULT;1.1;AES256
        // Or: $ANSIBLE_VAULT;1.2;AES256;vault_id
        let parts: Vec<&str> = header.split(';').collect();
        
        if parts.len() < 3 {
            return Err(VaultError::InvalidFormat {
                message: "Invalid vault header format".to_string(),
            });
        }
        
        let version = match parts[1] {
            "1.1" => VaultFormatVersion::V1_1,
            "1.2" => VaultFormatVersion::V1_2,
            "2.0" => VaultFormatVersion::V2_0,
            v => return Err(VaultError::UnsupportedVersion {
                version: v.to_string(),
            }),
        };
        
        let vault_id = if parts.len() > 3 && !parts[3].is_empty() {
            Some(parts[3].to_string())
        } else {
            None
        };
        
        Ok((version, vault_id))
    }
    
    fn parse_v1_1(data: Vec<u8>, vault_id: Option<String>) -> Result<VaultContent, VaultError> {
        // Format: salt + hmac + ciphertext
        if data.len() < 80 { // 32 salt + 32 hmac + min ciphertext
            return Err(VaultError::InvalidFormat {
                message: "Vault data too short".to_string(),
            });
        }
        
        let salt = data[0..32].to_vec();
        let hmac = data[32..64].to_vec();
        let ciphertext = data[64..].to_vec();
        
        Ok(VaultContent {
            vault_id,
            format_version: VaultFormatVersion::V1_1,
            cipher: "AES256".to_string(),
            salt,
            hmac,
            ciphertext,
        })
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