# 🚀 Quick Start Guide

## Your Web UI is Already Running!

**URL:** http://localhost:5009

## First Time Setup (30 seconds)

1. **Open Browser** → http://localhost:5009

2. **Create Account** (appears on first visit)
   - Enter email and password
   - Click "Create Account"

3. **Go to Settings Tab**
   - Click the ⚙️ Settings icon

4. **Configure Source Database**
   - Database Type: `mysql` or `postgresql`
   - Host: Your database host
   - Port: Your database port
   - Database: Database name
   - Username: Database user
   - Password: Database password

5. **Configure Target Database**
   - Database Type: **MUST MATCH SOURCE TYPE**
   - Host: Target database host
   - Port: Target database port
   - Database: Target database name
   - Username: Target database user
   - Password: Target database password

6. **Save Configuration**
   - Click "Save Configuration"

7. **Start Sync**
   - Go to Home tab
   - Choose sync mode:
     - **Initial Sync Only** - One-time data copy (recommended for first test)
     - **Real-time Sync** - Monitor changes (MySQL only)
     - **Full Sync** - Both initial + real-time (MySQL only)
   - Click "Start Sync"

8. **Monitor Progress**
   - Watch the Logs tab for real-time updates
   - Check Statistics tab for operation counts
   - View Chart tab for visual graphs

## Example Configurations

### MySQL to MySQL (Same Server, Different Ports)

**Source:**
```
Type: mysql
Host: 127.0.0.1
Port: 3306
Database: sourcedb
Username: root
Password: yourpassword
```

**Target:**
```
Type: mysql
Host: 127.0.0.1
Port: 3307
Database: targetdb
Username: root
Password: yourpassword
```

### PostgreSQL to PostgreSQL

**Source:**
```
Type: postgresql
Host: 127.0.0.1
Port: 5432
Database: sourcedb
Username: postgres
Password: yourpassword
```

**Target:**
```
Type: postgresql
Host: 127.0.0.1
Port: 5433
Database: targetdb
Username: postgres
Password: yourpassword
```

## Your Available Databases

Based on port scanning, you have:
- **MySQL** on port **3307**
- **PostgreSQL** on port **5433**

## Common Issues

### "Source and target database types must be the same"
- ✅ **Solution:** Make sure both source and target use same type (both MySQL or both PostgreSQL)

### Connection Failed
- ✅ Check database is running: `netstat -tln | grep PORT`
- ✅ Verify credentials
- ✅ Check firewall rules

### General_log Not Enabled (MySQL real-time sync)
- ✅ User needs SUPER privilege
- ✅ Tool tries to enable it automatically

## Testing Tip

**Start small!** Test with a database that has:
- Few tables (1-5)
- Small data (< 1000 rows)
- This lets you verify everything works before syncing production data

## Need Help?

- **Full Documentation:** See `README.md`
- **Testing Guide:** See `TESTING.md`
- **Run Tests:** `./test-config.sh`

## Stop the Web UI

```bash
# Find the process
ps aux | grep db_sync_proxy

# Kill it
kill <PID>

# Or use pkill
pkill -f db_sync_proxy
```

## Restart the Web UI

```bash
cd "/home/seddek/sadik/projects/DB sync"
./target/release/db_sync_proxy --web-ui
```

---

**Ready to go! Open http://localhost:5009 and start syncing! 🎉**

