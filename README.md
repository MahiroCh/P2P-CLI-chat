# Sendout

A minimal decentralized peer-to-peer chat application for Linux, written in Rust. It features a daemon-based architecture with an interactive REPL client for real-time messaging between peers.

## Overview

**Sendout** solves the problem of establishing direct, peer-to-peer communication without relying on centralized servers. The project demonstrates:

- **What**: A working P2P messaging system that allows users to send messages directly to peers using ticket-based connection sharing
- **Why**: To enable decentralized communication with full user control, privacy, and the ability to run without coordinating server after peers are connected
- **Architecture**: Separates concerns into a background daemon (network management) and a foreground CLI client (user interaction), communicating via Unix socket

The application uses Iroh's robust connection and relay mechanisms, supporting both direct connections and mediated relays.

## Features

- **Peer-to-Peer Messaging**: Send and receive messages directly between peers with end-to-end connectivity via Iroh
- **Interactive REPL Terminal**: User-friendly command-line interface for real-time chat interactions
- **Daemon Architecture**: Persistent background service that maintains peer connections while the CLI can reconnect
- **Connection Management**: Connect to peers using shareable tickets, disconnect cleanly, and list active connections
- **Relay Support**: Automatic relay fallback for peers behind firewalls or specific NAT
- **Flexible Logging**: Configurable logging levels for both daemon and CLI with XDG-compliant log directories
- **Process Management**: Built-in daemon control (start, stop, status) with proper cleanup
- **Single Binary**: Both daemon and CLI functionality in one executable for easy deployment

## Requirements

### System Requirements

- **OS**: Linux
- **Rust**: 1.79 or later (edition 2021)
- **Build Tools**: `cargo` build system or bare `rustc` compiler

### Runtime Dependencies

- Tokio async runtime for asynchronous I/O
- Iroh 0.98.1 for P2P networking and relay support
- No external database or separate backend service required

## Installation

### Building from Source

```bash
git clone https://github.com/MahiroCh/Sendout.git
cd p2p-chat
cargo build --release
```

The compiled binary will be located at `target/release/p2pchat`.

### Optional: System-wide Installation

```bash
# Copy to system bin directory
sudo cp target/release/p2pchat /usr/local/bin/

# Or add the project directory to your PATH
export PATH="$PATH:$(pwd)/target/release"
```

## Usage

To see help section for commands, use `help`, `-h`, `--help` (can be combined 
with commands to get help about specifics).

### 1. Starting the Daemon

Before using the chat, you must start the daemon (background service):

```bash
p2pchat daemon start
```

Optional: Set custom log level for daemon

```bash
p2pchat daemon start --log-level debug
```

Verify daemon status:

```bash
p2pchat daemon status
```

After you end chatting, stop the daemon:

```bash
p2pchat daemon stop
```

### 2. Starting Interactive Session

Launch the interactive chat client:

```bash
p2pchat client interactive
```

You'll see a prompt where you can enter commands.

### Interactive Commands

Once in interactive mode, use these commands:

#### Connection & Peer Management

```bash
# Display your shareable ticket (send this to peers to connect to you)
myid

# Connect to a peer using their ticket
connect <peer-ticket>

# Disconnect from a peer
disconnect <peer-id>

# List all currently connected peers
list
```

#### Messaging

```bash
# Send a message to a connected peer
send <peer-id> <message>
```

#### Session Control

```bash
# Exit interactive mode (daemon keeps running in background)
quit
```

### Quick Start Example

**Terminal 1 - User A starts daemon and shares ticket:**

```bash
$ p2pchat daemon start
[daemon started successfully]

$ p2pchat client interactive
> myid
my endpoint ID: [long ticket]
[info about ticket and tip]
[Share this ticket with User B]
```

**Terminal 2 - User B connects and sends a message:**

```bash
$ p2pchat daemon start
$ p2pchat client interactive
> connect [User A's ticket]
ok: connected to 7d6f4a2b1c8e9f3d

> send 7d6f4a2b1c8e9f3d "Hello from User B!"
ok: sent to 7d6f4a2b1c8e9f3d at 23:24:01

> list
Connected peers: 
  - 7d6f4a2b1c8e9f3d
```

**Back to Terminal 1 - User A receives and responds:**

```bash
*** peer connected: 3a1b4c5d7e8f9g2h
[23:24:05] <3a1b4c5d7e8f9g2h> Hello from User B!

> send 3a1b4c5d7e8f9g2h "Hi there! Nice to connect with you."
ok: sent to 3a1b4c5d7e8f9g2h at 23:26:01

> quit
Quitting interactive mode...
```

### Logging Configuration

#### Client Logging

Set client-side log level with the global `--cli-log-level` option:

```bash
p2pchat --cli-log-level debug client interactive
```

#### Daemon Logging

Set daemon log level with the `--log-level` option:

```bash
p2pchat daemon start --log-level debug
```

Or use the environment variable (useful when starting daemon in background):

```bash
P2PCHAT_DAEMON_LOG_LEVEL=debug p2pchat daemon start
```

Valid log levels: `error`, `warn`, `info`, `debug`.

#### Log File Locations

Logs are stored in XDG-compliant directories for easy management:

- **Daemon logs**: `$XDG_STATE_HOME/p2pchat/daemon.log` (fallback: `~/.local/state/p2pchat/daemon.log`)
- **CLI logs**: `$XDG_STATE_HOME/p2pchat/cli.log` (fallback: `~/.local/state/p2pchat/cli.log`)
- **Iroh network logs**: `$XDG_STATE_HOME/p2pchat/daemon-iroh.log` (fallback: `~/.local/state/p2pchat/daemon-iroh.log`)

#### Runtime Files

Socket and PID files for IPC and process management:

- **Socket**: `$XDG_RUNTIME_DIR/p2pchat/daemon.sock` (fallback: `~/.cache/p2pchat/daemon.sock`)
- **PID file**: `$XDG_RUNTIME_DIR/p2pchat/daemon.pid` (fallback: `~/.cache/p2pchat/daemon.pid`)

## License

This project is licensed under the GNU General Public License v3. 
See the [LICENSE](LICENSE) file for details.

## Authors

MahiroCh (Georgii Kachanov)

## Project Roadmap

- [ ] Improve formatting and style of output
- [ ] Implement delivery status tracking and retransmission for lost messages
- [ ] Enhance reconnection logic and state recovery after network interruptions
- [ ] Store chat transcripts locally for offline review
- [ ] Maintain a persistent contact list with aliases and favorite peers
- [ ] Develop a web dashboard or native GUI/TUI client
- [ ] Add message authentication and identity verification
- [ ] Support multi-peer conversations and group chat management
- [ ] Enable direct file sharing between peers with progress tracking
- [ ] Display online/offline status of connected peers
- [ ] Create .deb and probably other packages
- [ ] Improve cleanup on shutdown logic and handle stale socket/PID files gracefully
