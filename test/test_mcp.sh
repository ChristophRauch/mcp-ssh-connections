#!/bin/bash

# Test script for MCP SSH Sessions Server

echo "Testing MCP SSH Sessions Server"
echo "================================"

# Start the server in the background
cargo run 2>/dev/null &
SERVER_PID=$!
sleep 1

# Function to send JSON-RPC request
send_request() {
    echo "$1" | nc -N localhost 3000 2>/dev/null || echo "$1"
}

# Test 1: Initialize
echo "Test 1: Initialize"
send_request '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

# Test 2: List tools
echo -e "\nTest 2: List tools"
send_request '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Test 3: Connect to host
echo -e "\nTest 3: Connect to kleinebusinesswerkstatt.de"
send_request '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ssh_connect","arguments":{"host":"kleinebusinesswerkstatt.de"}}}'

# Test 4: Execute command
echo -e "\nTest 4: Execute 'whoami'"
send_request '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"whoami"}}}'

# Test 5: List sessions
echo -e "\nTest 5: List sessions"
send_request '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ssh_list_sessions","arguments":{}}}' 

# Test 6: Disconnect
echo -e "\nTest 6: Disconnect"
send_request '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ssh_disconnect","arguments":{"session_id":"kleinebusinesswerkstatt.de"}}}'

# Cleanup
kill $SERVER_PID 2>/dev/null
echo -e "\nTest complete!"