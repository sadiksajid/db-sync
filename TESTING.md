# Testing Guide for DB Sync Proxy

## ✅ Configuration Tests Passed

All basic configuration tests have passed successfully:
- ✅ Binary compilation
- ✅ Help command
- ✅ Environment variable parsing
- ✅ MySQL-to-MySQL configuration
- ✅ PostgreSQL-to-PostgreSQL configuration
- ✅ Type mismatch validation
- ✅ Web UI startup

## 🎯 Quick Test with Web UI

### Step 1: Start the Web UI

```bash
cd "/home/seddek/sadik/projects/DB sync"
./target/release/db_sync_proxy --web-ui
```

### Step 2: Open Browser

Navigate to: **http://localhost:5009**

### Step 3: Configure Databases

You have databases running on:
- **MySQL**: Port 3307
- **PostgreSQL**: Port 5433

#### Example Configuration (MySQL to MySQL)

**Source Database:**
- Type: MySQL
- Host: 127.0.0.1
- Port: 3307 (or your source MySQL port)
- Database: your_source_db
- Username: root
- Password: your_password

**Target Database:**
- Type: MySQL (must match source)
- Host: 127.0.0.1
- Port: 3307 (or different MySQL instance)
- Database: your_target_db
- Username: root
- Password: your_password

### Step 4: Test Sync

1. Click "Start Sync"
2. Choose "Initial Sync Only" for first test
3. Monitor logs in the Web UI

## 🧪 CLI Testing

### Test 1: MySQL to MySQL (if you have two MySQL instances)

```bash
export SOURCE_DB_TYPE=mysql
export TARGET_DB_TYPE=mysql

# Source
export SOURCE_DB_HOST=127.0.0.1
export SOURCE_DB_PORT=3306
export SOURCE_DB_DATABASE=source_db
export SOURCE_DB_USERNAME=root
export SOURCE_DB_PASSWORD=password

# Target
export TARGET_DB_HOST=127.0.0.1
export TARGET_DB_PORT=3307
export TARGET_DB_DATABASE=target_db
export TARGET_DB_USERNAME=root
export TARGET_DB_PASSWORD=password

# Run initial sync
./target/release/db_sync_proxy --initial-sync
```

### Test 2: PostgreSQL to PostgreSQL (if you have two PostgreSQL instances)

```bash
export SOURCE_DB_TYPE=postgresql
export TARGET_DB_TYPE=postgresql

# Source
export SOURCE_DB_HOST=127.0.0.1
export SOURCE_DB_PORT=5432
export SOURCE_DB_DATABASE=source_db
export SOURCE_DB_USERNAME=postgres
export SOURCE_DB_PASSWORD=password

# Target
export TARGET_DB_HOST=127.0.0.1
export TARGET_DB_PORT=5433
export TARGET_DB_DATABASE=target_db
export TARGET_DB_USERNAME=postgres
export TARGET_DB_PASSWORD=password

# Run initial sync
./target/release/db_sync_proxy --initial-sync
```

### Test 3: Real-time Sync (MySQL only)

```bash
# After initial sync, test real-time monitoring
./target/release/db_sync_proxy --realtime-sync
```

## 📊 What to Verify

### Initial Sync Test
- ✅ Schema copied to target database
- ✅ All tables created
- ✅ Data transferred correctly
- ✅ Row counts match

### Real-time Sync Test (MySQL only)
- ✅ INSERT operations replicated
- ✅ UPDATE operations replicated
- ✅ DELETE operations replicated
- ✅ Operations appear in Web UI logs
- ✅ Statistics tracked

## 🔧 Troubleshooting

### Connection Errors
- Verify database credentials
- Check firewall/network access
- Ensure database is running: `netstat -tln | grep PORT`

### Type Mismatch Error
This is **expected** behavior if source and target types don't match:
```
Source and target database types must be the same
```

### Permission Errors (MySQL real-time sync)
For MySQL real-time sync, you need:
- SUPER privilege to enable general_log
- SELECT on mysql.general_log

## 🎉 Success Indicators

### Initial Sync Success
```
✓ Database connections established
✓ Found N tables
✓ Created N tables
✓ Data transfer complete
✓ Initial sync completed
```

### Real-time Sync Success (MySQL)
```
✓ Real-time sync is now active and monitoring changes
📊 Make changes in MySQL to see them replicated
🔄 INSERT → table_name (5 columns)
✓ Event processed successfully
```

## 📝 Test Results Summary

Run the test script to see configuration validation:
```bash
./test-config.sh
```

## 🚀 Next Steps

1. **Test with actual databases** - Use the Web UI for easiest testing
2. **Monitor statistics** - Check the Chart tab in Web UI
3. **Test real-time sync** - Make changes in source DB and verify replication
4. **Performance testing** - Test with larger datasets

## ⚠️ Important Notes

- **Same-Type Only**: MySQL→MySQL or PostgreSQL→PostgreSQL (no cross-database sync)
- **Real-time Sync**: Currently only available for MySQL
- **PostgreSQL Real-time**: Coming soon
- **Testing**: Always test in development environment first

