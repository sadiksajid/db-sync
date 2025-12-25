#!/bin/bash

echo "🔄 Building Docker image..."
docker build -q -t db_sync_proxy:latest . || {
    echo "❌ Build failed!"
    exit 1
}

echo "✅ Build complete!"
echo ""
echo "🌐 Starting Web UI on port 5009..."
echo "📊 Open your browser at: http://localhost:5009"
echo "💾 Configuration will be saved to: ./db_sync_data/"
echo ""
echo "Press Ctrl+C to stop the server"
echo "----------------------------------------"

# Create data directory if it doesn't exist
mkdir -p ./db_sync_data

docker run --rm -it \
  -p 5009:5009 \
  -v "$(pwd)/db_sync_data:/app/data" \
  db_sync_proxy:latest \
  --web-ui

