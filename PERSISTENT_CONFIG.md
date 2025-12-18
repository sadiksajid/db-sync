# 💾 Persistent Configuration with SQLite

## ✅ **Feature Implemented**

Database credentials are now saved to a SQLite database and persist across restarts!

## 🎯 **How It Works**

- When you save configuration in the Web UI, it's stored in `data/config.db`
- On restart, your last saved configuration is automatically loaded
- No need to re-enter credentials every time!

## 📁 **File Location**

**Local Development:**
```
data/config.db
```

**Docker (with volume mount):**
```
./mysql_psql_data/config.db
```

## 🚀 **Usage**

### **Running Locally:**

```bash
# Start the web UI
RUST_LOG=info cargo run --release -- --web-ui
```

Your configuration is saved to `data/config.db`

### **Running with Docker:**

```bash
# Create data directory
mkdir -p ./mysql_psql_data

# Run with volume mount
docker run --rm -p 5009:5009 \
  -v "$(pwd)/mysql_psql_data:/app/data" \
  mysql_psql_proxy:latest \
  --web-ui
```

Or use the script:
```bash
./run-web-ui.sh
```

## 📊 **What's Saved**

The following configuration is persisted:

✅ **MySQL Settings:**
- Host, Port, Database
- Username, Password

✅ **PostgreSQL Settings:**
- Host, Port, Database
- Username, Password

✅ **Sync Settings:**
- Batch Size
- Poll Interval (seconds)

✅ **AI Settings:**
- Gemini API Key (optional)
- Gemini Model

## 🔒 **Security Notes**

⚠️ **Important:** Passwords are stored in plain text in the SQLite database!

**Recommendations:**
1. Keep the `data/` or `mysql_psql_data/` directory secure
2. Use file permissions to restrict access:
   ```bash
   chmod 700 ./mysql_psql_data
   ```
3. Don't commit the data directory to git
4. For production, consider using environment variables or a secrets manager

## 🔄 **Workflow**

1. **First Time:**
   - Start Web UI
   - Fill in database credentials
   - Click "💾 Save Configuration"
   - Message: "✅ Configuration saved (persisted to database)"

2. **Subsequent Starts:**
   - Start Web UI
   - Configuration automatically loads!
   - You can start sync immediately

3. **Update Configuration:**
   - Change any settings
   - Click "💾 Save Configuration"
   - New settings are saved

## 🗑️ **Clear Saved Configuration**

To start fresh:

```bash
# Local
rm -f data/config.db

# Docker volume
rm -f ./mysql_psql_data/config.db
```

## 📝 **Database Schema**

```sql
CREATE TABLE config (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Only one config allowed
    db_host TEXT NOT NULL,
    db_port INTEGER NOT NULL,
    db_database TEXT NOT NULL,
    db_username TEXT NOT NULL,
    db_password TEXT NOT NULL,
    psql_db_host TEXT NOT NULL,
    psql_db_port INTEGER NOT NULL,
    psql_db_database TEXT NOT NULL,
    psql_db_username TEXT NOT NULL,
    psql_db_password TEXT NOT NULL,
    batch_size INTEGER NOT NULL DEFAULT 100,
    poll_interval_secs INTEGER NOT NULL DEFAULT 10,
    gemini_api_key TEXT,
    gemini_model TEXT NOT NULL DEFAULT 'gemini-2.0-flash-exp',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## 🐛 **Troubleshooting**

### Configuration Not Loading

```bash
# Check if database exists
ls -lah data/config.db

# Verify database content (requires sqlite3)
sqlite3 data/config.db "SELECT * FROM config;"
```

### Permission Denied

```bash
# Fix permissions
chmod 755 data/
chmod 644 data/config.db
```

### Docker Volume Issues

```bash
# Check volume mount
docker inspect CONTAINER_ID | grep -A 5 "Mounts"

# Verify data directory exists
ls -lah ./mysql_psql_data/
```

## ✨ **Benefits**

✅ Save time - no re-entering credentials  
✅ Multiple environments - different configs per directory  
✅ Easy backup - just copy the `data/` directory  
✅ Portable - SQLite file is self-contained  
✅ No external dependencies - pure SQLite  

## 🔮 **Future Enhancements**

Planned improvements:
- [ ] Encrypt passwords in database
- [ ] Multiple configuration profiles
- [ ] Import/export configurations as JSON
- [ ] Configuration validation on load
- [ ] Automatic backup before overwrite

---

**Status:** ✅ Working in local development  
**Docker:** 🚧 Under investigation (works locally)

