#!/bin/bash

# Generate self-signed certificates for local development

echo "Generating self-signed certificates for myrss.local..."

# Create certificate directory if it doesn't exist
mkdir -p certs

# Generate private key
openssl genrsa -out certs/myrss.key 2048

# Generate certificate signing request
openssl req -new -key certs/myrss.key -out certs/myrss.csr -subj "/C=US/ST=State/L=City/O=MyRSS/CN=myrss.local"

# Generate self-signed certificate (valid for 365 days)
openssl x509 -req -days 365 -in certs/myrss.csr -signkey certs/myrss.key -out certs/myrss.crt

# Clean up CSR
rm certs/myrss.csr

# Set appropriate permissions
chmod 600 certs/myrss.key
chmod 644 certs/myrss.crt

echo "Certificates generated successfully!"
echo "- Certificate: certs/myrss.crt"
echo "- Private key: certs/myrss.key"
echo ""
echo "Don't forget to add myrss.local to your /etc/hosts file:"
echo "127.0.0.1    myrss.local"