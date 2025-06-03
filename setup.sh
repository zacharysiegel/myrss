#!/bin/bash

# MyRSS Initial Setup Script

set -e

echo "🚀 MyRSS Initial Setup"
echo "====================="
echo ""

# Check dependencies
echo "📋 Checking dependencies..."

if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

echo "✅ Dependencies satisfied"
echo ""

# Generate certificates
if [ ! -f "certs/myrss.crt" ] || [ ! -f "certs/myrss.key" ]; then
    echo "🔐 Generating self-signed certificates..."
    ./generate-certs.sh
    echo ""
fi

# Set up master password
echo "🔑 Setting up master password..."
read -s -p "Enter master password for secrets encryption: " MASTER_PASSWORD
echo ""
read -s -p "Confirm master password: " MASTER_PASSWORD_CONFIRM
echo ""

if [ "$MASTER_PASSWORD" != "$MASTER_PASSWORD_CONFIRM" ]; then
    echo "❌ Passwords do not match"
    exit 1
fi

echo "export MYRSS_MASTER_PASSWORD='$MASTER_PASSWORD'" > .env
echo "✅ Master password set"
echo ""

# Build secrets CLI
echo "🔨 Building secrets management tool..."
docker run --rm -v "$PWD":/app -w /app rust:1.75 cargo build --release -p myrss-secrets
echo "✅ Secrets tool built"
echo ""

# Initialize secrets
echo "🔐 Initializing secrets..."

# Generate session key
SESSION_KEY=$(openssl rand -hex 32)
echo "$SESSION_KEY" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.75 ./target/release/myrss-secrets add session_key

# Set database URL
echo "postgresql://myrss:myrss@postgres/myrss" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.75 ./target/release/myrss-secrets add database_url

# Create default admin user
echo "📝 Creating default admin user..."
read -p "Enter admin username (default: admin): " ADMIN_USER
ADMIN_USER=${ADMIN_USER:-admin}

read -s -p "Enter admin password: " ADMIN_PASS
echo ""

# Hash the password
ADMIN_PASS_HASH=$(echo -n "$ADMIN_PASS" | sha256sum | cut -d' ' -f1)

# Create users JSON
USERS_JSON="[{\"username\":\"$ADMIN_USER\",\"password_hash\":\"$ADMIN_PASS_HASH\"}]"
echo "$USERS_JSON" | docker run --rm -i -v "$PWD":/app -w /app -e MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD" rust:1.75 ./target/release/myrss-secrets add auth_users

echo "✅ Secrets initialized"
echo ""

# Update hosts file
echo "📝 Updating /etc/hosts..."
if ! grep -q "myrss.local" /etc/hosts; then
    echo "127.0.0.1    myrss.local" | sudo tee -a /etc/hosts > /dev/null
    echo "✅ Added myrss.local to /etc/hosts"
else
    echo "✅ myrss.local already in /etc/hosts"
fi
echo ""

# Deploy
echo "🚀 Deploying MyRSS..."
export MYRSS_MASTER_PASSWORD="$MASTER_PASSWORD"
./deploy.sh

echo ""
echo "✅ Setup complete!"
echo ""
echo "Access MyRSS at: https://myrss.local"
echo "Username: $ADMIN_USER"
echo "Password: [the password you entered]"
echo ""
echo "⚠️  Note: Your browser will warn about the self-signed certificate."
echo "This is expected for local development. Click 'Advanced' and proceed."