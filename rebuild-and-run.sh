#!/bin/bash
# Rebuild Docker image and run sync
# Usage: ./rebuild-and-run.sh [--initial-sync|--realtime-sync|--full-sync]
# Default: --full-sync (initial + realtime)

SYNC_MODE=${1:---full-sync}

echo "Building Docker image..."
IMAGE_SHA=$(docker build -q -t mysql_psql_proxy:latest .)

if [ $? -eq 0 ]; then
    echo "✓ Build successful: $IMAGE_SHA"
    echo "Starting sync with mode: $SYNC_MODE"
    echo "=========================================="
    echo ""
    
    docker run --rm -it \
      -e DB_HOST=192.168.1.237 \
      -e DB_PORT=3306 \
      -e DB_DATABASE=testing \
      -e DB_USERNAME=root \
      -e DB_PASSWORD=password \
      -e PSQL_DB_HOST=192.168.1.237 \
      -e PSQL_DB_PORT=5432 \
      -e PSQL_DB_DATABASE=testing \
      -e PSQL_DB_USERNAME=postgres \
      -e PSQL_DB_PASSWORD=postgres \
        -e GEMINI_API_KEY="" \
        -e GEMINI_MODEL="gemini-2.0-flash-exp" \
        -e POLL_INTERVAL_SECS=10 \
      -e BATCH_SIZE=200 \
      -e RUST_LOG=info \
      mysql_psql_proxy:latest \
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
