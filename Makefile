.PHONY: build test check clean docker-build run setup

# Build all Rust projects
build:
	cargo build --release

# Run tests
test:
	cargo test --all

# Check code compilation
check:
	cargo check --all

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Build Docker images
docker-build:
	docker-compose build

# Run with Docker
run:
	docker-compose up -d

# Stop services
stop:
	docker-compose down

# Initial setup
setup:
	./setup.sh

# Generate certificates
certs:
	./generate-certs.sh

# View logs
logs:
	docker-compose logs -f

# Build only the secrets tool
build-secrets:
	cargo build --release -p myrss-secrets