#!/bin/bash

echo "Testing OAuth authentication persistence..."

# Test Console OAuth (creates API key)
echo "1. Testing Console OAuth (API Key Creation)"
echo "   This will create an API key using OAuth and save it persistently"
echo "   Command: cargo run --bin code-mesh -- auth login"
echo ""

# Test Auth List to see storage
echo "2. Testing Auth Status"
echo "   This will show current authentication status"
echo "   Command: cargo run --bin code-mesh -- auth list"
echo ""

# Test Run Command to see if it finds stored credentials
echo "3. Testing Run Command with Stored Credentials"
echo "   This will check if the run command finds stored credentials"
echo "   Command: cargo run --bin code-mesh -- run 'help me debug this code'"
echo ""

# Test Logout
echo "4. Testing Logout"
echo "   This will remove stored credentials"
echo "   Command: cargo run --bin code-mesh -- auth logout anthropic"
echo ""

echo "Manual test sequence:"
echo "1. cargo run --bin code-mesh -- auth login"
echo "2. Select 'Anthropic (Claude)'"
echo "3. Select 'Console OAuth (API Key Creation)'"
echo "4. Complete OAuth flow"
echo "5. cargo run --bin code-mesh -- auth list (should show ✅ Authenticated)"
echo "6. cargo run --bin code-mesh -- run 'test' (should find stored credentials)"
echo "7. cargo run --bin code-mesh -- auth logout anthropic (should remove credentials)"
echo "8. cargo run --bin code-mesh -- auth list (should show ❌ Not authenticated)"