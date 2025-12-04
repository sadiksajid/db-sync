#!/bin/bash
# Rebuild Docker image and run sync
# Usage: ./rebuild-and-run.sh [--initial-sync|--realtime-sync|--full-sync]
# Default: --full-sync (initial + realtime)

SYNC_MODE=${1:---full-sync}

docker build -q -t mysql_psql_proxy:latest . && \
docker run --rm \
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
  -e SYNC_MODE=initial \
  -e BATCH_SIZE=200 \
  -e RUST_LOG=info \
  mysql_psql_proxy:latest $SYNC_MODE
