# ⚡ Quick Reference Card

## 🚀 Common Commands

```bash
# Monitor database activity (collect statistics)
./rebuild-and-run.sh --realtime-sync

# Run complete migration (initial + catch-up + real-time)
./rebuild-and-run.sh --full-sync

# Initial data transfer only
./rebuild-and-run.sh --initial-sync

# Rebuild Docker image
docker build -t mysql_psql_proxy:latest .
```

## 📊 Statistics

### What You'll See

**Every 5 minutes:**
```
📊 2025-12-09 14:00:00 - 45 inserts, 23 updates, 5 deletes (total: 73)
```

**On exit (Ctrl+C):**
```
📊 💡 BEST TIME TO SWITCH: 2025-12-09 12:00:00 (only 80 operations)
```

### Output Files

| File | Description |
|------|-------------|
| `sync_operations_stats.json` | Complete operation log (JSON) |

### Quick Analysis

```bash
# Count operations
cat sync_operations_stats.json | jq '. | length'

# Group by hour
cat sync_operations_stats.json | jq 'group_by(.hour) | .[] | {hour: .[0].hour, count: length}'

# Group by table
cat sync_operations_stats.json | jq 'group_by(.table) | .[] | {table: .[0].table, count: length}'

# Pretty print
cat sync_operations_stats.json | python -m json.tool
```

## 🔧 Environment Variables

### Required

```bash
DB_HOST=192.168.1.237          # MySQL host
DB_PORT=3306                    # MySQL port
DB_DATABASE=mydb                # MySQL database
DB_USERNAME=root                # MySQL user
DB_PASSWORD=password            # MySQL password

PSQL_DB_HOST=192.168.1.237     # PostgreSQL host
PSQL_DB_PORT=5432               # PostgreSQL port
PSQL_DB_DATABASE=mydb           # PostgreSQL database
PSQL_DB_USERNAME=postgres       # PostgreSQL user
PSQL_DB_PASSWORD=postgres       # PostgreSQL password
```

### Optional

```bash
BATCH_SIZE=200                                  # Records per batch (default: 200)
RUST_LOG=info                                   # Log level (info, debug, warn, error)
POLL_INTERVAL_SECS=5                           # general_log poll interval (default: 5)
GEMINI_API_KEY=AIza...                         # Gemini API key (for DB objects)
GEMINI_MODEL=gemini-2.0-flash-exp              # Gemini model (default)
```

## 📖 Documentation Files

| File | Description |
|------|-------------|
| `README.md` | Main documentation |
| `STATISTICS.md` | Complete statistics guide |
| `STATISTICS_QUICK_START.md` | Quick statistics reference |
| `COMPLETE_WORKFLOW.md` | End-to-end migration guide |
| `GEMINI_AI.md` | Gemini AI setup |
| `DATABASE_OBJECTS.md` | Database objects migration |
| `CATCHUP_SYNC.md` | Catch-up synchronization |

## 🎯 Sync Modes

| Mode | Flag | What It Does |
|------|------|--------------|
| Initial | `--initial-sync` | Schema + data transfer only |
| Real-time | `--realtime-sync` | Monitor changes only |
| Full | `--full-sync` | Initial + catch-up + real-time |

## ⚠️ Troubleshooting

### Issue: Stats file not created
```bash
# Check you're in realtime sync mode
./rebuild-and-run.sh --realtime-sync

# Check file permissions
ls -la sync_operations_stats.json
```

### Issue: Gemini 503 errors
```bash
# Now includes automatic retry (3 attempts)
# Just wait - it will retry automatically
```

### Issue: Slow general_log queries
```bash
# Add index (if not exists)
mysql> CREATE INDEX idx_event_time ON mysql.general_log(event_time);

# Or increase poll interval
export POLL_INTERVAL_SECS=10
```

### Issue: Migration too slow
```bash
# Increase batch size
export BATCH_SIZE=500

# Skip database objects
unset GEMINI_API_KEY
```

## 📊 Migration Workflow

### 1️⃣ Monitor (24-48h)
```bash
./rebuild-and-run.sh --realtime-sync
# Let run for 1-2 days
# Press Ctrl+C when ready
```

### 2️⃣ Review Stats
```
📊 💡 BEST TIME TO SWITCH: Sunday 3 AM (80 operations)
```

### 3️⃣ Schedule Migration
Plan to run during identified quiet window

### 4️⃣ Execute
```bash
# During quiet window
./rebuild-and-run.sh --full-sync
```

## 🔍 Verification

```bash
# Check row counts in MySQL
mysql> SELECT COUNT(*) FROM table_name;

# Check row counts in PostgreSQL
psql> SELECT COUNT(*) FROM table_name;

# Should match!
```

## 🆘 Emergency Rollback

```bash
# Stop proxy
Ctrl+C

# Point application back to MySQL
# Edit app config, restart app

# MySQL still has all data!
```

## ⏱️ Timing Estimates

| Database Size | Initial Transfer | Catch-up | Real-time Setup |
|---------------|------------------|----------|-----------------|
| < 1 GB | 10-20 min | 1-2 min | Instant |
| 1-10 GB | 30-60 min | 2-5 min | Instant |
| 10-100 GB | 2-4 hours | 5-10 min | Instant |

*Times vary based on network, hardware, and data complexity*

## 💡 Pro Tips

1. **Always test in staging first**
2. **Monitor during low-activity window** (collect better stats)
3. **Back up both databases** before migration
4. **Keep stats file** for future reference
5. **Use --full-sync** for production (includes catch-up)

## 📈 Success Metrics

✅ All tables migrated  
✅ Row counts match  
✅ No errors in console for 5+ minutes  
✅ Real-time sync working (test INSERT/UPDATE)  
✅ Catch-up completed (< 3 iterations)  

## 🎓 Learning Path

1. **New user?** Read `STATISTICS_QUICK_START.md`
2. **Planning migration?** Read `COMPLETE_WORKFLOW.md`
3. **Need visualization?** Read `STATISTICS.md`
4. **Using Gemini?** Read `GEMINI_AI.md`
5. **Troubleshooting?** Check `README.md`

## 🔗 Quick Links

- **Full Docs**: [README.md](README.md)
- **Statistics**: [STATISTICS_QUICK_START.md](STATISTICS_QUICK_START.md)
- **Workflow**: [COMPLETE_WORKFLOW.md](COMPLETE_WORKFLOW.md)
- **Test**: `./test_stats_logger.sh`

---

**Need help?** Check the documentation or review the logs with `RUST_LOG=debug`

