#!/bin/bash

# MyRSS Deployment Script

set -e

echo "🚀 Starting MyRSS deployment..."

# Check if master password is set
if [ -z "$MYRSS_MASTER_PASSWORD" ]; then
    echo "❌ Error: MYRSS_MASTER_PASSWORD environment variable is not set"
    exit 1
fi

# Check if secrets file exists
if [ ! -f "secrets.yaml" ]; then
    echo "❌ Error: secrets.yaml file not found"
    echo "Please set up secrets using the myrss-secrets CLI first"
    exit 1
fi

# Check if certificates exist
if [ ! -f "certs/myrss.crt" ] || [ ! -f "certs/myrss.key" ]; then
    echo "⚠️  Warning: SSL certificates not found"
    echo "Generating self-signed certificates..."
    ./generate-certs.sh
fi

# Pull latest images
echo "📦 Pulling latest Docker images..."
docker-compose pull

# Build services
echo "🔨 Building services..."
docker-compose build

# Stop existing services
echo "🛑 Stopping existing services..."
docker-compose down

# Start services
echo "🚀 Starting services..."
docker-compose up -d

# Wait for services to be healthy
echo "⏳ Waiting for services to be healthy..."
sleep 10

# Check service status
echo "✅ Checking service status..."
docker-compose ps

# Run database migrations
echo "🗄️  Running database migrations..."
docker-compose exec myrss-server ./myrss-server migrate

echo "✅ Deployment complete!"
echo ""
echo "Access MyRSS at: https://myrss.local"
echo ""
echo "To view logs:"
echo "  docker-compose logs -f"
echo ""
echo "To stop services:"
echo "  docker-compose down"