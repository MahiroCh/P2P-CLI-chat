# Description

Minimal P2P-chat for Linux written in Rust and based on Iroh library.

## Build

```bash
cargo build --release
```

## Usage

**Start daemon:**
```bash
./target/release/chat-daemon
```

**Connect to peer:**
```bash
./target/release/chat-cli connect <peer_node_id>
```

**Send message:**
```bash
./target/release/chat-cli send <peer_id> <message>
```

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
