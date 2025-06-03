#!/bin/bash

# MyRSS Deployment Script

set -e

echo "[INFO] Starting MyRSS deployment..."

# Check if master password is set
if [ -z "$MYRSS_MASTER_PASSWORD" ]; then
    echo "[ERROR] MYRSS_MASTER_PASSWORD environment variable is not set"
    exit 1
fi

# Check if secrets file exists
if [ ! -f "secrets.yaml" ]; then
    echo "[ERROR] secrets.yaml file not found"
    echo "Please set up secrets using the myrss-secrets CLI first"
    exit 1
fi

# Check if certificates exist
if [ ! -f "certs/myrss.crt" ] || [ ! -f "certs/myrss.key" ]; then
    echo "[WARNING] SSL certificates not found"
    echo "Generating self-signed certificates..."
    ./generate-certs.sh
fi

# Pull latest images
echo "[INFO] Pulling latest Docker images..."
docker-compose pull

# Build services
echo "[INFO] Building services..."
docker-compose build

# Stop existing services
echo "[INFO] Stopping existing services..."
docker-compose down

# Start services
echo "[INFO] Starting services..."
docker-compose up -d

# Wait for services to be healthy
echo "[INFO] Waiting for services to be healthy..."
sleep 10

# Check service status
echo "[INFO] Checking service status..."
docker-compose ps

# Run database migrations
echo "[INFO] Running database migrations..."
docker-compose exec myrss-server ./myrss-server migrate

echo "[SUCCESS] Deployment complete!"
echo ""
echo "Access MyRSS at: https://myrss.local"
echo ""
echo "To view logs:"
echo "  docker-compose logs -f"
echo ""
echo "To stop services:"
echo "  docker-compose down"