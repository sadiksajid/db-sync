#!/bin/bash
# Rebuild Docker image and run sync
# Usage: ./rebuild-and-run.sh [--initial-sync|--realtime-sync|--full-sync]
# Default: --initial-sync
# Note: --full-sync and --realtime-sync only work with MySQL

SYNC_MODE=${1:---initial-sync}

echo "Building Docker image..."
IMAGE_SHA=$(docker build -q -t db_sync_proxy:latest .)

if [ $? -eq 0 ]; then
    echo "✓ Build successful: $IMAGE_SHA"
    echo "Starting sync with mode: $SYNC_MODE"
    echo "=========================================="
    echo ""
    
    # Example: MySQL to MySQL sync
    # Change SOURCE_DB_TYPE and TARGET_DB_TYPE to "postgresql" for PostgreSQL sync
    docker run --rm -it \
      -e SOURCE_DB_TYPE=mysql \
      -e TARGET_DB_TYPE=mysql \
      -e SOURCE_DB_HOST=192.168.1.237 \
      -e SOURCE_DB_PORT=3306 \
      -e SOURCE_DB_DATABASE=sourcedb \
      -e SOURCE_DB_USERNAME=root \
      -e SOURCE_DB_PASSWORD=password \
      -e TARGET_DB_HOST=192.168.1.237 \
      -e TARGET_DB_PORT=3307 \
      -e TARGET_DB_DATABASE=targetdb \
      -e TARGET_DB_USERNAME=root \
      -e TARGET_DB_PASSWORD=password \
        -e POLL_INTERVAL_SECS=10 \
      -e BATCH_SIZE=200 \
      -e RUST_LOG=info \
      db_sync_proxy:latest \
      "$SYNC_MODE" 2>&1
    
    EXIT_CODE=$?
    echo ""
    echo "=========================================="
    if [ $EXIT_CODE -eq 0 ]; then
        echo "✓ Sync completed successfully"
    else
        echo "✗ Sync failed with exit code: $EXIT_CODE"
    fi
    exit $EXIT_CODE
else
    echo "✗ Build failed"
    exit 1
fi
