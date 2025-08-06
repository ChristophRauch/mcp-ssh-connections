#!/bin/bash

# Interactive test for MCP SSH Sessions Server

echo "Starting MCP SSH Sessions Server Interactive Test"
echo "================================================="
echo ""
echo "The server will start and you can send JSON-RPC commands."
echo "Example commands:"
echo ""
echo '1. Initialize:'
echo '   {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
echo ""
echo '2. List tools:'
echo '   {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
echo ""
echo '3. Connect to host:'
echo '   {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ssh_connect","arguments":{"host":"kleinebusinesswerkstatt.de"}}}'
echo ""
echo '4. Execute command:'
echo '   {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"whoami"}}}'
echo ""
echo '5. Execute with sudo:'
echo '   {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ssh_execute","arguments":{"session_id":"kleinebusinesswerkstatt.de","command":"whoami","sudo":true}}}'
echo ""
echo '6. List sessions:'
echo '   {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"ssh_list_sessions","arguments":{}}}'
echo ""
echo '7. Disconnect:'
echo '   {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"ssh_disconnect","arguments":{"session_id":"kleinebusinesswerkstatt.de"}}}'
echo ""
echo "Press Ctrl+C to exit"
echo "================================================="
echo ""

cargo run