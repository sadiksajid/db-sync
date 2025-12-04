# Real-Time Sync Setup Guide

## Problem: No Changes Detected

If you're running `--realtime-sync` but not seeing any changes replicated, it's likely because MySQL's `general_log` is not enabled or accessible.

## Solution: Enable MySQL General Log

### Option 1: Enable via MySQL Command (Requires SUPER privilege)

Connect to MySQL and run:

```sql
-- Enable general log
SET GLOBAL general_log = 'ON';

-- Set log output to table (so we can query it)
SET GLOBAL log_output = 'TABLE';

-- Verify it's enabled
SHOW VARIABLES LIKE 'general_log';
-- Should show: general_log = ON
```

### Option 2: Enable via MySQL Configuration File

Add to `/etc/mysql/my.cnf` or `/etc/my.cnf`:

```ini
[mysqld]
general_log = 1
log_output = TABLE
```

Then restart MySQL:
```bash
sudo systemctl restart mysql
```

### Option 3: Grant SUPER Privilege to Your User

If you can't enable general_log, grant SUPER privilege:

```sql
GRANT SUPER ON *.* TO 'your_username'@'%';
FLUSH PRIVILEGES;
```

## Verify Setup

After enabling, check if it's working:

```sql
-- Check if general_log is enabled
SHOW VARIABLES LIKE 'general_log';

-- Check if you can query the log
SELECT COUNT(*) FROM mysql.general_log LIMIT 1;

-- Make a test change
UPDATE your_table SET column = 'test' WHERE id = 1;

-- Check if it appears in the log
SELECT * FROM mysql.general_log 
WHERE argument LIKE 'UPDATE%' 
ORDER BY event_time DESC 
LIMIT 5;
```

## Testing Real-Time Sync

1. Start the real-time sync:
   ```bash
   ./rebuild-and-run.sh --realtime-sync
   ```

2. In another terminal, make a change in MySQL:
   ```sql
   UPDATE your_table SET name = 'updated' WHERE id = 1;
   ```

3. You should see logs like:
   ```
   INFO: Detected UPDATE query: UPDATE your_table...
   INFO: Processing UPDATE for table: your_table
   INFO: ✓ Successfully updated row in table: your_table
   ```

## Troubleshooting

### Error: "Access denied for mysql.general_log"
- Your MySQL user needs SUPER privilege
- Run: `GRANT SUPER ON *.* TO 'your_user'@'%';`

### Error: "Table 'mysql.general_log' doesn't exist"
- General log is disabled
- Enable it: `SET GLOBAL general_log = 'ON';`

### No logs appearing
- Check if general_log is actually ON: `SHOW VARIABLES LIKE 'general_log';`
- Check if log_output is TABLE: `SHOW VARIABLES LIKE 'log_output';`
- Make sure you're making changes to the monitored database

### Performance Concerns
- General log can impact MySQL performance
- Consider enabling only during sync periods
- Or use binlog replication (more advanced)

