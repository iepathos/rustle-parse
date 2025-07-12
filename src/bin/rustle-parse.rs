use clap::{Parser, ValueEnum};
use rustle_parse::{OutputFormat, ParseError, Parser as RustleParser};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "rustle-parse")]
#[command(about = "Parse Ansible playbooks and inventory files")]
#[command(version = "0.1.0")]
struct Cli {
    /// Path to playbook file (or stdin if -)
    #[arg(value_name = "PLAYBOOK_FILE")]
    playbook_file: Option<String>,

    /// Inventory file path
    #[arg(short, long, value_name = "FILE")]
    inventory: Option<PathBuf>,

    /// Extra variables (key=value,...)
    #[arg(short, long, value_name = "VARS")]
    extra_vars: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    output: OutputFormatCli,

    /// Cache directory for parsed results
    #[arg(short, long, value_name = "DIR")]
    cache_dir: Option<PathBuf>,

    /// Vault password file
    #[arg(short = 'v', long, value_name = "FILE")]
    vault_password_file: Option<PathBuf>,

    /// Only validate syntax, don't output
    #[arg(long)]
    syntax_check: bool,

    /// List all tasks with metadata
    #[arg(long)]
    list_tasks: bool,

    /// List all hosts with variables
    #[arg(long)]
    list_hosts: bool,

    /// Enable verbose output
    #[arg(long)]
    verbose: bool,

    /// Parse but don't write output
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, ValueEnum)]
enum OutputFormatCli {
    Json,
    Binary,
    Yaml,
}

impl From<OutputFormatCli> for OutputFormat {
    fn from(cli_format: OutputFormatCli) -> Self {
        match cli_format {
            OutputFormatCli::Json => OutputFormat::Json,
            OutputFormatCli::Binary => OutputFormat::Binary,
            OutputFormatCli::Yaml => OutputFormat::Yaml,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Set up logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Parse extra variables
    let extra_vars = parse_extra_vars(&cli.extra_vars)?;

    // Create parser
    let mut parser = RustleParser::new().with_extra_vars(extra_vars);

    // Add cache if specified
    if let Some(ref cache_dir) = cli.cache_dir {
        parser = parser.with_cache(cache_dir.clone());
    }

    // Add vault password if specified
    if let Some(ref vault_file) = cli.vault_password_file {
        let vault_password = tokio::fs::read_to_string(vault_file).await?;
        parser = parser.with_vault_password(vault_password.trim().to_string());
    }

    // Handle different modes
    if cli.syntax_check {
        return handle_syntax_check(&parser, &cli).await;
    }

    if cli.list_hosts {
        return handle_list_hosts(&parser, &cli).await;
    }

    // Parse playbook
    let playbook_path = get_playbook_path(&cli)?;
    let playbook = parser.parse_playbook(&playbook_path).await?;

    if cli.list_tasks {
        handle_list_tasks(&playbook);
        return Ok(());
    }

    if !cli.dry_run {
        output_result(&playbook, cli.output.into())?;
    }

    info!("Parsing completed successfully");
    Ok(())
}

fn parse_extra_vars(
    extra_vars_str: &Option<String>,
) -> Result<HashMap<String, serde_json::Value>, ParseError> {
    let mut vars = HashMap::new();

    if let Some(vars_str) = extra_vars_str {
        // Simple parser that handles JSON values with commas
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut in_key = true;
        let mut depth = 0;
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in vars_str.chars() {
            if escape_next {
                if in_key {
                    current_key.push(ch);
                } else {
                    current_value.push(ch);
                }
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                if in_key {
                    current_key.push(ch);
                } else {
                    current_value.push(ch);
                }
                continue;
            }

            if ch == '"' && !escape_next {
                in_quotes = !in_quotes;
            }

            if !in_quotes {
                match ch {
                    '[' | '{' => depth += 1,
                    ']' | '}' => depth -= 1,
                    '=' if in_key => {
                        in_key = false;
                        continue;
                    }
                    ',' if depth == 0 => {
                        // End of this key-value pair
                        if !in_key {
                            // We have a proper key=value pair
                            let key = current_key.trim().to_string();
                            let value = current_value.trim();
                            if !key.is_empty() {
                                let parsed_value =
                                    serde_json::from_str(value).unwrap_or_else(|_| {
                                        serde_json::Value::String(value.to_string())
                                    });
                                vars.insert(key, parsed_value);
                            }
                        }
                        // Reset for next pair (ignore malformed entries)
                        current_key.clear();
                        current_value.clear();
                        in_key = true;
                        continue;
                    }
                    _ => {}
                }
            }

            if in_key {
                current_key.push(ch);
            } else {
                current_value.push(ch);
            }
        }

        // Handle the last key-value pair
        if !in_key {
            // We have a proper key=value pair
            let key = current_key.trim().to_string();
            let value = current_value.trim();
            if !key.is_empty() {
                let parsed_value = serde_json::from_str(value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
                vars.insert(key, parsed_value);
            }
        }
    }

    Ok(vars)
}

fn get_playbook_path(cli: &Cli) -> Result<PathBuf, ParseError> {
    match &cli.playbook_file {
        Some(path) if path == "-" => {
            // For now, just return an error - stdin support needs temp file handling
            // TODO: Implement stdin support by reading content and creating a temporary file
            Err(ParseError::UnsupportedFeature {
                feature: "stdin input".to_string(),
            })
        }
        Some(path) => Ok(PathBuf::from(path)),
        None => Err(ParseError::InvalidStructure {
            message: "No playbook file specified".to_string(),
        }),
    }
}

async fn handle_syntax_check(
    parser: &RustleParser,
    cli: &Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let playbook_path = get_playbook_path(cli)?;

    match parser.validate_syntax(&playbook_path).await {
        Ok(()) => {
            println!("Syntax validation passed");
            Ok(())
        }
        Err(e) => {
            error!("Syntax validation failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_list_hosts(
    parser: &RustleParser,
    cli: &Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(inventory_path) = &cli.inventory {
        let inventory = parser.parse_inventory(inventory_path).await?;

        for (hostname, host) in &inventory.hosts {
            println!("{hostname}:");
            if let Some(address) = &host.address {
                println!("  address: {address}");
            }
            if let Some(port) = host.port {
                println!("  port: {port}");
            }
            if let Some(user) = &host.user {
                println!("  user: {user}");
            }
            for (key, value) in &host.vars {
                println!("  {key}: {value}");
            }
        }
    } else {
        println!("No inventory file specified");
    }

    Ok(())
}

fn handle_list_tasks(playbook: &rustle_parse::ParsedPlaybook) {
    for (play_idx, play) in playbook.plays.iter().enumerate() {
        println!("Play {}: {}", play_idx + 1, play.name);

        for (task_idx, task) in play.tasks.iter().enumerate() {
            println!("  Task {}: {} ({})", task_idx + 1, task.name, task.module);

            if !task.tags.is_empty() {
                println!("    Tags: {}", task.tags.join(", "));
            }

            if let Some(when_condition) = &task.when {
                println!("    When: {when_condition}");
            }
        }

        if !play.handlers.is_empty() {
            println!("  Handlers:");
            for (handler_idx, handler) in play.handlers.iter().enumerate() {
                println!(
                    "    Handler {}: {} ({})",
                    handler_idx + 1,
                    handler.name,
                    handler.module
                );
            }
        }
    }
}

fn output_result(
    playbook: &rustle_parse::ParsedPlaybook,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(playbook)?;
            println!("{json}");
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(playbook)?;
            println!("{yaml}");
        }
        OutputFormat::Binary => {
            return Err("Binary output format not yet implemented".into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extra_vars_empty() {
        let result = parse_extra_vars(&None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_extra_vars_single_string_value() {
        let vars_str = Some("key=value".to_string());
        let result = parse_extra_vars(&vars_str).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("key").unwrap(),
            &serde_json::Value::String("value".to_string())
        );
    }

    #[test]
    fn test_parse_extra_vars_multiple_values() {
        let vars_str = Some("key1=value1,key2=value2,key3=value3".to_string());
        let result = parse_extra_vars(&vars_str).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(
            result.get("key1").unwrap(),
            &serde_json::Value::String("value1".to_string())
        );
        assert_eq!(
            result.get("key2").unwrap(),
            &serde_json::Value::String("value2".to_string())
        );
        assert_eq!(
            result.get("key3").unwrap(),
            &serde_json::Value::String("value3".to_string())
        );
    }

    #[test]
    fn test_parse_extra_vars_json_values() {
        let vars_str =
            Some(r#"str=hello,num=42,bool=true,arr=[1,2,3],obj={"key":"value"}"#.to_string());
        let result = parse_extra_vars(&vars_str).unwrap();

        assert_eq!(result.len(), 5);
        assert_eq!(
            result.get("str").unwrap(),
            &serde_json::Value::String("hello".to_string())
        );
        assert_eq!(result.get("num").unwrap(), &serde_json::json!(42));
        assert_eq!(result.get("bool").unwrap(), &serde_json::json!(true));
        assert_eq!(result.get("arr").unwrap(), &serde_json::json!([1, 2, 3]));
        assert_eq!(
            result.get("obj").unwrap(),
            &serde_json::json!({"key": "value"})
        );
    }

    #[test]
    fn test_parse_extra_vars_with_whitespace() {
        let vars_str = Some("  key1  =  value1  ,  key2 = value2  ".to_string());
        let result = parse_extra_vars(&vars_str).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("key1").unwrap(),
            &serde_json::Value::String("value1".to_string())
        );
        assert_eq!(
            result.get("key2").unwrap(),
            &serde_json::Value::String("value2".to_string())
        );
    }

    #[test]
    fn test_parse_extra_vars_malformed_entries() {
        let vars_str = Some("valid=value,no_equals_sign,another_valid=test".to_string());
        let result = parse_extra_vars(&vars_str).unwrap();

        // Only valid entries should be parsed
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get("valid").unwrap(),
            &serde_json::Value::String("value".to_string())
        );
        assert_eq!(
            result.get("another_valid").unwrap(),
            &serde_json::Value::String("test".to_string())
        );
    }

    #[test]
    fn test_parse_extra_vars_empty_string() {
        let vars_str = Some("".to_string());
        let result = parse_extra_vars(&vars_str).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_playbook_path_from_arg() {
        let cli = Cli {
            playbook_file: Some("test.yml".to_string()),
            inventory: None,
            extra_vars: None,
            output: OutputFormatCli::Json,
            cache_dir: None,
            vault_password_file: None,
            syntax_check: false,
            list_tasks: false,
            list_hosts: false,
            verbose: false,
            dry_run: false,
        };

        let result = get_playbook_path(&cli).unwrap();
        assert_eq!(result, PathBuf::from("test.yml"));
    }

    #[test]
    fn test_get_playbook_path_stdin() {
        let cli = Cli {
            playbook_file: Some("-".to_string()),
            inventory: None,
            extra_vars: None,
            output: OutputFormatCli::Json,
            cache_dir: None,
            vault_password_file: None,
            syntax_check: false,
            list_tasks: false,
            list_hosts: false,
            verbose: false,
            dry_run: false,
        };

        let result = get_playbook_path(&cli);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnsupportedFeature { feature } => {
                assert_eq!(feature, "stdin input");
            }
            _ => panic!("Expected UnsupportedFeature error"),
        }
    }

    #[test]
    fn test_get_playbook_path_none() {
        let cli = Cli {
            playbook_file: None,
            inventory: None,
            extra_vars: None,
            output: OutputFormatCli::Json,
            cache_dir: None,
            vault_password_file: None,
            syntax_check: false,
            list_tasks: false,
            list_hosts: false,
            verbose: false,
            dry_run: false,
        };

        let result = get_playbook_path(&cli);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::InvalidStructure { message } => {
                assert_eq!(message, "No playbook file specified");
            }
            _ => panic!("Expected InvalidStructure error"),
        }
    }

    #[test]
    fn test_output_format_conversion() {
        assert!(matches!(
            OutputFormat::from(OutputFormatCli::Json),
            OutputFormat::Json
        ));
        assert!(matches!(
            OutputFormat::from(OutputFormatCli::Binary),
            OutputFormat::Binary
        ));
        assert!(matches!(
            OutputFormat::from(OutputFormatCli::Yaml),
            OutputFormat::Yaml
        ));
    }
}
