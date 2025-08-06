#!/bin/bash

# Test if MCP tools can be seen

echo "Testing MCP tool visibility..."

# Create test input for basic MCP protocol
cat > /tmp/mcp_tools_test.txt << 'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
EOF

echo "Starting MCP server and testing tool visibility..."
timeout 10 cargo run < /tmp/mcp_tools_test.txt 2>/dev/null

echo ""
echo "Test completed."