# MCP SSH Sessions

A [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server that provides SSH session management tools for AI assistants.

## Purpose

This MCP server was built with a specific goal: **persistent SSH sessions with working sudo caching and X11 askpass forwarding**. 

Why does this matter? Most existing solutions create new SSH connections for every command, which means:
- You have to enter your sudo password over and over again every few seconds
- Sudo's credential caching becomes useless since each command runs in a fresh session
- Complex workflows involving multiple sudo operations become painfully repetitive

By maintaining persistent SSH sessions, this server lets sudo caching work as intended - enter your password once, and subsequent sudo commands in the same session just work without re-authentication (within sudo's timeout window).

**Important:** If your server can't have `ssh-askpass` installed or you can't forward X11, use another MCP server instead.

## What it does

- **Connect to SSH hosts** and maintain persistent sessions
- **Execute commands** remotely with output capture
- **Support sudo operations** via X11 forwarding (when `ssh-askpass` is available)
- **Manage multiple sessions** with unique identifiers
- **Handle basic shell commands** including pipes and redirects

## What it doesn't do (yet)

- **Enterprise-grade security** - this works great for development but isn't hardened for critical environments
- **Connection authentication** - relies entirely on your existing SSH key setup
- **Advanced terminal features** - no PTY allocation, terminal resizing, or interactive programs
- **Session recovery** - if the MCP server crashes, SSH sessions are lost
- **Robust error handling** - network issues may require manual cleanup
- **Cross-platform support** - primarily tested on Linux, limited Windows compatibility

## How it works

The server uploads a bash relay script to remote hosts, then communicates through that script to execute commands. It's a simple approach that works for basic use cases but has limitations.

## Requirements

- Rust (for building)
- SSH client with SCP support (`ssh`, `scp` commands)
- `ssh-askpass` (for sudo GUI prompts)
- Bash on remote hosts

## Installation

```bash
cargo install --git https://github.com/ChristophRauch/mcp-ssh-connections
```

or, when you checked out the source locally

```bash
cargo install --path .
```

## MCP Tools Available

- `ssh_connect` - Connect to an SSH host
- `ssh_execute` - Execute commands on connected sessions  
- `ssh_disconnect` - Close SSH sessions
- `ssh_list_sessions` - List active sessions

## Testing

Basic test scripts are in the `test/` directory:

```bash
./test/test_mcp.sh              # Test MCP protocol basics
./test/test_mcp_interactive.sh  # Interactive session testing  
./test/test_automated.sh        # Automated test suite
./test/test_tools.sh           # Tool-specific tests
```

## Current Limitations

This is an early-stage project with several rough edges:

- **Basic error handling** - connection failures aren't always graceful
- **No session persistence** - server restarts lose all sessions
- **Limited command safety** - command escaping could be improved
- **No connection pooling** - each session is independent
- **Minimal logging** - debugging connection issues is difficult
- **X11 dependency** - sudo operations require GUI environment

## Contributing

This project would benefit from community contributions in several areas:

### High Priority
- **Better error handling** and connection recovery
- **Session persistence** across server restarts
- **Cross-platform support** (especially Windows)
- **Security audit** and hardening

### Medium Priority  
- **Integration testing** with various SSH configurations
- **Connection pooling** and resource management
- **Improved logging** and debugging capabilities
- **Documentation** for common use cases

### Ideas Welcome
- Terminal multiplexing support
- Non-interactive sudo alternatives
- Plugin system for custom commands
- Configuration file support

**Pull requests, issues, and feedback are welcome!** This is a community effort to make SSH accessible to AI assistants.

## Security Notes

- Works great for development and personal use
- Uses your existing SSH key configuration - no additional auth needed
- Commands execute with your normal SSH user privileges  
- Sudo operations use GUI password prompts via `ssh-askpass`
- For sensitive/production environments, audit the code first

*"It works on my machine!" - but YMMV depending on your SSH setup*

## License

MIT

---

*Implements MCP protocol version 2024-11-05*
