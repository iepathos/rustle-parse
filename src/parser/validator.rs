use crate::parser::error::ParseError;
use std::path::Path;
use tokio::fs;

pub async fn validate_playbook_syntax(path: &Path) -> Result<(), ParseError> {
    let content = fs::read_to_string(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ParseError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            }
        } else {
            ParseError::Io(e)
        }
    })?;

    // Basic YAML syntax validation
    let _: serde_yaml::Value = serde_yaml::from_str(&content)?;

    // TODO: Add Ansible-specific validation rules

    Ok(())
}
