# 📊 Real-time Statistics & Best Migration Window

The proxy automatically tracks all database operations during real-time synchronization to help you identify the **best time to switch from MySQL to PostgreSQL** with minimal impact.

## Features

### 1. **Automatic Statistics Collection**
- ✅ Tracks all `INSERT`, `UPDATE`, `DELETE` operations
- ✅ Logs timestamp, operation type, and table name
- ✅ Groups operations by hour
- ✅ Persists to JSON file (survives restarts)

### 2. **Real-time Console Output**
Every 5 minutes, you'll see:
```
📊 2025-12-09 14:00:00 - 45 inserts, 23 updates, 5 deletes (total: 73)
```

### 3. **Best Migration Window**
At the end of sync (or when you stop it), you'll see:
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

## How It Works

### During `--realtime-sync`:
1. **Every operation** is logged to `sync_operations_stats.json`
2. **Every 5 minutes**, console shows current hour stats
3. **Every 30 seconds**, stats are flushed to disk (for safety)
4. **On exit**, full hourly summary is displayed

### Output Files

#### `sync_operations_stats.json`
Complete operation log in JSON format:
```json
[
  {
    "timestamp": "2025-12-09T14:23:45.123Z",
    "hour": "2025-12-09 14:00:00",
    "operation_type": "INSERT",
    "table": "orders"
  },
  {
    "timestamp": "2025-12-09T14:24:12.456Z",
    "hour": "2025-12-09 14:00:00",
    "operation_type": "UPDATE",
    "table": "customers"
  }
]
```

### Visualization Ready

You can use the JSON file to create charts:
- **Excel/Google Sheets**: Import JSON, create pivot table, visualize
- **Python/Pandas**: `pd.read_json('sync_operations_stats.json')`
- **JavaScript/Chart.js**: Load JSON, group by hour, display

#### Example Python Visualization:

```python
import pandas as pd
import matplotlib.pyplot as plt

# Load stats
df = pd.read_json('sync_operations_stats.json')

# Group by hour and operation type
hourly = df.groupby(['hour', 'operation_type']).size().unstack(fill_value=0)

# Plot
hourly.plot(kind='bar', stacked=True, figsize=(12, 6))
plt.title('Database Operations by Hour')
plt.xlabel('Hour')
plt.ylabel('Number of Operations')
plt.legend(title='Operation Type')
plt.xticks(rotation=45)
plt.tight_layout()
plt.savefig('operations_chart.png')
```

## Usage

### Start Real-time Sync with Statistics

```bash
./rebuild-and-run.sh --realtime-sync
```

The statistics file `sync_operations_stats.json` will be created automatically in the current directory.

### Full Sync (Initial + Real-time)

```bash
./rebuild-and-run.sh --full-sync
```

Statistics will only be collected during the real-time phase.

### View Statistics

The stats are automatically displayed:
- **Every 5 minutes**: Current hour summary in console
- **On exit**: Full hourly breakdown + best migration window

### Stop and Review

Press `Ctrl+C` to stop the sync. You'll see:
1. ✅ Final stats flush to disk
2. 📊 Complete hourly summary
3. 💡 Recommended migration window (lowest activity hour)

## Planning Your Migration

### Step 1: Run for 24-48 Hours
Let the proxy run during your normal business hours to collect representative data:

```bash
./rebuild-and-run.sh --realtime-sync
```

### Step 2: Review the Stats
Look at the final summary or analyze the JSON file:
- **Identify peak hours** (highest operations)
- **Identify quiet hours** (lowest operations)
- **Consider your business patterns** (e.g., weekends, off-hours)

### Step 3: Schedule the Switch
Plan your final migration during the **recommended time window**:
- Lowest operation count = minimal catch-up required
- Faster synchronization
- Lower risk of data conflicts

### Step 4: Execute Final Migration

```bash
# During the quiet window:
./rebuild-and-run.sh --full-sync
```

The catch-up phase will be much faster during low-activity periods!

## File Management

### Default Location
- **File**: `sync_operations_stats.json`
- **Location**: Same directory where you run the proxy

### Backup Stats
```bash
cp sync_operations_stats.json stats_backup_$(date +%Y%m%d_%H%M%S).json
```

### Archive Old Stats
```bash
mkdir -p stats_archive
mv sync_operations_stats.json stats_archive/stats_$(date +%Y%m%d).json
```

### Start Fresh
```bash
rm sync_operations_stats.json
# Next run will create a new file
```

## Troubleshooting

### Stats File Not Created
- ✅ Check you're running `--realtime-sync` or `--full-sync`
- ✅ Check file permissions in the current directory
- ✅ Look for error messages in console

### Stats Not Updating
- ✅ Make sure database operations are happening (test with INSERT/UPDATE/DELETE)
- ✅ Check console for "📊" messages
- ✅ Stats flush every 30 seconds

### Viewing JSON File
```bash
# Pretty print
cat sync_operations_stats.json | python -m json.tool

# Count operations
cat sync_operations_stats.json | jq '. | length'

# Group by hour
cat sync_operations_stats.json | jq 'group_by(.hour) | .[] | {hour: .[0].hour, count: length}'
```

## Advanced: Custom Analysis

### Load in PostgreSQL
```sql
CREATE TABLE operation_stats (
    timestamp TIMESTAMPTZ,
    hour VARCHAR(50),
    operation_type VARCHAR(10),
    table_name VARCHAR(100)
);

COPY operation_stats(timestamp, hour, operation_type, table_name)
FROM '/path/to/sync_operations_stats.json'
WITH (FORMAT JSON);

-- Find busiest tables
SELECT table_name, COUNT(*) as ops
FROM operation_stats
GROUP BY table_name
ORDER BY ops DESC;

-- Find quietest hour
SELECT hour, COUNT(*) as ops
FROM operation_stats
GROUP BY hour
ORDER BY ops ASC
LIMIT 1;
```

### Load in MySQL
```sql
-- First install JSON UDF functions or use MySQL 8.0+

CREATE TABLE operation_stats (
    timestamp DATETIME,
    hour VARCHAR(50),
    operation_type VARCHAR(10),
    table_name VARCHAR(100)
);

-- Then import JSON using your preferred method
```

## Benefits

### 🎯 **Data-Driven Migration Planning**
- Know exactly when your database is quietest
- Minimize downtime and user impact

### 📈 **Performance Insights**
- See which tables are most active
- Understand your write patterns

### 🚀 **Faster Migrations**
- Migrate during low-activity periods
- Reduce catch-up synchronization time

### 📊 **Compliance & Auditing**
- Complete log of all synchronized operations
- Timestamp-accurate tracking

## Summary

The statistics feature gives you **data-driven insights** to plan your migration for **minimal impact**. 

Run the proxy for a day or two, review the stats, and switch during your quietest hour! 🎯

---

**Next Steps:**
1. Start real-time sync: `./rebuild-and-run.sh --realtime-sync`
2. Let it run for 24-48 hours
3. Review the summary (or press Ctrl+C anytime)
4. Plan your migration during the recommended window
5. Execute final switch with confidence! ✅

