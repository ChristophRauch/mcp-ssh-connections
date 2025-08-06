use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio, Child, ChildStdin, ChildStdout};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use anyhow::{Result, Context, bail};

// MCP Server for SSH Sessions with bash relay

#[derive(Debug)]
struct SshSession {
    host: String,
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl SshSession {
    fn connect(host: &str) -> Result<Self> {
        eprintln!("[SSH] Connecting to {}", host);
        
        // Create bash relay script
        let relay_script = r#"#!/bin/bash

export SUDO_ASKPASS='/usr/bin/ssh-askpass'

# Check X11 forwarding
[ -n "$DISPLAY" ] && echo "X11:$DISPLAY" >&2 || echo "X11:NONE" >&2

# Command execution
run() {
    bash -c "$*" 2>&1
    echo "<<<EXIT:$?>>>"
}

# Sudo command execution  
sudo_run() {
    sudo -A bash -c "$*" 2>&1
    echo "<<<EXIT:$?>>>"
}

echo "READY" >&2

# Main loop
while IFS= read -r line; do
    eval "$line"
done
"#;
        
        // Write and upload script
        let local_script = "/tmp/mcp_ssh_relay.sh";
        std::fs::write(local_script, relay_script)
            .context("Failed to write relay script to local temp file")?;
        
        let remote_script = "/tmp/mcp_ssh_relay_remote.sh";
        eprintln!("[SSH] Uploading relay script to {}", host);
        
        let scp = Command::new("scp")
            .args(["-q", "-o", "ConnectTimeout=10", "-o", "StrictHostKeyChecking=no", 
                   local_script, &format!("{}:{}", host, remote_script)])
            .output()
            .context("Failed to execute scp command")?;
        
        if !scp.status.success() {
            let stderr = String::from_utf8_lossy(&scp.stderr);
            bail!("Failed to upload relay script to {}: {}", host, stderr);
        }
        
        // Start SSH session
        eprintln!("[SSH] Starting SSH session with {}", host);
        let mut child = Command::new("ssh")
            .args(["-Y", "-o", "ConnectTimeout=10", "-o", "StrictHostKeyChecking=no",
                   host, "bash", remote_script])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(format!("Failed to start SSH process to {}", host))?;
        
        let stdin = child.stdin.take().context("Failed to get stdin from SSH process")?;
        let stdout = child.stdout.take().context("Failed to get stdout from SSH process")?;
        let stderr = child.stderr.take().context("Failed to get stderr from SSH process")?;
        
        // Monitor stderr for ready signal
        let (tx, rx) = std::sync::mpsc::channel();
        let host_clone = host.to_string();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("[{}] {}", host_clone, line);
                    if line.contains("READY") {
                        let _ = tx.send(());
                    }
                }
            }
        });
        
        // Wait for ready signal with timeout
        eprintln!("[SSH] Waiting for relay to be ready on {}", host);
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(_) => {
                eprintln!("[SSH] Relay ready on {}", host);
            }
            Err(_) => {
                // Try to kill the child process
                let _ = child.kill();
                bail!("SSH relay failed to start on {} within 10 seconds", host);
            }
        }
        
        Ok(SshSession {
            host: host.to_string(),
            child,
            stdin,
            reader: BufReader::new(stdout),
        })
    }
    
    fn execute(&mut self, command: &str, use_sudo: bool) -> Result<(String, i32)> {
        eprintln!("[SSH] Executing on {}: {} (sudo: {})", self.host, command, use_sudo);
        
        // Escape command for safe eval in bash
        let escaped = command
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`");
        
        let request = if use_sudo {
            format!(r#"sudo_run "{}""#, escaped)
        } else {
            format!(r#"run "{}""#, escaped)
        };
        
        // Send command
        writeln!(self.stdin, "{}", request)
            .context(format!("Failed to send command to {}", self.host))?;
        self.stdin.flush()
            .context(format!("Failed to flush stdin to {}", self.host))?;
        
        // Read output with timeout
        let mut output = Vec::new();
        let mut exit_code = 0;
        let mut lines_read = 0;
        
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    eprintln!("[SSH] Unexpected EOF from {} after {} lines", self.host, lines_read);
                    break; // EOF
                }
                Ok(_) => {
                    lines_read += 1;
                    let line = line.trim_end().to_string();
                    
                    if let Some(code_str) = line.strip_prefix("<<<EXIT:") {
                        if let Some(code_str) = code_str.strip_suffix(">>>") {
                            exit_code = code_str.parse()
                                .context(format!("Invalid exit code format from {}: {}", self.host, code_str))?;
                        }
                        eprintln!("[SSH] Command completed on {} with exit code: {}", self.host, exit_code);
                        break;
                    }
                    output.push(line);
                    
                    // Safety check to prevent infinite loops
                    if lines_read > 10000 {
                        bail!("Too many lines read from {} (>10000), aborting", self.host);
                    }
                }
                Err(e) => {
                    return Err(anyhow::Error::from(e)
                        .context(format!("Error reading output from {}", self.host)));
                }
            }
        }
        
        let output_text = output.join("\n");
        eprintln!("[SSH] Got {} lines of output from {}", output.len(), self.host);
        
        Ok((output_text, exit_code))
    }
    
    fn disconnect(mut self) -> Result<()> {
        eprintln!("[SSH] Disconnecting from {}", self.host);
        
        // Close stdin to signal the relay to exit
        drop(self.stdin);
        
        // Wait for the SSH process to finish, with timeout
        match self.child.wait() {
            Ok(status) => {
                eprintln!("[SSH] SSH process to {} exited with status: {}", self.host, status);
                Ok(())
            }
            Err(e) => {
                eprintln!("[SSH] Error waiting for SSH process to {}: {}", self.host, e);
                // Try to kill the process if it's still running
                let _ = self.child.kill();
                Err(anyhow::Error::from(e).context(format!("Failed to cleanly disconnect from {}", self.host)))
            }
        }
    }
}

struct McpServer {
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
}

impl McpServer {
    fn new() -> Self {
        McpServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn handle_request(&self, request: Value) -> Result<Value> {
        let method = request["method"].as_str()
            .context("Missing method in request")?;
        
        match method {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_tool_call(&request),
            _ => bail!("Unknown method: {}", method),
        }
    }
    
    fn handle_initialize(&self) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "mcp-ssh-sessions",
                "version": "0.1.0"
            }
        }))
    }
    
    fn handle_list_tools(&self) -> Result<Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "ssh_connect",
                    "description": "Connect to an SSH host",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "host": {
                                "type": "string",
                                "description": "Hostname or IP to connect to"
                            },
                            "session_id": {
                                "type": "string",
                                "description": "Optional session ID (defaults to host)"
                            }
                        },
                        "required": ["host"]
                    }
                },
                {
                    "name": "ssh_execute",
                    "description": "Execute a command on a connected SSH session",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "session_id": {
                                "type": "string",
                                "description": "Session ID or hostname"
                            },
                            "command": {
                                "type": "string",
                                "description": "Command to execute"
                            },
                            "sudo": {
                                "type": "boolean",
                                "description": "Execute with sudo",
                                "default": false
                            }
                        },
                        "required": ["session_id", "command"]
                    }
                },
                {
                    "name": "ssh_disconnect",
                    "description": "Disconnect an SSH session",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "session_id": {
                                "type": "string",
                                "description": "Session ID or hostname"
                            }
                        },
                        "required": ["session_id"]
                    }
                },
                {
                    "name": "ssh_list_sessions",
                    "description": "List all active SSH sessions",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        }))
    }
    
    fn handle_tool_call(&self, request: &Value) -> Result<Value> {
        let tool_name = request["params"]["name"].as_str()
            .context("Missing tool name")?;
        let arguments = &request["params"]["arguments"];
        
        match tool_name {
            "ssh_connect" => {
                let host = arguments["host"].as_str()
                    .context("Missing or invalid host parameter - must be a string")?;
                
                if host.is_empty() {
                    bail!("Host parameter cannot be empty");
                }
                
                let session_id = arguments["session_id"].as_str()
                    .unwrap_or(host);
                
                eprintln!("[MCP] Attempting to connect to {} with session ID: {}", host, session_id);
                
                // Check if session already exists
                {
                    let sessions = self.sessions.lock().unwrap();
                    if sessions.contains_key(session_id) {
                        bail!("Session '{}' already exists. Use ssh_disconnect first or choose a different session_id.", session_id);
                    }
                }
                
                let session = SshSession::connect(host)
                    .context(format!("Failed to establish SSH connection to {}", host))?;
                
                let mut sessions = self.sessions.lock().unwrap();
                sessions.insert(session_id.to_string(), session);
                
                eprintln!("[MCP] Successfully connected to {} (session: {})", host, session_id);
                
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Successfully connected to {} (session: {})", host, session_id)
                    }]
                }))
            }
            
            "ssh_execute" => {
                let session_id = arguments["session_id"].as_str()
                    .context("Missing or invalid session_id parameter - must be a string")?;
                let command = arguments["command"].as_str()
                    .context("Missing or invalid command parameter - must be a string")?;
                let use_sudo = arguments["sudo"].as_bool().unwrap_or(false);
                
                if command.is_empty() {
                    bail!("Command parameter cannot be empty");
                }
                
                eprintln!("[MCP] Executing command on session '{}': {} (sudo: {})", session_id, command, use_sudo);
                
                let mut sessions = self.sessions.lock().unwrap();
                let session = sessions.get_mut(session_id)
                    .context(format!("No active session found with ID '{}'. Use ssh_list_sessions to see available sessions.", session_id))?;
                
                let (output, exit_code) = session.execute(command, use_sudo)
                    .context(format!("Failed to execute command on session '{}'", session_id))?;
                
                eprintln!("[MCP] Command executed on session '{}' with exit code: {}", session_id, exit_code);
                
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": output
                    }],
                    "metadata": {
                        "exit_code": exit_code,
                        "session_id": session_id,
                        "command": command,
                        "sudo": use_sudo
                    }
                }))
            }
            
            "ssh_disconnect" => {
                let session_id = arguments["session_id"].as_str()
                    .context("Missing or invalid session_id parameter - must be a string")?;
                
                eprintln!("[MCP] Attempting to disconnect session '{}'", session_id);
                
                let mut sessions = self.sessions.lock().unwrap();
                let session = sessions.remove(session_id)
                    .context(format!("No active session found with ID '{}'. Use ssh_list_sessions to see available sessions.", session_id))?;
                
                session.disconnect()
                    .context(format!("Failed to cleanly disconnect session '{}'", session_id))?;
                
                eprintln!("[MCP] Successfully disconnected session '{}'", session_id);
                
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Successfully disconnected session: {}", session_id)
                    }]
                }))
            }
            
            "ssh_list_sessions" => {
                let sessions = self.sessions.lock().unwrap();
                let session_count = sessions.len();
                let session_list: Vec<String> = sessions.keys().cloned().collect();
                
                eprintln!("[MCP] Listing {} active sessions", session_count);
                
                let response_text = if session_list.is_empty() {
                    "No active SSH sessions".to_string()
                } else {
                    format!("Active SSH sessions ({}):\n{}", session_count, 
                        session_list.iter()
                            .map(|s| format!("  - {}", s))
                            .collect::<Vec<_>>()
                            .join("\n"))
                };
                
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": response_text
                    }],
                    "metadata": {
                        "session_count": session_count,
                        "sessions": session_list
                    }
                }))
            }
            
            _ => bail!("Unknown tool: {}", tool_name),
        }
    }
    
    fn run(&self) -> Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        
        // Send initial server info notification
        let server_info = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "mcp-ssh-sessions",
                    "version": "0.1.0"
                }
            }
        });
        
        writeln!(stdout, "{}", server_info)?;
        stdout.flush()?;
        eprintln!("[MCP] Server initialized and capabilities sent");
        
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    eprintln!("[MCP] Error reading stdin: {}", e);
                    continue;
                }
            };
            
            if line.trim().is_empty() {
                continue;
            }
            
            eprintln!("[MCP] Received: {}", line);
            
            let request: Value = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("[MCP] JSON parse error: {}", e);
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", e)
                        }
                    });
                    writeln!(stdout, "{}", error_response)?;
                    stdout.flush()?;
                    continue;
                }
            };
            
            let response = match self.handle_request(request.clone()) {
                Ok(result) => {
                    eprintln!("[MCP] Request handled successfully");
                    json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "result": result
                    })
                }
                Err(e) => {
                    eprintln!("[MCP] Request error: {}", e);
                    json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "error": {
                            "code": -32603,
                            "message": e.to_string()
                        }
                    })
                }
            };
            
            eprintln!("[MCP] Sending response: {}", serde_json::to_string(&response)?);
            writeln!(stdout, "{}", response)?;
            stdout.flush()?;
        }
        
        // Cleanup all sessions on exit
        eprintln!("[MCP] Server shutting down, cleaning up sessions");
        let mut sessions = self.sessions.lock().unwrap();
        for (id, session) in sessions.drain() {
            eprintln!("[MCP] Closing session: {}", id);
            let _ = session.disconnect();
        }
        
        Ok(())
    }
}

fn main() -> Result<()> {
    // Set up error handling
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[PANIC] Server panic: {:?}", panic_info);
        std::process::exit(1);
    }));
    
    // Set up signal handler for graceful shutdown
    let result = std::panic::catch_unwind(|| {
        eprintln!("[MAIN] MCP SSH Sessions Server v0.1.0 starting...");
        eprintln!("[MAIN] Protocol: JSON-RPC over stdin/stdout");
        eprintln!("[MAIN] Features: SSH connections with bash relay, sudo support via X11 forwarding");
        
        let server = McpServer::new();
        match server.run() {
            Ok(()) => {
                eprintln!("[MAIN] Server shutdown gracefully");
                0
            }
            Err(e) => {
                eprintln!("[MAIN] Server error: {}", e);
                eprintln!("[MAIN] Error context: {:#}", e);
                1
            }
        }
    });
    
    match result {
        Ok(exit_code) => {
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }
        Err(panic) => {
            eprintln!("[MAIN] Server panicked: {:?}", panic);
            std::process::exit(1);
        }
    }
    
    Ok(())
}