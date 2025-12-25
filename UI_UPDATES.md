# Web UI Updates - Same-Type Database Synchronization

## Summary
The Web UI has been updated to support same-type database synchronization (MySQL→MySQL or PostgreSQL→PostgreSQL) with proper database type selection.

## Changes Made

### 1. Branding Updates
- **Page Title**: Changed from "MySQL to PostgreSQL Sync" to "Database Sync Proxy"
- **Header**: Updated main heading and subtitle to reflect same-type sync capability
- **Login Page**: Updated all branding references to "Database Sync Proxy"

### 2. Settings Page Enhancements

#### Source Database Configuration
- ✅ **New Field**: Database Type dropdown (MySQL / PostgreSQL)
- ✅ Updated field IDs with `source_` prefix:
  - `source_db_type` - Database type selector
  - `source_db_host` - Host address
  - `source_db_port` - Port number (auto-updates based on DB type)
  - `source_db_database` - Database name
  - `source_db_username` - Username
  - `source_db_password` - Password
- ✅ Test button now labeled "Test Source Connection"

#### Target Database Configuration
- ✅ **New Field**: Database Type dropdown with warning: "(Must match source!)"
- ✅ Updated field IDs with `target_` prefix:
  - `target_db_type` - Database type selector
  - `target_db_host` - Host address
  - `target_db_port` - Port number (auto-updates based on DB type)
  - `target_db_database` - Database name
  - `target_db_username` - Username
  - `target_db_password` - Password
- ✅ Test button now labeled "Test Target Connection"

### 3. Smart Features

#### Auto Port Selection
When you change the database type, the port automatically updates:
- **MySQL** → Port 3306
- **PostgreSQL** → Port 5432

#### Type Validation
The UI validates that source and target database types match:
- ✅ If types don't match, shows error: "Error: Source and target database types must be the same!"
- ✅ Prevents configuration save until types match

#### Backward Compatibility
The configuration loader supports both old and new field names:
- Old `db_*` fields map to new `source_*` fields
- Old `psql_db_*` fields map to new `target_*` fields

### 4. Updated Configuration Structure

**New Configuration Format:**
```javascript
{
  // Source Database
  source_db_type: "mysql" | "postgresql",
  source_db_host: "host.docker.internal",
  source_db_port: 3306 | 5432,
  source_db_database: "my_source_db",
  source_db_username: "root",
  source_db_password: "password",
  
  // Target Database
  target_db_type: "mysql" | "postgresql",
  target_db_host: "host.docker.internal",
  target_db_port: 3306 | 5432,
  target_db_database: "my_target_db",
  target_db_username: "root",
  target_db_password: "password",
  
  // Other settings
  batch_size: 100,
  poll_interval_secs: 10,
  sync_mode: "full-sync",
  gemini_api_key: null,
  gemini_model: "gemini-2.0-flash-exp"
}
```

## How to Use

### 1. Access the Web UI
Open your browser to: **http://localhost:5009**

### 2. Create Account or Login
First-time users create an account, returning users login.

### 3. Configure Databases

#### For MySQL → MySQL Sync:
1. Go to **Settings** tab
2. **Source Database**:
   - Type: `MySQL`
   - Host: Your source MySQL host
   - Port: `3306` (auto-filled)
   - Database: Source database name
   - Credentials: MySQL username/password
3. **Target Database**:
   - Type: `MySQL` (must match source!)
   - Host: Your target MySQL host
   - Port: `3306` (auto-filled)
   - Database: Target database name
   - Credentials: MySQL username/password
4. Click "Test Source Connection" and "Test Target Connection"
5. Click "Save Configuration"

#### For PostgreSQL → PostgreSQL Sync:
1. Go to **Settings** tab
2. **Source Database**:
   - Type: `PostgreSQL`
   - Host: Your source PostgreSQL host
   - Port: `5432` (auto-filled)
   - Database: Source database name
   - Credentials: PostgreSQL username/password
3. **Target Database**:
   - Type: `PostgreSQL` (must match source!)
   - Host: Your target PostgreSQL host
   - Port: `5432` (auto-filled)
   - Database: Target database name
   - Credentials: PostgreSQL username/password
4. Click "Test Source Connection" and "Test Target Connection"
5. Click "Save Configuration"

### 4. Start Sync
1. Go to **Home** tab
2. Select sync mode:
   - **Initial Sync**: One-time data copy
   - **Realtime Sync**: Continuous synchronization (MySQL only)
   - **Full Sync**: Initial + Realtime
   - **Catchup Sync**: Resume from last position
3. Click "Start Sync"

### 5. Monitor Progress
- **Logs** tab: Real-time operation logs
- **Statistics** tab: Sync statistics and counts
- **Chart** tab: Visual graphs of operations

## Technical Details

### Files Modified
1. `static/index.html` - Main dashboard UI
2. `static/login.html` - Login page
3. `docker-compose.yml` - Environment variables
4. `Dockerfile` - Binary name

### JavaScript Functions Updated
- `loadConfig()` - Now loads source/target fields
- `saveConfig()` - Validates matching types, saves new format
- `testConnection()` - Tests source or target independently
- `setupDatabaseTypeListeners()` - Auto-updates ports

### Event Listeners
- Database type dropdowns trigger port updates
- Configuration validation on save

## Troubleshooting

### Port Not Auto-Updating
- Refresh the page
- The event listeners are set up after login

### Type Mismatch Error
- Ensure both source and target database types are the same
- You cannot sync MySQL → PostgreSQL (by design)

### Connection Test Fails
- Verify host is accessible (use `host.docker.internal` for host databases)
- Check credentials
- Ensure database exists
- Check firewall rules

### Old Configuration Not Loading
- Old field names are automatically mapped to new ones
- If issues persist, re-enter configuration manually

## Support

For issues or questions:
1. Check `docker compose logs -f` for errors
2. Review `README.md` for detailed documentation
3. See `TESTING.md` for testing procedures
4. Check `QUICK_START.md` for quick setup guide

---

**Last Updated**: December 19, 2025  
**Version**: 1.0.0 (Same-Type Sync)

