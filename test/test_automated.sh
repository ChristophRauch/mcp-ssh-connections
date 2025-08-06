#!/bin/bash

# Automated test for MCP SSH Sessions Server

echo "Running automated MCP test..."

# Create test input
cat > /tmp/mcp_test_input.txt << 'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ssh_connect","arguments":{"host":"kleinebusinesswerkstatt.de"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"date"}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"whoami"}}}
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"pwd"}}}
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"echo Hello from MCP"}}}
{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"whoami","sudo":true}}}
{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"ssh_list_sessions","arguments":{}}}
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"ssh_disconnect","arguments":{"session_id":"kleinebusinesswerkstatt.de"}}}
EOF

# Run the server with test input
timeout 30 cargo run < /tmp/mcp_test_input.txt 2>&1

echo ""
echo "Test complete!"