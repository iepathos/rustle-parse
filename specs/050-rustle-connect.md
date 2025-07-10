# Spec 050: Rustle Connect Tool

## Feature Summary

The `rustle-connect` tool is a specialized SSH connection manager that handles all remote connectivity for the Rustle ecosystem. It provides connection multiplexing, authentication management, connection pooling, and serves as the transport layer abstraction for all remote operations.

**Problem it solves**: Centralizes all SSH connection logic, enabling efficient connection reuse, better error handling, and consistent authentication across all Rustle tools.

**High-level approach**: Create a standalone service/tool that manages SSH connections, provides connection APIs for other tools, and handles all transport-layer concerns including authentication, multiplexing, and error recovery.

## Goals & Requirements

### Functional Requirements
- Establish and manage SSH connections to remote hosts
- Implement connection multiplexing and reuse
- Handle various authentication methods (keys, passwords, agents)
- Provide connection pooling and lifecycle management
- Support SSH tunneling and port forwarding
- Implement connection health monitoring and recovery
- Provide both CLI interface and daemon mode
- Support batch connection establishment
- Handle connection failures and automatic retry

### Non-functional Requirements
- **Performance**: Establish 100 connections in &lt;5 seconds
- **Reliability**: 99.9% connection success rate with proper credentials
- **Scalability**: Handle 1000+ concurrent connections efficiently
- **Security**: Secure credential handling and storage
- **Resource Usage**: &lt;1MB memory per connection

### Success Criteria
- All Rustle tools can reliably connect through rustle-connect
- Connection setup time reduced by 80% through multiplexing
- Zero credential exposure in process lists or logs
- Automatic recovery from network interruptions
- Compatible with all standard SSH configurations

## API/Interface Design

### Command Line Interface
```bash
# Daemon mode
rustle-connect daemon [OPTIONS]

# Connection management
rustle-connect connect [OPTIONS] HOST
rustle-connect disconnect HOST
rustle-connect list-connections
rustle-connect test-connection HOST

# Tunnel management  
rustle-connect tunnel [OPTIONS] HOST LOCAL_PORT:REMOTE_PORT
rustle-connect forward [OPTIONS] HOST LOCAL_PORT:REMOTE_HOST:REMOTE_PORT

OPTIONS:
    -u, --user &lt;USER&gt;              SSH username
    -i, --identity &lt;KEY_FILE&gt;      SSH private key file
    -p, --port &lt;PORT&gt;              SSH port [default: 22]
    -o, --ssh-option &lt;OPTION&gt;      SSH configuration option
    --password-file &lt;FILE&gt;         File containing SSH password
    --use-agent                    Use SSH agent for authentication
    --control-path &lt;PATH&gt;          SSH control socket path
    --connection-timeout &lt;SECS&gt;    Connection timeout [default: 10]
    --keepalive-interval &lt;SECS&gt;    Keepalive interval [default: 30]
    --max-retries &lt;NUM&gt;            Maximum connection retries [default: 3]
    --multiplexing                 Enable SSH multiplexing [default: true]
    --compression                  Enable SSH compression
    -v, --verbose                  Enable verbose output
    --config &lt;FILE&gt;                SSH configuration file
    --socket-path &lt;PATH&gt;           Daemon socket path
```

### Core Data Structures

```rust
// Connection management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth_method: AuthMethod,
    pub ssh_options: HashMap&lt;String, String&gt;,
    pub timeout: Duration,
    pub keepalive_interval: Duration,
    pub max_retries: u32,
    pub compression: bool,
    pub multiplexing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Key { 
        private_key_path: PathBuf,
        passphrase: Option&lt;String&gt;,
    },
    Password { 
        password: String,
    },
    Agent,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub config: ConnectionConfig,
    pub status: ConnectionStatus,
    pub established_at: Option&lt;DateTime&lt;Utc&gt;&gt;,
    pub last_used: Option&lt;DateTime&lt;Utc&gt;&gt;,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub command_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Failed { error: String },
    Disconnected,
    Reconnecting,
}

// Command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCommand {
    pub command: String,
    pub args: Vec&lt;String&gt;,
    pub env: HashMap&lt;String, String&gt;,
    pub working_dir: Option&lt;PathBuf&gt;,
    pub timeout: Option&lt;Duration&gt;,
    pub input: Option&lt;Vec&lt;u8&gt;&gt;,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: Vec&lt;u8&gt;,
    pub stderr: Vec&lt;u8&gt;,
    pub duration: Duration,
    pub signal: Option&lt;i32&gt;,
}

// File transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub direction: TransferDirection,
    pub mode: Option&lt;u32&gt;,
    pub preserve_permissions: bool,
    pub preserve_timestamps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferDirection {
    Upload,   // Local to remote
    Download, // Remote to local
}
```

### Connection Manager API

```rust
pub struct ConnectionManager {
    connections: Arc&lt;RwLock&lt;HashMap&lt;String, Connection&gt;&gt;&gt;,
    connection_pool: Arc&lt;ConnectionPool&gt;,
    config: ConnectionManagerConfig,
}

impl ConnectionManager {
    pub fn new(config: ConnectionManagerConfig) -&gt; Self;
    
    pub async fn connect(&amp;self, config: ConnectionConfig) -&gt; Result&lt;String, ConnectError&gt;;
    pub async fn disconnect(&amp;self, connection_id: &amp;str) -&gt; Result&lt;(), ConnectError&gt;;
    pub async fn get_connection(&amp;self, connection_id: &amp;str) -&gt; Option&lt;Connection&gt;;
    pub async fn list_connections(&amp;self) -&gt; Vec&lt;Connection&gt;;
    
    pub async fn execute_command(
        &amp;self,
        connection_id: &amp;str,
        command: RemoteCommand,
    ) -&gt; Result&lt;CommandResult, ConnectError&gt;;
    
    pub async fn transfer_file(
        &amp;self,
        connection_id: &amp;str,
        transfer: FileTransfer,
    ) -&gt; Result&lt;(), ConnectError&gt;;
    
    pub async fn health_check(&amp;self, connection_id: &amp;str) -&gt; Result&lt;bool, ConnectError&gt;;
    pub async fn cleanup_stale_connections(&amp;self) -&gt; Result&lt;u32, ConnectError&gt;;
}

// Daemon service interface
pub struct ConnectionDaemon {
    manager: ConnectionManager,
    socket_path: PathBuf,
    running: Arc&lt;AtomicBool&gt;,
}

impl ConnectionDaemon {
    pub fn new(socket_path: PathBuf, config: ConnectionManagerConfig) -&gt; Self;
    
    pub async fn start(&amp;self) -&gt; Result&lt;(), ConnectError&gt;;
    pub async fn stop(&amp;self) -&gt; Result&lt;(), ConnectError&gt;;
    pub async fn handle_client(&amp;self, stream: UnixStream) -&gt; Result&lt;(), ConnectError&gt;;
}
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Authentication failed for {host}: {reason}")]
    AuthenticationFailed { host: String, reason: String },
    
    #[error("Connection timeout to {host}:{port} after {timeout}s")]
    ConnectionTimeout { host: String, port: u16, timeout: u64 },
    
    #[error("Host {host} is unreachable: {reason}")]
    HostUnreachable { host: String, reason: String },
    
    #[error("SSH key error: {message}")]
    KeyError { message: String },
    
    #[error("Connection {connection_id} not found")]
    ConnectionNotFound { connection_id: String },
    
    #[error("Maximum connections ({max}) exceeded")]
    MaxConnectionsExceeded { max: u32 },
    
    #[error("Command execution failed: {command}, exit_code: {exit_code}")]
    CommandFailed { command: String, exit_code: i32 },
    
    #[error("File transfer failed: {source} -&gt; {destination}: {reason}")]
    TransferFailed { source: String, destination: String, reason: String },
    
    #[error("SSH multiplexing error: {message}")]
    MultiplexingError { message: String },
    
    #[error("Daemon communication error: {message}")]
    DaemonError { message: String },
}
```

## File and Package Structure

```
src/bin/rustle-connect.rs       # Main binary entry point
src/connect/
├── mod.rs                      # Module exports
├── manager.rs                  # Connection manager
├── daemon.rs                   # Daemon service
├── pool.rs                     # Connection pooling
├── auth.rs                     # Authentication handling
├── multiplex.rs                # SSH multiplexing
├── command.rs                  # Command execution
├── transfer.rs                 # File transfer
├── health.rs                   # Health monitoring
├── config.rs                   # Configuration
└── error.rs                    # Error types

src/types/
├── connection.rs               # Connection data structures
└── protocol.rs                 # Communication protocol

tests/connect/
├── manager_tests.rs
├── daemon_tests.rs
├── auth_tests.rs
├── command_tests.rs
├── transfer_tests.rs
└── integration_tests.rs
```

## Implementation Details

### Phase 1: Basic Connection Management
1. Implement core SSH connection establishment
2. Add basic authentication (key, password)
3. Create connection manager with pooling
4. Add command execution capabilities

### Phase 2: Multiplexing and Optimization
1. Implement SSH multiplexing support
2. Add connection health monitoring
3. Create connection reuse and lifecycle management
4. Add performance optimizations

### Phase 3: Daemon Mode and IPC
1. Implement daemon service mode
2. Add Unix socket communication
3. Create client library for other tools
4. Add daemon management commands

### Phase 4: Advanced Features
1. Add file transfer capabilities
2. Implement tunneling and port forwarding
3. Add advanced authentication methods
4. Create monitoring and metrics

### Key Algorithms

**Connection Multiplexing**:
```rust
async fn establish_multiplexed_connection(
    config: &amp;ConnectionConfig
) -&gt; Result&lt;MultiplexedConnection, ConnectError&gt; {
    let control_path = format!(
        "~/.rustle/ssh-{}-{}@{}:{}",
        rand::random::&lt;u32&gt;(),
        config.user,
        config.host,
        config.port
    );
    
    // Establish master connection
    let master_session = Session::new()?;
    master_session.set_tcp_stream(
        TcpStream::connect((config.host.as_str(), config.port)).await?
    );
    
    master_session.handshake()?;
    authenticate_session(&amp;master_session, &amp;config.auth_method)?;
    
    // Set up control socket
    let channel = master_session.channel_session()?;
    channel.exec(&amp;format!(
        "ssh -o ControlMaster=yes -o ControlPath={} -o ControlPersist=600 {}@{} sleep 600",
        control_path, config.user, config.host
    ))?;
    
    Ok(MultiplexedConnection {
        master_session,
        control_path,
        config: config.clone(),
        channels: Arc::new(RwLock::new(Vec::new())),
    })
}

async fn get_or_create_channel(
    connection: &amp;MultiplexedConnection
) -&gt; Result&lt;Channel, ConnectError&gt; {
    // Try to reuse existing channel
    {
        let channels = connection.channels.read().await;
        for channel in channels.iter() {
            if channel.is_available() {
                return Ok(channel.clone());
            }
        }
    }
    
    // Create new channel through multiplexed connection
    let channel = connection.master_session.channel_session()?;
    
    let mut channels = connection.channels.write().await;
    channels.push(channel.clone());
    
    Ok(channel)
}
```

**Connection Health Monitoring**:
```rust
async fn monitor_connection_health(
    connection_id: String,
    manager: Arc&lt;ConnectionManager&gt;
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        let health_check_result = manager
            .execute_command(
                &amp;connection_id,
                RemoteCommand {
                    command: "echo".to_string(),
                    args: vec!["rustle-health-check".to_string()],
                    env: HashMap::new(),
                    working_dir: None,
                    timeout: Some(Duration::from_secs(5)),
                    input: None,
                }
            )
            .await;
        
        match health_check_result {
            Ok(result) if result.exit_code == 0 =&gt; {
                // Connection healthy, update last_used timestamp
                manager.update_connection_health(&amp;connection_id, true).await;
            }
            _ =&gt; {
                // Connection unhealthy, attempt reconnection
                warn!("Connection {} failed health check, attempting reconnection", connection_id);
                if let Err(e) = manager.reconnect(&amp;connection_id).await {
                    error!("Failed to reconnect {}: {}", connection_id, e);
                    manager.mark_connection_failed(&amp;connection_id, e.to_string()).await;
                }
            }
        }
    }
}
```

**Authentication Handling**:
```rust
fn authenticate_session(
    session: &amp;Session, 
    auth_method: &amp;AuthMethod
) -&gt; Result&lt;(), ConnectError&gt; {
    match auth_method {
        AuthMethod::Key { private_key_path, passphrase } =&gt; {
            let public_key_path = private_key_path.with_extension("pub");
            session.userauth_pubkey_file(
                None,
                Some(&amp;public_key_path),
                private_key_path,
                passphrase.as_deref(),
            )?;
        }
        
        AuthMethod::Password { password } =&gt; {
            session.userauth_password(None, password)?;
        }
        
        AuthMethod::Agent =&gt; {
            let mut agent = session.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            
            for identity in agent.identities()? {
                if agent.userauth(None, &amp;identity).is_ok() {
                    return Ok(());
                }
            }
            
            return Err(ConnectError::AuthenticationFailed {
                host: "unknown".to_string(),
                reason: "No suitable agent identity found".to_string(),
            });
        }
        
        AuthMethod::Interactive =&gt; {
            // Interactive authentication would require terminal input
            // In daemon mode, this should fail gracefully
            return Err(ConnectError::AuthenticationFailed {
                host: "unknown".to_string(),
                reason: "Interactive authentication not supported in daemon mode".to_string(),
            });
        }
    }
    
    if !session.authenticated() {
        return Err(ConnectError::AuthenticationFailed {
            host: "unknown".to_string(),
            reason: "Authentication completed but session not authenticated".to_string(),
        });
    }
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests
- **Connection establishment**: Test various authentication methods
- **Multiplexing**: Test connection reuse and channel management
- **Command execution**: Test command running and result handling
- **Error handling**: Test various failure scenarios
- **Health monitoring**: Test connection health checks

### Integration Tests
- **Multi-host connections**: Test connecting to multiple hosts simultaneously
- **Daemon mode**: Test daemon startup, client connections, and shutdown
- **File transfers**: Test upload/download operations
- **Long-running connections**: Test connection persistence and recovery
- **Network failures**: Test behavior during network interruptions

### Test Infrastructure
```
tests/fixtures/
├── keys/
│   ├── test_rsa              # Test SSH keys
│   ├── test_rsa.pub
│   ├── test_ed25519
│   └── test_ed25519.pub
├── configs/
│   ├── ssh_config            # Test SSH configurations
│   └── daemon_config.toml
└── scripts/
    ├── setup_test_hosts.sh   # Test environment setup
    └── cleanup_test_hosts.sh
```

### Mock Requirements
- SSH server mocking for unit tests
- Network failure simulation
- Authentication failure scenarios
- File system mocking for key operations

## Edge Cases &amp; Error Handling

### Network Conditions
- Network timeouts and interruptions
- DNS resolution failures
- Port accessibility issues
- Firewall blocking connections

### Authentication Issues
- Invalid or expired SSH keys
- Password authentication failures
- SSH agent unavailability
- Permission denied scenarios

### Resource Management
- Maximum connection limits
- Memory usage with many connections
- File descriptor limits
- Connection cleanup on crashes

### SSH-specific Issues
- Host key verification failures
- Unsupported SSH server versions
- Configuration file conflicts
- Control socket permission issues

## Dependencies

### External Crates
```toml
[dependencies]
openssh = "0.10"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
shellexpand = "3"
rand = "0.8"
```

### Internal Dependencies
- `rustle::types` - Core type definitions
- `rustle::error` - Error handling
- `rustle::config` - Configuration management

## Configuration

### Environment Variables
- `RUSTLE_SSH_CONFIG`: SSH configuration file path
- `RUSTLE_CONNECT_SOCKET`: Daemon socket path
- `RUSTLE_SSH_KEY_PATH`: Default SSH private key path
- `RUSTLE_MAX_CONNECTIONS`: Maximum concurrent connections
- `RUSTLE_CONNECTION_TIMEOUT`: Default connection timeout

### Configuration File Support
```toml
[connection]
max_connections = 1000
default_timeout_secs = 10
keepalive_interval_secs = 30
max_retries = 3
enable_multiplexing = true
enable_compression = false

[daemon]
socket_path = "~/.rustle/connect.sock"
enable_daemon = false
cleanup_interval_secs = 300

[ssh]
config_file = "~/.ssh/config"
known_hosts_file = "~/.ssh/known_hosts"
control_path = "~/.rustle/ssh-%h-%p-%r"
control_persist_secs = 600

[auth]
default_key_path = "~/.ssh/id_rsa"
prefer_agent = true
agent_timeout_secs = 10
```

## Documentation

### CLI Help Text
```
rustle-connect - SSH connection manager for Rustle ecosystem

USAGE:
    rustle-connect &lt;SUBCOMMAND&gt;

SUBCOMMANDS:
    daemon              Run connection daemon
    connect             Establish connection to host
    disconnect          Close connection to host
    list-connections    List active connections
    test-connection     Test connection to host
    tunnel              Create SSH tunnel
    forward             Set up port forwarding
    help                Print this message or the help of the given subcommand(s)

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

EXAMPLES:
    rustle-connect daemon                                    # Start daemon
    rustle-connect connect user@host                        # Connect to host
    rustle-connect connect -i ~/.ssh/key user@host         # Connect with specific key
    rustle-connect list-connections                         # List active connections
    rustle-connect tunnel user@host 8080:localhost:80      # Create tunnel
```

### API Documentation
Comprehensive rustdoc documentation covering:
- Connection management patterns
- Authentication methods and security
- Performance optimization techniques
- Daemon integration guidelines

### Integration Examples
```bash
# Start daemon for connection management
rustle-connect daemon &amp;

# Other tools can now use connections
rustle-exec --connection-daemon ~/.rustle/connect.sock plan.json

# Direct connection management
CONNECTION_ID=$(rustle-connect connect user@host)
rustle-exec --connection $CONNECTION_ID single-task.json
rustle-connect disconnect $CONNECTION_ID

# Batch connection establishment
rustle-connect connect user@host1 user@host2 user@host3
rustle-exec --use-daemon plan.json
```