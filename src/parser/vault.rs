use crate::parser::error::ParseError;

pub struct VaultDecryptor {
    #[allow(dead_code)]
    password: String,
}

impl VaultDecryptor {
    pub fn new(password: String) -> Self {
        Self { password }
    }

    pub fn decrypt(&self, _encrypted_data: &str) -> Result<String, ParseError> {
        // TODO: Implement vault decryption
        Err(ParseError::UnsupportedFeature {
            feature: "Vault decryption".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixtures::VAULT_ENCRYPTED_CONTENT;

    #[test]
    fn test_vault_decryptor_creation() {
        let password = "test_password".to_string();
        let decryptor = VaultDecryptor::new(password.clone());
        assert_eq!(decryptor.password, password);
    }

    #[test]
    fn test_vault_decryptor_decrypt_returns_error() {
        let decryptor = VaultDecryptor::new("password".to_string());
        let result = decryptor.decrypt(VAULT_ENCRYPTED_CONTENT);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnsupportedFeature { .. }
        ));
    }

    #[test]
    fn test_vault_decryptor_decrypt_with_empty_data() {
        let decryptor = VaultDecryptor::new("password".to_string());
        let result = decryptor.decrypt("");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnsupportedFeature { .. }
        ));
    }

    #[test]
    fn test_vault_decryptor_decrypt_with_invalid_data() {
        let decryptor = VaultDecryptor::new("password".to_string());
        let invalid_data = "this is not vault encrypted data";
        let result = decryptor.decrypt(invalid_data);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnsupportedFeature { .. }
        ));
    }

    #[test]
    fn test_vault_decryptor_with_different_passwords() {
        let passwords = vec!["pass1", "pass2", "very_long_password_123"];

        for password in passwords {
            let decryptor = VaultDecryptor::new(password.to_string());
            assert_eq!(decryptor.password, password);

            // All should return the same unsupported feature error
            let result = decryptor.decrypt("test_data");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                ParseError::UnsupportedFeature { .. }
            ));
        }
    }

    #[test]
    fn test_vault_decryptor_error_message() {
        let decryptor = VaultDecryptor::new("password".to_string());
        let result = decryptor.decrypt("test");

        match result.unwrap_err() {
            ParseError::UnsupportedFeature { feature } => {
                assert_eq!(feature, "Vault decryption");
            }
            _ => panic!("Expected UnsupportedFeature error"),
        }
    }
}
