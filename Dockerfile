# Build stage  
FROM rust:slim-bookworm as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Create a dummy src directory to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/db_sync_proxy* target/release/deps/db_sync_proxy*

# Copy source code and static files
COPY src ./src
COPY static ./static

# Build the application (force rebuild of main binary)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    default-mysql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/db_sync_proxy /usr/local/bin/db_sync_proxy

# Copy static files for web UI
COPY --from=builder /app/static ./static

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/

# Create data directory for SQLite database
RUN mkdir -p /app/data && chmod 755 /app/data

# Make binaries executable
RUN chmod +x /usr/local/bin/db_sync_proxy && \
    chmod +x /usr/local/bin/docker-entrypoint.sh

# Expose web UI port
EXPOSE 5009

# Volume for persistent configuration
VOLUME ["/app/data"]

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

