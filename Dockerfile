# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Create a dummy src directory to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/mysql_psql_proxy /usr/local/bin/mysql_psql_proxy

# Make it executable
RUN chmod +x /usr/local/bin/mysql_psql_proxy

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/mysql_psql_proxy"]

