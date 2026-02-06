# P2P Chat - Decentralized CLI

Decentralized P2P-chat Linux CLI written in Rust and based on Iroh library (https://docs.iroh.computer).

A minimal MVP implementation featuring a daemon-based architecture with Unix socket IPC for inter-process communication. Enables secure, encrypted peer-to-peer messaging on local networks with relay server support for future internet connectivity.

## Architecture

- **Daemon** (`chat-daemon`): Runs the Iroh p2p node, accepts client connections via Unix socket, manages peer connections and in-memory message storage
- **CLI** (`chat-cli`): Command-line interface to interact with the daemon via IPC (Unix domain sockets), supports both single commands and interactive mode

## Building

### Requirements

- Rust 1.70+
- Linux (Unix domain sockets)

### Build Release Binaries

```bash
cargo build --release
```

Binaries will be available at:
- `target/release/chat-daemon` - The p2p node daemon
- `target/release/chat-cli` - The CLI client

## Running

### Start the Daemon

```bash
./target/release/chat-daemon
```

The daemon will:
- Initialize the Iroh endpoint
- Display its node ID and endpoint address
- Start listening for CLI connections on `/tmp/chat_daemon.sock`

Example output:
```
Starting P2P Chat Daemon
Socket listener bound to /tmp/chat_daemon.sock
Iroh node started: <node_id>
Endpoint address: EndpointAddr { id: <node_id>, addrs: [...] }
Local endpoints: [...IP addresses...]
Listening for CLI connections...
```

### Connect CLI Client

#### Interactive Mode

```bash
./target/release/chat-cli connect
```

Then use commands:
- `help` - Show available commands
- `status` - Display daemon status and endpoint info
- `peers` - List all connected peers
- `join <endpoint>` - Connect to a remote peer (requires endpoint address)
- `send <peer_id> <message>` - Send a message to a peer
- `quit` - Disconnect and exit

#### Single Commands

```bash
# Get daemon status
./target/release/chat-cli status

# List peers
./target/release/chat-cli peers

# Join a peer
./target/release/chat-cli join "<peer_endpoint_address>"

# Send message
./target/release/chat-cli send <peer_id> "Hello, peer!"
```

## Usage Example (Local Network)

### Device 1

```bash
# Terminal 1: Start daemon
./target/release/chat-daemon

# In another terminal: Get your endpoint address
./target/release/chat-cli status
# Output shows your node ID and endpoint address
```

### Device 2

```bash
# Terminal 1: Start daemon
./target/release/chat-daemon

# Terminal 2: Connect to Device 1
./target/release/chat-cli join "<device1_endpoint_address>"

# Terminal 2: Send message
./target/release/chat-cli send <device1_node_id> "Hello from Device 2!"
```

## Protocol

### IPC Messages (JSON)

All communication between CLI and daemon uses JSON over Unix sockets.

**Commands** (CLI → Daemon):
```json
{
  "Command": {
    "Status": null
  }
}
```

**Responses** (Daemon → CLI):
```json
{
  "Response": {
    "Status": {
      "node_id": "...",
      "local_endpoints": ["..."],
      "peers": [...]
    }
  }
}
```

### Message Format

Chat messages include:
- Sender ID (node ID)
- Timestamp (Unix seconds)
- Content (text)

## Design Notes

### Future-Proof for Internet Connectivity

The implementation leverages Iroh's relay server support. Currently working on local networks (mDNS discovery), it automatically scales to internet connectivity when peers use relay endpoints.

### In-Memory Storage

MVP uses in-memory `HashMap` for message storage per peer. Ready for migration to SQLite while maintaining the same interface.

### Socket Communication

Uses Unix domain sockets at `/tmp/chat_daemon.sock` for local IPC. Hardcoded for MVP simplicity.

## Implementation Notes

- **Async Runtime:** Tokio for async I/O
- **Serialization:** serde/serde_json for message protocols
- **CLI Parsing:** clap for command-line interface
- **Logging:** tracing/tracing-subscriber
- **Networking:** Iroh 0.96 (latest stable)

## Limitations (MVP)

1. No message persistence beyond current session
2. Manual peer endpoint sharing required (no automatic discovery via DNS)
3. Flat peer list (no grouping/channels)
4. No user authentication or authorization
5. All messages stored in memory (no history on reconnect)
6. Single daemon instance per system

## Future Enhancements

- [ ] Message history persistence (SQLite)
- [ ] DNS-based address lookup for easier peer discovery
- [ ] Group chat support
- [ ] User authentication
- [ ] Message encryption at application level
- [ ] Web UI frontend
- [ ] Mobile client support
- [ ] Multi-daemon federation

## References

- [Iroh Documentation](https://docs.iroh.computer)
- [Iroh API Docs](https://docs.rs/iroh)
- [Software Requirements Specification](.github/Copilot/Software%20Requirements%20Specification.md)