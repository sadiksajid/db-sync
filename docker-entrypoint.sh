#!/bin/bash
set -e

echo "🚀 Starting MySQL to PostgreSQL Proxy..."
echo "📂 Working directory: $(pwd)"
echo "📁 Data directory: /app/data"
ls -la /app/data || echo "⚠️  Data directory doesn't exist"

# Check binary
if [ ! -f /usr/local/bin/mysql_psql_proxy ]; then
  echo "❌ Binary not found!"
  exit 1
fi

# Check if it's executable
if [ ! -x /usr/local/bin/mysql_psql_proxy ]; then
  echo "❌ Binary not executable!"
  exit 1
fi

# Test binary
echo "🧪 Testing binary..."
/usr/local/bin/mysql_psql_proxy --version 2>&1 || echo "⚠️  --version failed with code $?"

echo "🔧 Running: /usr/local/bin/mysql_psql_proxy $@"
echo "📝 RUST_LOG=$RUST_LOG"

# Don't use exec so we can catch the exit code
/usr/local/bin/mysql_psql_proxy "$@" 2>&1
EXIT_CODE=$?
echo "❌ Binary exited with code: $EXIT_CODE"
exit $EXIT_CODE

