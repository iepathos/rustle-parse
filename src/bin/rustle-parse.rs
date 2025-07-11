use clap::{Parser, ValueEnum};
use rustle_parse::{OutputFormat, ParseError, Parser as RustleParser};
use std::collections::HashMap;
use std::io::{self, Read};
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
        for pair in vars_str.split(',') {
            if let Some((key, value)) = pair.split_once('=') {
                // Try to parse as JSON first, fall back to string
                let parsed_value = serde_json::from_str(value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
                vars.insert(key.trim().to_string(), parsed_value);
            }
        }
    }

    Ok(vars)
}

fn get_playbook_path(cli: &Cli) -> Result<PathBuf, ParseError> {
    match &cli.playbook_file {
        Some(path) if path == "-" => {
            // Read from stdin and create a temporary file
            let mut content = String::new();
            io::stdin()
                .read_to_string(&mut content)
                .map_err(ParseError::Io)?;

            // For now, just return an error - stdin support needs temp file handling
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
