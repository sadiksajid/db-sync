# 🚀 Complete Migration Workflow with Statistics

## Overview

This guide shows the **recommended workflow** for migrating from MySQL to PostgreSQL with **zero data loss** and **minimal downtime** using statistics-driven planning.

## 📊 Workflow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 1: MONITORING (24-48 hours)                               │
│ Goal: Collect statistics to identify best migration window      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                   ./rebuild-and-run.sh --realtime-sync
                              │
                              ▼
                  [ Proxy monitors all operations ]
                  [ Logs: sync_operations_stats.json ]
                              │
                              ▼
                   Press Ctrl+C after 24-48 hours
                              │
                              ▼
            📊 View statistics and identify best window
            💡 BEST TIME TO SWITCH: Sunday 3 AM (80 ops)

┌─────────────────────────────────────────────────────────────────┐
│ PHASE 2: PLANNING                                                │
│ Goal: Review stats and schedule the migration                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
              Review hourly breakdown from stats
              Select the window with lowest activity
              Schedule migration for that window

┌─────────────────────────────────────────────────────────────────┐
│ PHASE 3: MIGRATION (During quiet window)                        │
│ Goal: Execute full migration with minimal impact                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                   ./rebuild-and-run.sh --full-sync
                              │
                              ▼
              ┌──────────────────────────────┐
              │ 1. Initial Sync              │
              │    - Schema migration        │
              │    - Data transfer           │
              │    - Views, Functions, etc.  │
              └──────────┬───────────────────┘
                         │
                         ▼
              ┌──────────────────────────────┐
              │ 2. Catch-up Sync             │
              │    - Replay changes during   │
              │      initial transfer        │
              └──────────┬───────────────────┘
                         │
                         ▼
              ┌──────────────────────────────┐
              │ 3. Real-time Sync            │
              │    - Monitor ongoing changes │
              │    - Keep databases in sync  │
              │    - Collect more stats      │
              └──────────┬───────────────────┘
                         │
                         ▼
                  Databases synchronized!

┌─────────────────────────────────────────────────────────────────┐
│ PHASE 4: CUTOVER                                                 │
│ Goal: Switch application to PostgreSQL                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                  Stop real-time sync
                  Update application config
                  Point app to PostgreSQL
                  Monitor for issues
                              │
                              ▼
                     ✅ Migration Complete!
```

## 📋 Detailed Steps

### Phase 1: Monitoring (24-48 Hours)

**Goal**: Understand your database activity patterns

**Steps**:

1. **Start monitoring**:
   ```bash
   ./rebuild-and-run.sh --realtime-sync
   ```

2. **Let it run**: 
   - Minimum: 24 hours (to see daily pattern)
   - Recommended: 48 hours (to see weekday pattern)
   - Ideal: 7 days (to see weekly pattern including weekend)

3. **Observe console output** (every 5 minutes):
   ```
   📊 2025-12-09 14:00:00 - 45 inserts, 23 updates, 5 deletes (total: 73)
   ```

4. **Stop and review** (Press Ctrl+C):
   ```
   📊 ═══════════════════════════════════════════════════════════════
   📊 HOURLY OPERATION STATISTICS
   📊 ═══════════════════════════════════════════════════════════════
   📊 2025-12-09 08:00:00 - 245 inserts, 120 updates, 15 deletes (total: 380)
   📊 2025-12-09 09:00:00 - 189 inserts, 95 updates, 8 deletes (total: 292)
   ...
   📊 💡 BEST TIME TO SWITCH: 2025-12-09 03:00:00 (only 45 operations)
   📊 ═══════════════════════════════════════════════════════════════
   ```

5. **Backup stats file**:
   ```bash
   cp sync_operations_stats.json stats_monitoring_$(date +%Y%m%d).json
   ```

### Phase 2: Planning

**Goal**: Make data-driven migration decision

**Steps**:

1. **Analyze the statistics**:
   - Identify the hour with **lowest operation count**
   - Consider your **business constraints** (e.g., can't do it during business hours)
   - Look for **patterns** (e.g., weekends are quieter)

2. **Choose migration window**:
   - Example: "Sunday 3 AM" if stats show 45 ops/hour
   - Allow 2-4 hours for the process
   - Notify stakeholders

3. **Prepare checklist**:
   ```
   ☐ MySQL credentials verified
   ☐ PostgreSQL credentials verified
   ☐ Gemini API key ready (if using database objects)
   ☐ Disk space checked (PostgreSQL needs space)
   ☐ Backup both databases
   ☐ Application maintenance window scheduled
   ☐ Rollback plan documented
   ```

### Phase 3: Migration (During Quiet Window)

**Goal**: Execute the migration

**Steps**:

1. **Final backup**:
   ```bash
   # MySQL
   mysqldump -u root -p database_name > backup_mysql_$(date +%Y%m%d_%H%M%S).sql
   
   # PostgreSQL (if it has existing data)
   pg_dump -U postgres database_name > backup_pg_$(date +%Y%m%d_%H%M%S).sql
   ```

2. **Start full sync**:
   ```bash
   ./rebuild-and-run.sh --full-sync
   ```

3. **Monitor progress**:
   - Watch console for errors
   - Check PostgreSQL for data
   - Verify record counts match

4. **Verify real-time sync is working**:
   - Make a test INSERT in MySQL
   - Verify it appears in PostgreSQL
   - Check console logs

5. **Let real-time sync run** while you prepare cutover

### Phase 4: Cutover

**Goal**: Switch application to PostgreSQL

**Steps**:

1. **Put application in maintenance mode**:
   ```bash
   # Your application-specific command
   ./app maintenance on
   ```

2. **Wait for final sync** (should be fast - you chose a quiet window!):
   - Check console: "No changes detected"
   - Verify row counts match

3. **Stop the proxy**:
   ```bash
   # Press Ctrl+C
   # Or send SIGTERM to the container
   docker stop <container_id>
   ```

4. **Update application configuration**:
   ```diff
   # Before
   - DB_HOST=mysql.example.com
   - DB_PORT=3306
   + DB_HOST=postgres.example.com
   + DB_PORT=5432
   ```

5. **Start application**:
   ```bash
   ./app start
   ```

6. **Monitor for issues**:
   - Check application logs
   - Verify writes go to PostgreSQL
   - Test critical user flows
   - Monitor performance

7. **Take maintenance mode off**:
   ```bash
   ./app maintenance off
   ```

8. **Celebrate!** 🎉

## 🔄 Rollback Plan

If something goes wrong:

### Option 1: Quick Rollback (Within 1 Hour)

1. Stop application
2. Revert database configuration to MySQL
3. Restart application
4. Investigate issue

### Option 2: Keep Both Running

1. Leave real-time sync running
2. Point application back to MySQL
3. PostgreSQL will catch up automatically
4. Fix issues and try cutover again

## ⏱️ Timing Examples

### Small Database (< 1 GB, < 1M rows)

| Phase | Duration | When |
|-------|----------|------|
| Monitoring | 24 hours | Monday-Tuesday |
| Planning | 1 hour | Tuesday afternoon |
| Migration | 30 minutes | Wednesday 3 AM |
| Cutover | 15 minutes | Wednesday 3:30 AM |
| **Total** | **~26 hours** | |

### Medium Database (1-10 GB, 1-10M rows)

| Phase | Duration | When |
|-------|----------|------|
| Monitoring | 48 hours | Monday-Wednesday |
| Planning | 2 hours | Wednesday afternoon |
| Migration | 2 hours | Saturday 2 AM |
| Cutover | 30 minutes | Saturday 4:30 AM |
| **Total** | **~52 hours** | |

### Large Database (> 10 GB, > 10M rows)

| Phase | Duration | When |
|-------|----------|------|
| Monitoring | 7 days | Week 1 |
| Planning | 1 day | Week 2 Monday |
| Migration | 4-8 hours | Week 2 Sunday 12 AM |
| Cutover | 1 hour | Week 2 Sunday 9 AM |
| **Total** | **~8 days** | |

## 📊 Using Statistics Effectively

### During Monitoring Phase

**Every 5 minutes**, you'll see:
```
📊 2025-12-09 14:00:00 - 45 inserts, 23 updates, 5 deletes (total: 73)
```

**What to look for**:
- ✅ High numbers during business hours → Expected
- ✅ Low numbers during off-hours → Good migration window
- ⚠️ Unexpectedly high numbers → Investigate (batch jobs?)
- ⚠️ Consistent high numbers 24/7 → May need different strategy

### During Migration Phase

**Watch for**:
- Catch-up sync iterations: Should be 1-2 (not 10!)
- Real-time sync speed: Should apply changes within seconds
- Operation counts: Should match monitoring phase

**Red flags**:
- ❌ Catch-up keeps finding changes → Too much activity, abort and reschedule
- ❌ Real-time sync lagging → Check PostgreSQL performance
- ❌ Many errors in console → Investigate before continuing

## 🎯 Success Criteria

Before proceeding to cutover:

- ✅ All tables migrated successfully
- ✅ Row counts match between MySQL and PostgreSQL
- ✅ Real-time sync is working (test INSERT/UPDATE/DELETE)
- ✅ No errors in console for 5+ minutes
- ✅ Catch-up sync completed (< 3 iterations)
- ✅ Application tested against PostgreSQL (in test environment)
- ✅ Rollback plan ready
- ✅ Stakeholders notified

## 💡 Pro Tips

### Tip 1: Test First
Run the full workflow in a **staging environment** before production:
```bash
# Point to staging databases
export DB_DATABASE=staging_db
export PSQL_DB_DATABASE=staging_db_pg
./rebuild-and-run.sh --full-sync
```

### Tip 2: Monitor Both Databases
Keep monitoring MySQL even after cutover:
- Some queries might still hit MySQL (old connections, etc.)
- Useful for debugging issues
- Can restart real-time sync if needed

### Tip 3: Archive Statistics
Keep the stats file for future reference:
```bash
mkdir -p migration_history
cp sync_operations_stats.json migration_history/stats_$(date +%Y%m%d_%H%M%S).json
```

### Tip 4: Use Quiet Window for Testing
If your identified window is "Sunday 3 AM", do a **test migration** the week before at the same time to validate timing.

### Tip 5: Have a Champion
Designate someone to:
- Monitor the console during migration
- Make go/no-go decision
- Execute rollback if needed

## 📚 Additional Resources

- **Quick Start**: [STATISTICS_QUICK_START.md](STATISTICS_QUICK_START.md)
- **Full Statistics Guide**: [STATISTICS.md](STATISTICS.md)
- **Database Objects**: [DATABASE_OBJECTS.md](DATABASE_OBJECTS.md)
- **Gemini AI Setup**: [GEMINI_AI.md](GEMINI_AI.md)
- **Troubleshooting**: [README.md](README.md#troubleshooting)

## 🆘 Help!

### Issue: Stats show no quiet window

**Solution**: Consider:
- Pausing batch jobs during migration
- Scaling out reads to reduce write pressure
- Staged migration (migrate table-by-table)
- Blue-green deployment with separate databases

### Issue: Migration taking too long

**Reasons**:
- Large database → Use higher BATCH_SIZE
- Slow network → Consider running proxy closer to databases
- Many database objects → Skip Gemini AI (faster but less accurate)

**Solutions**:
```bash
# Increase batch size
export BATCH_SIZE=1000

# Skip database objects
unset GEMINI_API_KEY

# Run again
./rebuild-and-run.sh --full-sync
```

### Issue: Real-time sync lagging

**Check**:
1. PostgreSQL write performance
2. Network latency
3. `general_log` query performance (should be fast with index)

**Fix**:
```bash
# In MySQL, add index if not exists
CREATE INDEX idx_event_time ON mysql.general_log(event_time);

# Increase poll interval to reduce load
export POLL_INTERVAL_SECS=10
```

## ✅ Checklist

Print this and check off as you go:

```
PHASE 1: MONITORING
☐ Started real-time sync
☐ Let run for 24-48 hours
☐ Reviewed statistics
☐ Identified best migration window
☐ Backed up stats file

PHASE 2: PLANNING
☐ Chose migration date/time
☐ Notified stakeholders
☐ Prepared rollback plan
☐ Tested in staging (optional but recommended)
☐ Verified credentials and connectivity

PHASE 3: MIGRATION
☐ Backed up both databases
☐ Started full sync at chosen time
☐ Monitored console for errors
☐ Verified catch-up completed
☐ Confirmed real-time sync working
☐ Tested INSERT/UPDATE/DELETE

PHASE 4: CUTOVER
☐ Application in maintenance mode
☐ Final verification (row counts, etc.)
☐ Stopped proxy
☐ Updated application config
☐ Restarted application
☐ Verified writes to PostgreSQL
☐ Removed maintenance mode
☐ Monitored for 1+ hour
☐ 🎉 Celebrated success!
```

---

**You've got this!** 💪 The statistics feature gives you the data you need to make smart decisions. Follow this workflow and your migration will be smooth and successful! 🚀

