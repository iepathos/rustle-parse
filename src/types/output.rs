use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Binary,
    Yaml,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "binary" => Ok(OutputFormat::Binary),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(format!("Unknown output format: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Json));
    }

    #[test]
    fn test_output_format_from_str_valid() {
        assert!(matches!(
            "json".parse::<OutputFormat>().unwrap(),
            OutputFormat::Json
        ));
        assert!(matches!(
            "binary".parse::<OutputFormat>().unwrap(),
            OutputFormat::Binary
        ));
        assert!(matches!(
            "yaml".parse::<OutputFormat>().unwrap(),
            OutputFormat::Yaml
        ));
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert!(matches!(
            "JSON".parse::<OutputFormat>().unwrap(),
            OutputFormat::Json
        ));
        assert!(matches!(
            "Binary".parse::<OutputFormat>().unwrap(),
            OutputFormat::Binary
        ));
        assert!(matches!(
            "YAML".parse::<OutputFormat>().unwrap(),
            OutputFormat::Yaml
        ));
        assert!(matches!(
            "JsOn".parse::<OutputFormat>().unwrap(),
            OutputFormat::Json
        ));
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        let result = "invalid".parse::<OutputFormat>();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown output format: invalid"));
    }

    #[test]
    fn test_output_format_serialization() {
        let json_format = OutputFormat::Json;
        let serialized = serde_json::to_string(&json_format).unwrap();
        assert_eq!(serialized, r#""json""#);

        let binary_format = OutputFormat::Binary;
        let serialized = serde_json::to_string(&binary_format).unwrap();
        assert_eq!(serialized, r#""binary""#);

        let yaml_format = OutputFormat::Yaml;
        let serialized = serde_json::to_string(&yaml_format).unwrap();
        assert_eq!(serialized, r#""yaml""#);
    }

    #[test]
    fn test_output_format_deserialization() {
        let json_format: OutputFormat = serde_json::from_str(r#""json""#).unwrap();
        assert!(matches!(json_format, OutputFormat::Json));

        let binary_format: OutputFormat = serde_json::from_str(r#""binary""#).unwrap();
        assert!(matches!(binary_format, OutputFormat::Binary));

        let yaml_format: OutputFormat = serde_json::from_str(r#""yaml""#).unwrap();
        assert!(matches!(yaml_format, OutputFormat::Yaml));
    }

    #[test]
    fn test_output_format_debug() {
        let format = OutputFormat::Json;
        let debug_str = format!("{:?}", format);
        assert_eq!(debug_str, "Json");
    }

    #[test]
    fn test_output_format_clone() {
        let format = OutputFormat::Binary;
        let cloned = format.clone();
        assert!(matches!(cloned, OutputFormat::Binary));
    }

    #[test]
    fn test_output_format_copy() {
        let format = OutputFormat::Yaml;
        let copied = format;
        assert!(matches!(copied, OutputFormat::Yaml));
        assert!(matches!(format, OutputFormat::Yaml)); // Original still valid
    }
}
