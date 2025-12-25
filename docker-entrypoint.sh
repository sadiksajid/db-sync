#!/bin/bash
set -e

echo "🚀 Starting Database Sync Proxy..."
echo "📂 Working directory: $(pwd)"
echo "📁 Data directory: /app/data"
ls -la /app/data || echo "⚠️  Data directory doesn't exist"

# Check binary
if [ ! -f /usr/local/bin/db_sync_proxy ]; then
  echo "❌ Binary not found!"
  exit 1
fi

# Check if it's executable
if [ ! -x /usr/local/bin/db_sync_proxy ]; then
  echo "❌ Binary not executable!"
  exit 1
fi

echo "🔧 Running: /usr/local/bin/db_sync_proxy $@"
echo "📝 RUST_LOG=$RUST_LOG"

# Use exec to replace the shell with the application
exec /usr/local/bin/db_sync_proxy "$@"

