# SPEC_CLIENT.md
Core Client Library Specification — Termion

## Overview

The `termion-client` module provides the gRPC client layer that wraps MinKNOW APIs. It handles connection management, service discovery, streaming, and error mapping. Both TUI and CLI consume this library.

---

## Responsibilities

### What it DOES:
- Manages gRPC connections to MinKNOW manager and position services
- Handles endpoint discovery (manager → devices → positions)
- Provides typed wrapper APIs over raw gRPC stubs
- Implements streaming primitives with backpressure handling
- Exposes a unified error type for all MinKNOW interactions
- Manages reconnection with exponential backoff

### What it does NOT:
- Render UI or format output
- Parse CLI arguments
- Handle user input or keybindings
- Implement business logic beyond API abstraction

---

## Connection Management

### Discovery Flow

```
1. Connect to Manager endpoint (localhost:9501 default)
2. Call manager.list_devices() → Vec<Device>
3. For each device, call manager.list_positions() → Vec<Position>
4. Each position has its own gRPC endpoint for data services
5. Connect to position-specific services as needed
```

### Connection Configuration

```rust
pub struct ConnectionConfig {
    /// Manager host (default: "localhost")
    pub host: String,
    
    /// Manager port (default: 9501)
    pub port: u16,
    
    /// Connection timeout
    pub connect_timeout: Duration,  // default: 5s
    
    /// Request timeout
    pub request_timeout: Duration,  // default: 30s
}
```

### Reconnection Policy

Per decision D5.3:
- **Strategy:** Exponential backoff
- **Initial delay:** 1 second
- **Max delay:** 30 seconds
- **Multiplier:** 2x
- **Jitter:** ±10% (avoid thundering herd)

```rust
pub struct ReconnectPolicy {
    pub initial_delay: Duration,   // 1s
    pub max_delay: Duration,       // 30s
    pub multiplier: f64,           // 2.0
    pub jitter_fraction: f64,      // 0.1
}
```

---

## Service APIs

### Manager Service

```rust
impl ManagerClient {
    /// List all connected devices
    pub async fn list_devices(&self) -> Result<Vec<Device>>;
    
    /// List positions for a device
    pub async fn list_positions(&self, device_id: &str) -> Result<Vec<Position>>;
    
    /// Get connection info for a position's services
    pub async fn get_position_services(&self, position: &Position) -> Result<PositionServices>;
}
```

### Acquisition Service (per-position)

```rust
impl AcquisitionClient {
    /// Get current run state
    pub async fn get_run_state(&self) -> Result<RunState>;
    
    /// Stop current acquisition
    pub async fn stop(&self) -> Result<()>;
    
    /// Pause current acquisition
    pub async fn pause(&self) -> Result<()>;
    
    /// Resume paused acquisition
    pub async fn resume(&self) -> Result<()>;
    
    /// Subscribe to run state changes
    pub async fn watch_run_state(&self) -> Result<impl Stream<Item = Result<RunState>>>;
}
```

### Statistics Service (per-position)

```rust
impl StatisticsClient {
    /// Get current statistics snapshot
    pub async fn get_stats(&self) -> Result<StatsSnapshot>;
    
    /// Subscribe to statistics stream
    pub async fn watch_stats(&self) -> Result<impl Stream<Item = Result<StatsSnapshot>>>;
}
```

---

## Domain Types

Separate from proto-generated types to provide stable API:

```rust
pub struct Device {
    pub id: String,
    pub name: String,
    pub state: DeviceState,
}

pub struct Position {
    pub id: String,
    pub name: String,
    pub device_id: String,
    pub state: PositionState,
}

pub enum RunState {
    Idle,
    Starting,
    Running,
    Paused,
    Finishing,
    Stopped,
    Error(String),
}

pub struct StatsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub reads_processed: u64,
    pub bases_called: u64,
    pub throughput_bps: f64,
    // ... additional fields per MinKNOW stats
}
```

---

## Streaming & Backpressure

Per decision D5.2: Drop stale frames if UI can't keep up.

### Channel Strategy

```rust
// Bounded channel with drop-oldest semantics
pub struct StreamBuffer<T> {
    capacity: usize,  // e.g., 16 frames
    // When full, drop oldest before inserting new
}
```

### Stream Helpers

```rust
impl<T> StreamBuffer<T> {
    /// Get latest value (non-blocking)
    pub fn latest(&self) -> Option<T>;
    
    /// Async receive (blocks until available)
    pub async fn recv(&mut self) -> Option<T>;
    
    /// Check if producer has disconnected
    pub fn is_closed(&self) -> bool;
}
```

---

## Error Handling

Unified error type for all client operations:

```rust
pub enum ClientError {
    /// Failed to connect to endpoint
    Connection { endpoint: String, source: tonic::transport::Error },
    
    /// gRPC call failed
    Grpc { method: String, status: tonic::Status },
    
    /// Response parsing/validation failed
    Protocol { message: String },
    
    /// Resource not found (device, position, etc.)
    NotFound { resource: String, id: String },
    
    /// Operation timed out
    Timeout { operation: String },
    
    /// Connection lost, attempting reconnect
    Disconnected,
}

impl ClientError {
    /// Is this error recoverable via retry?
    pub fn is_retriable(&self) -> bool;
    
    /// Human-readable error message
    pub fn display_message(&self) -> String;
}
```

---

## Testing

Per decision D6.2: Mock gRPC server for integration tests.

### Mock Server

```rust
pub struct MockMinKnowServer {
    devices: Vec<Device>,
    positions: HashMap<String, Vec<Position>>,
    run_states: HashMap<String, RunState>,
}

impl MockMinKnowServer {
    pub fn new() -> Self;
    pub fn with_devices(devices: Vec<Device>) -> Self;
    pub fn set_run_state(&mut self, position_id: &str, state: RunState);
    pub async fn start(&self) -> SocketAddr;
}
```

### Test Patterns

```rust
#[tokio::test]
async fn test_list_devices() {
    let server = MockMinKnowServer::new()
        .with_devices(vec![mock_device("dev1")]);
    let addr = server.start().await;
    
    let client = ManagerClient::connect(&format!("http://{}", addr)).await?;
    let devices = client.list_devices().await?;
    
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "dev1");
}
```

---

## Dependencies

- `tonic` — gRPC client
- `prost` — Protocol buffer codegen
- `tokio` — Async runtime
- `tracing` — Instrumentation
