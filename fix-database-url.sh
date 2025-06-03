#!/bin/bash

echo "[INFO] Fixing database URL for Docker networking..."

# Check if secrets file exists
if [ ! -f "secrets.yaml" ]; then
    echo "[ERROR] secrets.yaml not found. Please run setup.sh first."
    exit 1
fi

# Check for .env file
if [ ! -f ".env" ]; then
    echo "[ERROR] .env file not found. Please run setup.sh first."
    exit 1
fi

# Source the .env file
source .env

if [ -z "$MYRSS_MASTER_PASSWORD" ]; then
    echo "[ERROR] MYRSS_MASTER_PASSWORD not set in .env"
    exit 1
fi

echo "[INFO] Current database URL:"
./target/release/myrss-secrets get database_url 2>/dev/null || echo "(Could not read current value)"

echo ""
echo "[INFO] Setting correct database URL for Docker..."
echo "postgresql://myrss:myrss@postgres/myrss" | ./target/release/myrss-secrets add database_url

echo ""
echo "[SUCCESS] Database URL updated!"
echo ""
echo "Now restart the services:"
echo "  docker-compose restart"
echo ""
echo "Or redeploy:"
echo "  ./deploy.sh"