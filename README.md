# rustle-parse

A specialized YAML and inventory parser for Ansible-compatible playbooks that converts playbooks and inventory files into structured JSON/binary format. This tool centralizes all parsing logic into a single, focused tool that can be reused across the Rustle ecosystem.

## 🚀 Features

- **Parse Ansible-compatible YAML playbooks** with full support for plays, tasks, handlers, and roles
- **Multiple inventory formats** - INI, YAML, and JSON inventory parsing
- **Jinja2 template resolution** with Ansible-compatible filters
- **Syntax validation** and comprehensive error reporting with line numbers
- **Multiple output formats** - JSON, YAML, and binary (planned)
- **High performance** - built in Rust for speed and memory efficiency
- **Comprehensive CLI** with various inspection and validation modes

## 📦 Installation

### From Source

```bash
git clone <repository-url> rustle-parse
cd rustle-parse
cargo build --release
```

The binary will be available at `target/release/rustle-parse`.

### Development Setup

```bash
# Install development dependencies
rustup component add rustfmt clippy
cargo install cargo-watch cargo-tarpaulin cargo-audit

# Run in development mode
cargo run -- --help
```

## 🛠️ Usage

### Basic Usage

```bash
# Parse a playbook and output JSON
rustle-parse playbook.yml

# Parse with inventory
rustle-parse -i hosts.ini playbook.yml

# Parse with extra variables
rustle-parse -e "env=prod,debug=true" playbook.yml

# Output in different formats
rustle-parse -o yaml playbook.yml
rustle-parse -o json playbook.yml
```

### Validation and Inspection

```bash
# Validate syntax only
rustle-parse --syntax-check playbook.yml

# List all tasks with metadata
rustle-parse --list-tasks playbook.yml

# List all hosts from inventory
rustle-parse --list-hosts -i inventory.ini

# Dry run (parse but don't output)
rustle-parse --dry-run playbook.yml
```

### Advanced Options

```bash
# Use vault password file
rustle-parse -v vault-password.txt playbook.yml

# Enable verbose logging
rustle-parse --verbose playbook.yml

# Use caching for better performance
rustle-parse -c /tmp/cache playbook.yml
```

## 📋 Command Line Reference

```
rustle-parse [OPTIONS] [PLAYBOOK_FILE]

Arguments:
  [PLAYBOOK_FILE]  Path to playbook file (or stdin if -)

Options:
  -i, --inventory <FILE>            Inventory file path
  -e, --extra-vars <VARS>           Extra variables (key=value,...)
  -o, --output <OUTPUT>             Output format [default: json] [possible values: json, binary, yaml]
  -c, --cache-dir <DIR>             Cache directory for parsed results
  -v, --vault-password-file <FILE>  Vault password file
      --syntax-check                Only validate syntax, don't output
      --list-tasks                  List all tasks with metadata
      --list-hosts                  List all hosts with variables
      --verbose                     Enable verbose output
      --dry-run                     Parse but don't write output
  -h, --help                        Print help
  -V, --version                     Print version
```

## 📁 Project Structure

```
rustle-parse/
├── src/
│   ├── bin/
│   │   └── rustle-parse.rs        # CLI binary entry point
│   ├── parser/
│   │   ├── mod.rs                 # Parser module exports
│   │   ├── playbook.rs            # Playbook parsing logic
│   │   ├── inventory.rs           # Inventory parsing logic
│   │   ├── template.rs            # Jinja2 template engine
│   │   ├── error.rs               # Error types and handling
│   │   ├── vault.rs               # Vault decryption (planned)
│   │   ├── cache.rs               # Parse result caching (planned)
│   │   ├── validator.rs           # Syntax validation
│   │   └── dependency.rs          # Dependency resolution
│   ├── types/
│   │   ├── parsed.rs              # Parsed data structures
│   │   └── output.rs              # Output format types
│   └── lib.rs                     # Library exports
├── tests/
│   ├── fixtures/                  # Test playbooks and inventories
│   └── parser/                    # Integration tests
├── specs/                         # Specification documents
├── Cargo.toml                     # Project manifest
└── README.md                      # This file
```

## 🔍 Output Format

The tool outputs structured JSON by default. Here's an example of parsed playbook output:

```json
{
  "metadata": {
    "file_path": "playbook.yml",
    "version": null,
    "created_at": "2025-07-10T02:12:32.663108Z",
    "checksum": "d48e92ff5b2b8cd603041d0d6a56a9c4674696e8e3c7601a6c526e6a37adea50"
  },
  "plays": [
    {
      "name": "Example play",
      "hosts": "all",
      "vars": {},
      "tasks": [
        {
          "id": "task_0",
          "name": "Example task",
          "module": "debug",
          "args": {
            "msg": "Hello World"
          },
          "tags": [],
          "when": null,
          "dependencies": []
        }
      ],
      "handlers": [],
      "roles": []
    }
  ],
  "variables": {},
  "facts_required": false,
  "vault_ids": []
}
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_tests

# Generate code coverage
cargo tarpaulin --out Html
```

## 🔧 Development

### Development Workflow

```bash
# Run with hot reloading
cargo watch -x "run -- --help"

# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check compilation
cargo check
```

### Architecture

The parser is built with a modular architecture:

- **Parser Core**: Handles YAML deserialization and data structure conversion
- **Template Engine**: Processes Jinja2 templates with Ansible-compatible filters
- **Error Handling**: Comprehensive error types with context and line numbers
- **CLI Interface**: Full-featured command-line interface with multiple modes

### Key Dependencies

- `serde` & `serde_yaml` - YAML parsing and serialization
- `minijinja` - Jinja2 template engine
- `clap` - Command-line argument parsing
- `tokio` - Async runtime for file I/O
- `thiserror` & `anyhow` - Error handling
- `tracing` - Structured logging

## 🎯 Roadmap

### Current Status ✅

- [x] Basic YAML playbook parsing
- [x] Template resolution with Jinja2
- [x] CLI interface with all major features
- [x] Comprehensive error handling
- [x] Integration tests and fixtures

### Planned Features 🔄

- [ ] Complete INI inventory parsing
- [ ] Ansible Vault decryption
- [ ] Parse result caching
- [ ] Binary output format
- [ ] Performance optimizations
- [ ] Dynamic inventory script support

### Future Enhancements 🔮

- [ ] Dependency graph visualization
- [ ] Advanced syntax validation
- [ ] Integration with other Rustle tools
- [ ] Plugin system for custom modules

## 📄 Specifications

This implementation follows [Specification 030: Rustle Parse Tool](specs/030-rustle-parse.md). See the specs directory for detailed requirements and design decisions.

## 🤝 Contributing

1. Follow the guidelines in `CLAUDE.md`
2. Ensure all tests pass: `cargo test`
3. Run formatters: `cargo fmt`
4. Check lints: `cargo clippy`
5. Update documentation as needed

## 📝 License

[Add your license here]

---

Built with ❤️ in Rust for the Rustle automation ecosystem.