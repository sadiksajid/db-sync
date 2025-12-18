#!/bin/bash
# Quick test of statistics logging

echo "🧪 Testing Statistics Logger"
echo ""
echo "Starting real-time sync for 30 seconds to generate stats..."
echo ""

# Run real-time sync for 30 seconds
timeout 30 ./rebuild-and-run.sh --realtime-sync &
PID=$!

# Wait for it to start
sleep 3

echo "Make some changes in MySQL (INSERT/UPDATE/DELETE)..."
echo "Stats will be logged to sync_operations_stats.json"
echo ""

# Wait for timeout
wait $PID

echo ""
echo "✅ Test complete! Check sync_operations_stats.json for results"
echo ""

if [ -f "sync_operations_stats.json" ]; then
    echo "📊 Stats file created successfully!"
    echo "Number of operations logged: $(cat sync_operations_stats.json | jq '. | length' 2>/dev/null || echo 'Install jq to see count')"
else
    echo "⚠️  Stats file not found. Make sure some operations happened in MySQL."
fi
