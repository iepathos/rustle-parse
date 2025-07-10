# Rustle: Rust-based Configuration Management Tool

## Project Overview

**Rustle** is a high-performance configuration management tool written in Rust that provides Ansible-compatible YAML syntax while delivering superior performance through native binary execution on target hosts.

### Goals

- **Primary**: Ansible-compatible playbook syntax with 10x+ performance improvement
- **Secondary**: Eliminate Python dependency issues and version conflicts
- **Tertiary**: Provide single-binary deployment for edge/embedded systems
- **Long-term**: Full backward compatibility with existing Ansible playbooks

### Non-Goals

- Perfect Ansible compatibility in v1.0 (evolutionary approach)
- GUI interface (CLI-first design)
- Real-time interactive sessions (batch execution focus)

## Architecture

### Execution Model

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Controller    │    │   Target Host   │    │   Target Host   │
│   (rustle CLI)  │    │                 │    │                 │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │  Playbook   │ │    │ │   Compiled  │ │    │ │   Compiled  │ │
│ │  Parser     │ │────┼─┤   Binary    │ │    │ │   Binary    │ │
│ │             │ │    │ │   Runner    │ │    │ │   Runner    │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │  Binary     │ │    │ │   Module    │ │    │ │   Module    │ │
│ │  Compiler   │ │    │ │   Library   │ │    │ │   Library   │ │
│ │             │ │    │ │   (Static)  │ │    │ │   (Static)  │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Core Components

1. **Controller (rustle CLI)**
   - YAML parser with Jinja2-compatible templating
   - Task dependency resolver
   - Binary compiler for target architecture
   - SSH transport layer
   - Inventory management

2. **Target Binary (rustle-runner)**
   - Embedded playbook data
   - Statically linked module implementations
   - Local execution engine
   - Result reporting back to controller

3. **Module System**
   - Native Rust modules for performance-critical tasks
   - Python compatibility layer for existing modules
   - Plugin architecture for custom modules

## YAML Format & Compatibility

### Target Compatibility Level

**Phase 1 (v0.1-0.5)**: Core Ansible subset
- Basic playbook structure
- Essential modules (file, copy, template, shell, package)
- Variables and facts
- Conditional execution (when)
- Loops (with_items, loop)

**Phase 2 (v0.6-0.9)**: Extended compatibility
- Handlers and notifiers
- Includes and imports
- Roles support
- Vault encryption
- Python module wrapper

**Phase 3 (v1.0+)**: Full compatibility
- All Ansible core modules
- Complex templating
- Custom module support
- Ansible Galaxy integration

### Example Playbook

```yaml
---
- name: Configure web servers
  hosts: webservers
  become: yes
  vars:
    nginx_port: 80
    app_name: myapp
  
  tasks:
    - name: Install nginx
      package:
        name: nginx
        state: present
      notify: restart nginx
    
    - name: Configure nginx
      template:
        src: nginx.conf.j2
        dest: /etc/nginx/nginx.conf
        backup: yes
      notify: restart nginx
    
    - name: Start nginx service
      service:
        name: nginx
        state: started
        enabled: yes
  
  handlers:
    - name: restart nginx
      service:
        name: nginx
        state: restarted
```

## Module System

### Core Modules (Phase 1)

**File Operations**
- `file` - File/directory management
- `copy` - Copy files to remote hosts
- `template` - Template processing with Jinja2
- `fetch` - Fetch files from remote hosts
- `synchronize` - Efficient file synchronization

**System Management**
- `service` - Service management (systemd, init.d)
- `package` - Package management (apt, yum, dnf, etc.)
- `user` - User account management
- `group` - Group management
- `mount` - Mount point management

**Command Execution**
- `shell` - Shell command execution
- `command` - Raw command execution
- `script` - Script execution

**Network**
- `uri` - HTTP/HTTPS requests
- `get_url` - Download files

### Module Architecture

```rust
trait Module {
    fn name(&self) -> &str;
    fn execute(&self, args: &ModuleArgs, context: &ExecutionContext) -> ModuleResult;
    fn check_mode(&self, args: &ModuleArgs, context: &ExecutionContext) -> bool;
}

struct ModuleArgs {
    params: HashMap<String, Value>,
    variables: HashMap<String, Value>,
}

struct ExecutionContext {
    working_dir: PathBuf,
    user: String,
    sudo: bool,
    check_mode: bool,
}

struct ModuleResult {
    changed: bool,
    failed: bool,
    msg: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    data: HashMap<String, Value>,
}
```

## Connection & Transport

### SSH Transport Layer

```rust
struct SshTransport {
    host: String,
    port: u16,
    user: String,
    key_file: Option<PathBuf>,
    password: Option<String>,
    connection_timeout: Duration,
    command_timeout: Duration,
}

impl Transport for SshTransport {
    fn connect(&mut self) -> Result<(), TransportError>;
    fn upload_file(&self, local: &Path, remote: &Path) -> Result<(), TransportError>;
    fn execute_binary(&self, binary_path: &Path, args: &[String]) -> Result<ExecutionResult, TransportError>;
    fn download_file(&self, remote: &Path, local: &Path) -> Result<(), TransportError>;
}
```

### Connection Features

- **Connection multiplexing**: Single SSH connection per host
- **Connection persistence**: Keep connections alive between tasks
- **Parallel execution**: Concurrent execution across hosts
- **Error handling**: Robust retry logic and connection recovery
- **Authentication**: SSH keys, passwords, agents, jump hosts

## CLI Interface

### Primary Commands

```bash
# Execute playbook
rustle-playbook playbook.yml -i inventory.ini

# Execute specific tasks
rustle-playbook playbook.yml -i inventory.ini --tags deploy

# Check mode (dry run)
rustle-playbook playbook.yml -i inventory.ini --check

# Limit to specific hosts
rustle-playbook playbook.yml -i inventory.ini --limit webservers

# Increase verbosity
rustle-playbook playbook.yml -i inventory.ini -vvv

# Run ad-hoc commands
rustle all -i inventory.ini -m shell -a "uptime"
```

### Configuration

```toml
# rustle.toml
[defaults]
host_key_checking = false
timeout = 10
forks = 50
remote_user = "root"
private_key_file = "~/.ssh/id_rsa"

[ssh_connection]
ssh_args = "-o ControlMaster=auto -o ControlPersist=60s"
control_path = "~/.rustle/cp/rustle-ssh-%%h-%%p-%%r"
```

## Inventory & Variables

### Inventory Format

```ini
[webservers]
web1.example.com ansible_host=192.168.1.10
web2.example.com ansible_host=192.168.1.11

[dbservers]
db1.example.com ansible_host=192.168.1.20

[production:children]
webservers
dbservers

[production:vars]
env=production
backup_server=backup.example.com
```

### Variable Precedence

1. Command line variables (`-e var=value`)
2. Task variables
3. Block variables
4. Role variables
5. Play variables
6. Host variables
7. Group variables
8. Inventory variables
9. Default variables

## Performance & Security

### Performance Targets

- **Startup time**: <100ms for playbook parsing
- **Execution time**: 10x faster than Ansible for common tasks
- **Memory usage**: <50MB RSS for typical playbooks
- **Network efficiency**: Minimal SSH round-trips
- **Parallel execution**: 100+ concurrent hosts

### Security Features

- **Memory safety**: Rust's ownership system prevents buffer overflows
- **Secure defaults**: Host key checking enabled by default
- **Credential management**: Integration with SSH agents and key files
- **Audit logging**: Comprehensive execution logging
- **Privilege escalation**: Secure sudo handling

### Binary Security

- **Static linking**: No dynamic library dependencies
- **Minimal attack surface**: Single binary with embedded modules
- **Checksums**: Binary integrity verification
- **Code signing**: Optional binary signing for enterprise

## Development Roadmap

### Phase 1 (MVP - 3 months)
- [ ] YAML parser with basic Jinja2 support
- [ ] Core execution engine
- [ ] SSH transport layer
- [ ] 10 essential modules
- [ ] Basic inventory support
- [ ] CLI interface

### Phase 2 (Beta - 6 months)
- [ ] Handler system
- [ ] Role support
- [ ] Vault encryption
- [ ] Python module compatibility layer
- [ ] Advanced templating
- [ ] Error handling improvements

### Phase 3 (Stable - 12 months)
- [ ] Full Ansible module compatibility
- [ ] Performance optimizations
- [ ] Enterprise features
- [ ] Ansible Galaxy integration
- [ ] Migration tools

## Technical Dependencies

### Core Libraries
- `serde` + `serde_yaml` - YAML parsing
- `tera` - Jinja2-compatible templating
- `tokio` - Async runtime
- `openssh` - SSH client
- `clap` - CLI argument parsing
- `log` + `env_logger` - Logging
- `anyhow` - Error handling

### Optional Dependencies
- `pyo3` - Python module compatibility
- `ring` - Cryptography for Vault
- `tar` - Archive handling
- `flate2` - Compression

## Success Metrics

### Adoption Metrics
- GitHub stars and forks
- Crates.io downloads
- Community contributions
- Enterprise adoption

### Performance Metrics
- Benchmark comparisons with Ansible
- Memory usage profiles
- Network efficiency measurements
- Scaling tests (1000+ hosts)

### Compatibility Metrics
- Ansible playbook compatibility percentage
- Module coverage
- Test suite pass rate
- Community feedback

---

*This specification is a living document that will evolve based on community feedback and implementation learnings.*