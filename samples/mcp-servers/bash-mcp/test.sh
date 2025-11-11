#!/usr/bin/env bash
# Quick test script for bash-mcp server
# This tests the basic functionality of the MCP server

BASE_DIR="/tmp"
TEST_DIR="$BASE_DIR/bash-mcp-test"
TEST_FILE="$TEST_DIR/test.txt"

# Create test directory and file
mkdir -p "$TEST_DIR"
echo "Hello from bash-mcp test!" > "$TEST_FILE"

echo "Testing bash-mcp server..."
echo ""

# Test initialize
echo "1. Testing initialize..."
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' | \
  bun samples/mcp-servers/bash-mcp/server.ts "$BASE_DIR" 2>&1 | grep -q "protocolVersion" && echo "✓ Initialize works" || echo "✗ Initialize failed"

# Cleanup
rm -rf "$TEST_DIR"

echo ""
echo "Test complete. For full testing, use cmd2ai with the server configured."

