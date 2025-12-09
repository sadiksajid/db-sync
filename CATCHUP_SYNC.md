# Catch-Up Sync Mechanism

## Overview

The catch-up sync mechanism ensures **zero data loss** during the migration from MySQL to PostgreSQL. It addresses the critical issue of changes that occur while the initial data transfer is in progress.

## The Problem

Consider this scenario:

1. **T0**: Initial sync starts, records timestamp: `2024-12-08 10:00:00`
2. **T0 - T5**: Data transfer in progress (takes 5 minutes for large database)
3. **T2**: User inserts new row in MySQL (during transfer)
4. **T4**: User updates a row in MySQL (during transfer)
5. **T5**: Initial sync completes
6. **Problem**: The INSERT and UPDATE at T2 and T4 are **not** in PostgreSQL!

Without catch-up sync, these changes would be lost until the real-time sync starts detecting new changes.

## The Solution

The catch-up sync mechanism works as follows:

```
┌─────────────────────────────────────────────────────────────┐
│  PHASE 1: Initial Sync                                      │
│  ─────────────────────                                      │
│  1. Record start timestamp (T_start)                        │
│  2. Transfer schema                                         │
│  3. Transfer all data in batches                            │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  PHASE 2: Catch-Up Sync (NEW!)                              │
│  ───────────────────────                                    │
│  Loop until no more changes:                                │
│    1. Query general_log for changes since T_start          │
│    2. Apply all detected changes to PostgreSQL             │
│    3. If changes found, record new timestamp and repeat    │
│    4. If no changes, we're synchronized!                   │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  PHASE 3: Real-Time Sync                                    │
│  ────────────────────────                                   │
│  1. Continue monitoring general_log                         │
│  2. Apply changes in real-time                              │
└─────────────────────────────────────────────────────────────┘
```

## How It Works

### 1. Timestamp Recording

At the start of `run_initial_sync()`, we record MySQL's current timestamp using:

```sql
SELECT NOW(6)  -- Returns: "2024-12-08 10:00:00.123456"
```

This timestamp marks the beginning of the data transfer window.

### 2. Catch-Up Query

After the initial sync completes, we query `mysql.general_log` for all changes since the start timestamp:

```sql
SELECT 
    CAST(argument AS CHAR) as query,
    CAST(event_time AS CHAR) as event_time
FROM mysql.general_log
WHERE 
    command_type = 'Query'
    AND (
        UPPER(CAST(argument AS CHAR)) LIKE 'INSERT%' OR
        UPPER(CAST(argument AS CHAR)) LIKE 'UPDATE%' OR
        UPPER(CAST(argument AS CHAR)) LIKE 'DELETE%'
    )
    AND event_time >= '2024-12-08 10:00:00.123456'
ORDER BY event_time ASC
LIMIT 10000
```

### 3. Iterative Application

The catch-up runs in a loop:

```rust
loop {
    iteration += 1;
    
    // Get timestamp before catch-up
    let before_catchup = get_mysql_timestamp();
    
    // Run catch-up from current timestamp
    let changes_found = binlog_reader.catchup_from_timestamp(&current_timestamp).await?;
    
    if changes_found == 0 {
        // No changes - we're synchronized!
        break;
    }
    
    // Wait for PostgreSQL writer to finish
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Update timestamp for next iteration
    current_timestamp = before_catchup;
    
    // Safety: max 10 iterations
    if iteration >= 10 {
        warn!("Reached maximum catch-up iterations (10)");
        break;
    }
}
```

### 4. Event Processing

Each detected change is:

1. **Parsed**: Extract table name, column names, and values
2. **Enqueued**: Sent to the asynchronous job queue
3. **Applied**: PostgreSQL writer processes the change

The same event queue and writer used for real-time sync handles catch-up events.

## Example Scenario

Let's walk through a real example:

### Timeline

```
10:00:00.000 - Initial sync starts (records timestamp)
10:00:05.000 - User executes: INSERT INTO users (name) VALUES ('Alice')
10:01:30.000 - User executes: UPDATE users SET email='alice@example.com' WHERE name='Alice'
10:03:00.000 - Initial sync completes (3 minutes)
```

### Catch-Up Process

**Iteration 1:**

```
Current timestamp: 2024-12-08 10:00:00.000
Query general_log from 10:00:00.000
Found 2 changes:
  1. INSERT INTO users (name) VALUES ('Alice')        [10:00:05]
  2. UPDATE users SET email='...' WHERE name='Alice'  [10:01:30]
Apply both changes to PostgreSQL
Record new timestamp: 2024-12-08 10:03:00.000
```

**Iteration 2:**

```
Current timestamp: 2024-12-08 10:03:00.000
Query general_log from 10:03:00.000
Found 0 changes
✓ Synchronized! Proceed to real-time sync
```

## Configuration

### Automatic (Default)

When using `--full-sync`, catch-up is automatically enabled:

```bash
docker run --rm \
  -e DB_HOST=192.168.1.237 \
  -e DB_PORT=3306 \
  -e DB_DATABASE=my_db \
  -e DB_USERNAME=root \
  -e DB_PASSWORD=password \
  -e PSQL_DB_HOST=192.168.1.237 \
  -e PSQL_DB_PORT=5432 \
  -e PSQL_DB_DATABASE=my_db \
  -e PSQL_DB_USERNAME=postgres \
  -e PSQL_DB_PASSWORD=postgres \
  mysql_psql_proxy:latest \
  --full-sync
```

### Manual (Separate Modes)

If running `--initial-sync` and `--realtime-sync` separately, there will be a gap:

```bash
# Step 1: Initial sync
docker run ... --initial-sync

# GAP: Changes made here are LOST!

# Step 2: Real-time sync
docker run ... --realtime-sync
```

**Solution**: Always use `--full-sync` to enable catch-up.

## Performance Considerations

### Catch-Up Speed

- **Small changes** (< 100): Very fast (< 1 second)
- **Medium changes** (100-1000): Fast (1-5 seconds)
- **Large changes** (> 1000): Depends on query complexity (5-30 seconds)

### Iteration Limit

The catch-up loop has a safety limit of **10 iterations**. This prevents infinite loops if:

- Database is under heavy write load
- Changes keep occurring faster than catch-up can process

If the limit is reached, a warning is logged, and the system proceeds to real-time sync (which will eventually catch up).

### Queue Capacity

The event queue has a capacity of **1000 events**. If the queue fills up:

- Catch-up blocks until space is available
- Real-time sync drops events (logs warning)

For very large catch-up operations, consider increasing the queue size in `src/main.rs`:

```rust
// Increase from 1000 to 10000
let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(10000);
```

## Requirements

### MySQL Configuration

Catch-up sync requires `general_log` to be enabled:

```sql
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';
```

The proxy attempts to enable this automatically, but requires `SUPER` privilege:

```sql
GRANT SUPER ON *.* TO 'your_user'@'%';
FLUSH PRIVILEGES;
```

### General Log Retention

Ensure `mysql.general_log` retains data long enough for the initial sync to complete. For very large databases:

1. Enable `general_log` before starting the sync
2. Monitor `mysql.general_log` size
3. If needed, increase `max_binlog_size` or adjust retention policies

## Limitations

### 1. Query Parsing

Catch-up uses query parsing, which has limitations:

- **Complex queries**: Multi-table joins, subqueries may not be parsed correctly
- **Stored procedures**: Calls like `CALL my_proc()` are not supported
- **Triggers**: Trigger-generated changes are not captured

**Workaround**: Use binary log (binlog) streaming for more reliable change capture (planned feature).

### 2. Transaction Boundaries

Catch-up applies changes individually, not as transactions:

- **Inconsistency**: If a MySQL transaction involved multiple tables, PostgreSQL may see partial state briefly
- **Impact**: Minimal for most applications, as catch-up completes quickly

### 3. General Log Performance

`mysql.general_log` can grow large and slow down queries:

- **Impact**: Catch-up queries may be slow on heavily-used databases
- **Workaround**: Periodically truncate `mysql.general_log` (safe after catch-up completes)

```sql
TRUNCATE TABLE mysql.general_log;
```

### 4. Timestamp Precision

MySQL timestamps have microsecond precision, but:

- **Clock skew**: If MySQL server clock is not synchronized, catch-up may miss changes
- **Concurrent changes**: Changes occurring in the same microsecond may be processed out of order

**Workaround**: Use NTP to synchronize clocks, ensure `NOW(6)` returns microseconds.

## Monitoring

### Log Messages

Look for these key log messages:

```
INFO  📍 Recording start timestamp for catch-up sync...
INFO  📍 Start timestamp: 2024-12-08 10:00:00.123456

INFO  🔄 Starting catch-up sync to replay changes from initial transfer
INFO  🔄 Catching up from timestamp: 2024-12-08 10:00:00.123456

INFO  🔄 Catch-up iteration #1
INFO  ⚠️  Found 42 changes that occurred during initial transfer
INFO  Applying catch-up changes...
INFO    Applied 42 changes out of 42 found
INFO  ✓ Catch-up complete: applied 42 changes out of 42 found

INFO  🔄 Catch-up iteration #2
INFO  ✓ No changes detected during initial transfer

INFO  ✓ Catch-up complete: databases are synchronized
```

### Error Conditions

Watch for these warnings/errors:

```
WARN  ⚠️  Reached maximum catch-up iterations (10). Proceeding to live sync.
WARN  ⚠️  There may still be pending changes - live sync will handle them.
```

This indicates the database is under heavy write load. The real-time sync will eventually catch up, but there may be a temporary lag.

```
ERROR Failed to send catch-up event to queue: channel closed
```

This indicates the PostgreSQL writer crashed or stopped. Check PostgreSQL connection and logs.

## Testing

### Test Scenario 1: Basic Catch-Up

```bash
# Terminal 1: Start full sync
docker run ... --full-sync

# Terminal 2: While initial sync is running, make changes in MySQL
mysql> INSERT INTO test_table (name) VALUES ('test1');
mysql> UPDATE test_table SET name='test2' WHERE id=1;
mysql> DELETE FROM test_table WHERE id=2;

# Expected: Catch-up detects and applies all 3 changes
# PostgreSQL should have same data as MySQL after catch-up
```

### Test Scenario 2: High Write Load

```bash
# Terminal 1: Start full sync
docker run ... --full-sync

# Terminal 2: Generate high write load
for i in {1..1000}; do
  mysql -e "INSERT INTO test_table (name) VALUES ('test$i')"
done

# Expected: Multiple catch-up iterations, all changes eventually applied
```

### Test Scenario 3: Zero Changes

```bash
# No writes during initial sync
docker run ... --full-sync

# Expected: Catch-up iteration #1 finds 0 changes, completes immediately
```

## Troubleshooting

### Issue: Catch-up takes too long

**Symptoms**: Catch-up runs for many iterations, never stabilizes.

**Causes**:
- Database under heavy write load
- `mysql.general_log` is very large

**Solutions**:
1. Reduce write load during migration
2. Truncate `mysql.general_log` before starting
3. Increase iteration limit in code (if appropriate)

### Issue: Changes not detected

**Symptoms**: Catch-up finds 0 changes, but MySQL and PostgreSQL data differs.

**Causes**:
- `general_log` was disabled during initial sync
- `general_log` was truncated during initial sync
- Clock skew between application and MySQL server

**Solutions**:
1. Ensure `general_log` is enabled before starting
2. Don't truncate `mysql.general_log` during sync
3. Synchronize clocks using NTP

### Issue: Catch-up finds changes but doesn't apply them

**Symptoms**: Catch-up logs "Found X changes" but PostgreSQL is not updated.

**Causes**:
- PostgreSQL writer crashed
- Event queue is full
- PostgreSQL connection lost

**Solutions**:
1. Check PostgreSQL logs for errors
2. Increase event queue capacity
3. Verify PostgreSQL connection is stable

## Future Enhancements

### 1. Binary Log (Binlog) Streaming

Replace `general_log` polling with true binlog streaming:

- **Pros**: More reliable, better performance, transaction boundaries preserved
- **Cons**: More complex setup, requires binlog configuration

### 2. Checkpointing

Save catch-up progress to allow resuming from crash:

- **Pros**: Can recover from failures, no duplicate processing
- **Cons**: Requires persistent storage, more complexity

### 3. Parallel Catch-Up

Process catch-up changes in parallel:

- **Pros**: Faster catch-up for large backlogs
- **Cons**: Must preserve ordering for same-row updates, more complex

### 4. Adaptive Iteration Limit

Adjust iteration limit based on database size and write rate:

- **Pros**: Better handling of high write loads
- **Cons**: Requires heuristics, may be unpredictable

## Summary

The catch-up sync mechanism ensures **zero data loss** during MySQL to PostgreSQL migration by:

1. **Recording** the start timestamp of the initial data transfer
2. **Detecting** all changes that occurred during the transfer using `mysql.general_log`
3. **Applying** those changes to PostgreSQL in chronological order
4. **Repeating** until no more changes are found
5. **Transitioning** seamlessly to real-time sync

This approach provides a robust, automated solution to the critical problem of maintaining data consistency during migration.

