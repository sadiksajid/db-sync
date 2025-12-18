# 📊 Statistics Quick Start Guide

## TL;DR

Run real-time sync, and the proxy will automatically tell you the **best time to migrate** based on actual database activity patterns.

## 3-Step Process

### 1️⃣ Start Real-time Sync (Let it Run 24-48 Hours)

```bash
./rebuild-and-run.sh --realtime-sync
```

### 2️⃣ Make Database Changes (Normal Operations)
The proxy automatically tracks all:
- ✅ INSERT operations
- ✅ UPDATE operations  
- ✅ DELETE operations

### 3️⃣ Review Stats (Press Ctrl+C When Ready)

You'll see output like:
```
📊 ═══════════════════════════════════════════════════════════════
📊 HOURLY OPERATION STATISTICS
📊 ═══════════════════════════════════════════════════════════════
📊 2025-12-09 08:00:00 - 245 inserts, 120 updates, 15 deletes (total: 380)
📊 2025-12-09 09:00:00 - 189 inserts, 95 updates, 8 deletes (total: 292)
📊 2025-12-09 10:00:00 - 450 inserts, 230 updates, 25 deletes (total: 705)
📊 2025-12-09 11:00:00 - 89 inserts, 45 updates, 3 deletes (total: 137)
📊 2025-12-09 12:00:00 - 56 inserts, 22 updates, 2 deletes (total: 80)
📊 ═══════════════════════════════════════════════════════════════
📊 💡 BEST TIME TO SWITCH: 2025-12-09 12:00:00 (only 80 operations)
📊 ═══════════════════════════════════════════════════════════════
```

## What Gets Logged?

### Console Output (Every 5 Minutes)
```
📊 2025-12-09 14:00:00 - 45 inserts, 23 updates, 5 deletes (total: 73)
```

### JSON File (`sync_operations_stats.json`)
```json
[
  {
    "timestamp": "2025-12-09T14:23:45.123Z",
    "hour": "2025-12-09 14:00:00",
    "operation_type": "INSERT",
    "table": "orders"
  }
]
```

## Use Cases

### 1. **Find Low-Activity Window**
Run proxy for 1-2 days → See which hours have lowest activity → Schedule migration

### 2. **Understand Peak Hours**
Identify when NOT to migrate (e.g., lunch hours, end of day)

### 3. **Table-Specific Analysis**
Check the JSON file to see which tables are most active

### 4. **Compliance & Auditing**
Complete timestamped log of all synchronized operations

## Example: Planning a Weekend Migration

**Friday 5 PM**: Start real-time sync
```bash
./rebuild-and-run.sh --realtime-sync
```

**Monday 9 AM**: Press Ctrl+C and review stats

You'll see:
- **Weekend hours**: Low activity (50-100 ops/hour)
- **Business hours**: High activity (500+ ops/hour)

**Recommendation**: Migrate on Saturday 3 AM when you had only 45 operations/hour ✅

**Next Weekend**: Execute final migration
```bash
# Saturday 3 AM
./rebuild-and-run.sh --full-sync
```

## View Stats Anytime

### Pretty Print JSON
```bash
cat sync_operations_stats.json | python -m json.tool
```

### Count Operations
```bash
cat sync_operations_stats.json | jq '. | length'
```

### Group by Hour
```bash
cat sync_operations_stats.json | jq 'group_by(.hour) | .[] | {hour: .[0].hour, count: length}'
```

### Group by Table
```bash
cat sync_operations_stats.json | jq 'group_by(.table) | .[] | {table: .[0].table, count: length}'
```

## Files Created

| File | Description | Location |
|------|-------------|----------|
| `sync_operations_stats.json` | Complete operation log | Current directory |

## Auto-Flush

Stats are automatically saved to disk every **30 seconds**, so even if the proxy crashes, you won't lose data! 💾

## What Happens During Migration?

### Initial Sync (--initial-sync)
❌ No stats collected (one-time bulk transfer)

### Real-time Sync (--realtime-sync)
✅ Stats collected continuously

### Full Sync (--full-sync)
✅ Stats collected during the real-time phase

## Tips

### Tip 1: Run for Representative Period
- ✅ Run for at least 24 hours
- ✅ Include business hours and off-hours
- ✅ Include weekdays if your pattern differs on weekends

### Tip 2: Don't Delete the JSON File
- Keep it for historical analysis
- Archive it after migration: `mv sync_operations_stats.json stats_archive_$(date +%Y%m%d).json`

### Tip 3: Combine with Business Knowledge
- Stats show **technical** load
- You know **business** patterns
- Combine both for optimal planning

### Tip 4: Test First
```bash
./test_stats_logger.sh
```

## Need More Details?

See [STATISTICS.md](STATISTICS.md) for:
- ✅ How to create charts from JSON data
- ✅ How to load into Excel/Python/PostgreSQL
- ✅ Advanced analysis techniques
- ✅ Troubleshooting tips

---

**Ready? Start tracking now!** 🚀

```bash
./rebuild-and-run.sh --realtime-sync
```

