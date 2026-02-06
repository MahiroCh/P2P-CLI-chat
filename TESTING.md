# Quick Start Testing Guide

Follow these steps to test the P2P Chat MVP on your two Linux devices.

## Preparation

1. Build the project on both devices:
   ```bash
   cargo build --release
   ```

2. The binaries will be at:
   - `target/release/chat-daemon`
   - `target/release/chat-cli`

## Testing on Local Network (2 Linux Devices)

### Device 1

**Terminal 1 - Start Daemon:**
```bash
./target/release/chat-daemon
```

You'll see output like:
```
Starting P2P Chat Daemon
Socket listener bound to /tmp/chat_daemon.sock
Iroh node started: <node_id_1>
Endpoint address: EndpointAddr { id: <node_id_1>, addrs: [<IP:port>] }
Local endpoints: [<IP:port>]
Listening for CLI connections...
```

**Terminal 2 - Get Status (keep this running to see incoming messages):**
```bash
./target/release/chat-cli connect
```

This enters interactive mode. Type `status` to see your endpoint info:
```
> status

=== Daemon Status ===
Node ID: <node_id_1>
Local Endpoints:
  <IP:port>
Connected Peers: 0

>
```

**Copy the full endpoint address** - you'll need it on Device 2.

### Device 2

**Terminal 1 - Start Daemon:**
```bash
./target/release/chat-daemon
```

**Terminal 2 - Connect to Device 1:**
```bash
./target/release/chat-cli connect
```

**Join Device 1 (paste the endpoint address from Device 1):**
```
> join "<endpoint_from_device_1>"
✓ Connected to peer: <node_id_1>

>
```

**Send a test message:**
```
> send <node_id_1> "Hello from Device 2!"
✓ Message sent to <node_id_1>

>
```

### Verify on Device 1

Check Device 1's interactive CLI (Terminal 2) - you should see:
```
> 
✓ Message sent to <node_id_2>

✓ Connected Peers: 1
  <node_id_2>
```

## Testing Features

### 1. Status Check
```bash
./target/release/chat-cli status
```

Shows:
- Your node ID
- Local endpoints (IP:port)
- Connected peers list

### 2. List Peers
```bash
./target/release/chat-cli peers
```

### 3. Join (Connect to Peer)
```bash
./target/release/chat-cli join "<endpoint>"
```

Where `<endpoint>` is the full address from another device's status.

### 4. Send Message
```bash
./target/release/chat-cli send <peer_node_id> "Your message here"
```

## Troubleshooting

**Issue: "Failed to connect to daemon"**
- Make sure daemon is running on that device
- Check that `/tmp/chat_daemon.sock` exists

**Issue: "Connection failed"**
- Verify both devices are on the same local network
- Check firewall isn't blocking UDP/QUIC (Iroh uses QUIC)
- Verify the endpoint address format is correct

**Issue: Message not appearing on remote**
- CLI must be in interactive mode (`chat-cli connect`) to receive messages
- Check both nodes are connected (use `status` command)

## Expected Behavior

- Both daemons start independently with unique node IDs
- Devices on same network can discover each other via Iroh's relay
- Messages sent show immediate confirmation
- Long-running CLI processes receive real-time message notifications
- Stopping daemon stops all connections gracefully
