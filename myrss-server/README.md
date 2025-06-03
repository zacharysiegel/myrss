# MyRSS Server

## Development Setup

### SQLx Offline Mode

This project uses SQLx for compile-time checked SQL queries. For development:

1. Start a PostgreSQL database:
   ```bash
   docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres --name myrss-db postgres:16
   ```

2. Set up the database:
   ```bash
   export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/myrss"
   sqlx database create
   sqlx migrate run
   ```

3. Prepare offline query data:
   ```bash
   cargo sqlx prepare
   ```

This generates `.sqlx/` directory with cached query metadata for offline compilation.

### Docker Build

The Docker build uses a dummy DATABASE_URL to satisfy SQLx's compile-time requirements. In production, the actual database connection is configured at runtime through environment variables.