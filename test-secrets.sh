#!/bin/bash

# Test script to verify secrets CLI works with piped input

set -e

echo "Testing secrets CLI with environment variable and piped input..."

# Set test password
export MYRSS_MASTER_PASSWORD="test-password"

# Test adding a secret with piped input
echo "test-value" | cargo run --bin myrss-secrets -- add test-key

# Test getting the secret
cargo run --bin myrss-secrets -- get test-key

# Clean up
rm -f secrets.yaml

echo "Test completed successfully!"