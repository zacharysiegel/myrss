#!/bin/bash

# MyRSS Initial Setup Script

set -e

echo "MyRSS Initial Setup"
echo "====================="
echo ""
echo "This script is idempotent - it's safe to run multiple times"
echo ""

# Check dependencies
echo "[INFO] Checking dependencies..."

if ! command -v docker &> /dev/null; then
    echo "[ERROR] Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "[ERROR] Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

echo "[SUCCESS] Dependencies satisfied"
echo ""

# Generate certificates
if [ ! -f "certs/myrss.crt" ] || [ ! -f "certs/myrss.key" ]; then
    echo "[INFO] Generating self-signed certificates..."
    ./generate-certs.sh
    echo ""
fi

# Set up master password
if [ -f ".env" ]; then
    echo "[INFO] Loading existing master password..."
    source .env
    if [ -z "$MYRSS_MASTER_PASSWORD" ]; then
        echo "[ERROR] .env file exists but MYRSS_MASTER_PASSWORD is not set"
        exit 1
    fi
    MASTER_PASSWORD="$MYRSS_MASTER_PASSWORD"
    echo "[SUCCESS] Master password loaded from .env"
else
    echo "[INFO] Setting up new master password..."
    read -s -p "Enter master password for secrets encryption: " MASTER_PASSWORD
    echo ""
    read -s -p "Confirm master password: " MASTER_PASSWORD_CONFIRM
    echo ""

    if [ "$MASTER_PASSWORD" != "$MASTER_PASSWORD_CONFIRM" ]; then
        echo "[ERROR] Passwords do not match"
        exit 1
    fi

    echo "export MYRSS_MASTER_PASSWORD='$MASTER_PASSWORD'" > .env
    echo "[SUCCESS] Master password saved to .env"
fi
echo ""

# Build secrets CLI
if [ -f "./target/release/myrss-secrets" ]; then
    echo "[INFO] Secrets tool already built, skipping..."
else
    echo "[INFO] Building secrets management tool..."
    echo "   (This may take a few minutes on first run)"
    
    # Retry logic for network issues
    MAX_RETRIES=3
    RETRY_COUNT=0
    while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
        if docker run --rm -v "$PWD":/app -w /app rust:1.82 cargo build --release -p myrss-secrets; then
            echo "[SUCCESS] Secrets tool built successfully"
            break
        else
            RETRY_COUNT=$((RETRY_COUNT + 1))
            if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
                echo "[WARNING] Build failed, retrying ($RETRY_COUNT/$MAX_RETRIES)..."
                sleep 5
            else
                echo "[ERROR] Failed to build secrets tool after $MAX_RETRIES attempts"
                echo "   This might be due to temporary network issues with crates.io"
                echo "   Please try running the setup again in a few minutes"
                exit 1
            fi
        fi
    done
fi
echo ""

# Initialize secrets if not already done
if [ -f "secrets.yaml" ]; then
    echo "[INFO] Secrets file already exists, skipping initialization..."
    echo ""
else
    echo "[INFO] Initializing secrets..."
    
    # Generate session key
    SESSION_KEY=$(openssl rand -hex 32)
    echo "$SESSION_KEY" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.82 ./target/release/myrss-secrets add session_key
    
    # Set database URL (using 'postgres' as hostname for Docker networking)
    echo "postgresql://myrss:myrss@postgres/myrss" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.82 ./target/release/myrss-secrets add database_url
    
    # Create default admin user
    echo "[INFO] Creating default admin user..."
    read -p "Enter admin username (default: admin): " ADMIN_USER
    ADMIN_USER=${ADMIN_USER:-admin}
    
    read -s -p "Enter admin password: " ADMIN_PASS
    echo ""
    
    # Hash the password
    ADMIN_PASS_HASH=$(echo -n "$ADMIN_PASS" | sha256sum | cut -d' ' -f1)
    
    # Create users JSON
    USERS_JSON="[{\"username\":\"$ADMIN_USER\",\"password_hash\":\"$ADMIN_PASS_HASH\"}]"
    echo "$USERS_JSON" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.82 ./target/release/myrss-secrets add auth_users
    
    echo "[SUCCESS] Secrets initialized"
    echo ""
fi

# Update hosts file
echo "[INFO] Updating /etc/hosts..."
if ! grep -q "myrss.local" /etc/hosts; then
    echo "127.0.0.1    myrss.local" | sudo tee -a /etc/hosts > /dev/null
    echo "[SUCCESS] Added myrss.local to /etc/hosts"
else
    echo "[SUCCESS] myrss.local already in /etc/hosts"
fi
echo ""

# Deploy
echo "[INFO] Deploying MyRSS..."
export MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD"
./deploy.sh

echo ""
echo "[SUCCESS] Setup complete!"
echo ""
echo "Access MyRSS at: https://myrss.local"
echo "Username: $ADMIN_USER"
echo "Password: [the password you entered]"
echo ""
echo "[WARNING] Note: Your browser will warn about the self-signed certificate."
echo "This is expected for local development. Click 'Advanced' and proceed."