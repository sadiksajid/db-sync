# Settings Page Layout Update - Master-Slave Architecture

## Summary
The Settings page has been completely redesigned to support a master-slave architecture with:
- **One database type selector** at the top (applies to all databases)
- **Source Database (Master)** on the right
- **Multiple Slave Databases** on the left with add/remove functionality

## New Architecture

### Visual Layout

```
┌────────────────────────────────────────────────────────────────┐
│                    🔧 Configuration                            │
├────────────────────────────────────────────────────────────────┤
│  Database Type: [MySQL ▼]                                     │
│  (All databases must be of the same type)                     │
├──────────────────────────────────┬─────────────────────────────┤
│  SLAVE DATABASES (LEFT)          │  SOURCE DATABASE (RIGHT)    │
│                                  │                             │
│  [+ Add Slave]                   │  Host: [____________]       │
│                                  │  Port: [3306______]         │
│  ┌─────────────────────────┐    │  Database: [______]         │
│  │ Slave #1                │    │  Username: [______]         │
│  │ Host: [____________]    │    │  Password: [******]         │
│  │ Port: [3306______]      │    │                             │
│  │ Database: [______]      │    │  [Test Source Connection]   │
│  │ Username: [______]      │    │                             │
│  │ Password: [******]      │    └─────────────────────────────┘
│  │ [Test Connection] [×]   │    
│  └─────────────────────────┘    
│                                  
│  ┌─────────────────────────┐    
│  │ Slave #2                │    
│  │ ...                     │    
│  └─────────────────────────┘    
└──────────────────────────────────┴─────────────────────────────┘
```

## Key Features

### 1. Single Database Type Selector
**Location:** Top of the page  
**Purpose:** Sets the database type for ALL databases (source and slaves)

- **Options:** MySQL or PostgreSQL
- **Auto Port Update:** When you change the type, all port fields update automatically
  - MySQL → Port 3306
  - PostgreSQL → Port 5432
- **Note:** "All databases (source and slaves) must be of the same type"

### 2. Slave Databases (Left Side)

#### Features:
- ✅ **Add Multiple Slaves:** Click "Add Slave" button to add new slave configurations
- ✅ **Remove Slaves:** Each slave has a delete (×) button in the top-right corner
- ✅ **Individual Testing:** Each slave has its own "Test Connection" button
- ✅ **Compact Form:** Smaller input fields optimized for multiple slaves
- ✅ **Scroll Support:** Container scrolls when you have many slaves (max height: 600px)

#### Each Slave Contains:
- Host address
- Port number
- Database name
- Username
- Password
- Test Connection button
- Remove button (×)

#### Empty State:
When no slaves are configured, shows:
- Empty state message: "No slave databases configured"
- Button: "Add Your First Slave"

### 3. Source Database (Right Side)

**Purpose:** The master database that will be replicated to all slaves

#### Fields:
- Host address
- Port number
- Database name
- Username
- Password
- Test Source Connection button

## JavaScript Functions

### New Functions Added:

1. **`addSlaveDatabase()`**
   - Adds a new slave configuration panel
   - Auto-increments slave counter
   - Applies default port based on database type
   - Updates visibility

2. **`removeSlaveDatabase(slaveId)`**
   - Removes a specific slave by ID
   - Updates visibility of empty state

3. **`updateSlavesVisibility()`**
   - Shows/hides the "no slaves" message
   - Shows/hides the slaves container

4. **`getAllSlaves()`**
   - Collects all slave configurations into an array
   - Returns array of slave objects

5. **`setupDatabaseTypeListeners()` (Updated)**
   - Listens to the single database type selector
   - Updates source port automatically
   - Updates all slave ports automatically

### Updated Functions:

1. **`loadConfig()` (Updated)**
   - Loads the single `db_type`
   - Loads source database configuration
   - Loads multiple slaves from `slaves` array
   - Backward compatible with old single-target config
   - Calls `updateSlavesVisibility()`

2. **`saveConfig()` (Updated)**
   - Validates at least one slave is configured
   - Validates all slaves have required fields
   - Saves with new structure:
     - `db_type`: Single type for all databases
     - `source_db_*`: Source database fields
     - `slaves`: Array of slave configurations
     - Backward compatibility: First slave is also saved as `target_db_*`

3. **`testConnection(type, slaveId)` (Updated)**
   - `type`: 'source' or 'slave'
   - `slaveId`: Required when testing a slave
   - Creates appropriate test config based on type
   - Updates the correct status element

## Configuration Structure

### New Format:

```javascript
{
  // Single database type for all
  db_type: "mysql" | "postgresql",
  
  // Source (Master) Database
  source_db_type: "mysql",
  source_db_host: "host.docker.internal",
  source_db_port: 3306,
  source_db_database: "master_db",
  source_db_username: "root",
  source_db_password: "password",
  
  // Slave Databases (Array)
  slaves: [
    {
      host: "slave1.example.com",
      port: 3306,
      database: "slave_db_1",
      username: "root",
      password: "password"
    },
    {
      host: "slave2.example.com",
      port: 3306,
      database: "slave_db_2",
      username: "root",
      password: "password"
    }
    // ... more slaves
  ],
  
  // Backward Compatibility (first slave)
  target_db_type: "mysql",
  target_db_host: "slave1.example.com",
  target_db_port: 3306,
  target_db_database: "slave_db_1",
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

### 1. Select Database Type
1. Go to Settings tab
2. At the top, select your database type (MySQL or PostgreSQL)
3. All port fields will auto-update

### 2. Configure Source (Master) Database
**On the right side:**
1. Enter the master database host
2. Port will be auto-filled (3306 for MySQL, 5432 for PostgreSQL)
3. Enter database name
4. Enter credentials (username and password)
5. Click "Test Source Connection" to verify

### 3. Add Slave Databases
**On the left side:**
1. Click "**+ Add Slave**" button
2. A new slave panel appears (Slave #1)
3. Fill in the slave database details:
   - Host
   - Port (auto-filled based on type)
   - Database name
   - Username
   - Password
4. Click "**Test Connection**" for this slave
5. Repeat for additional slaves (click "+ Add Slave" again)

### 4. Remove Slaves
- Click the **×** button in the top-right corner of any slave panel to remove it

### 5. Save Configuration
1. After configuring source and all slaves
2. Scroll down to the bottom
3. Click "**Save Configuration**" button
4. The system validates:
   - At least one slave is configured
   - All slaves have required fields (host, database, username)

### 6. Start Sync
- After saving, go to the Home tab
- Select sync mode
- Click "Start Sync"
- The system will replicate from the master to ALL configured slaves

## Validation Rules

### On Save:
✅ **At least one slave required**  
   - Error: "You must configure at least one slave database!"

✅ **All slaves must have required fields**  
   - Required: host, database, username
   - Optional: password (but recommended)
   - Error: "Slave #X is missing required fields (host, database, username)"

✅ **Database type consistency**  
   - All databases (source and slaves) use the same `db_type`
   - Enforced by single type selector

## Backward Compatibility

The new system maintains backward compatibility with old configurations:

1. **Loading Old Config:**
   - If `slaves` array is empty
   - And old `target_db_*` or `psql_db_*` fields exist
   - Creates one slave from the old target config

2. **Saving New Config:**
   - Saves with new `slaves` array
   - Also saves first slave as `target_db_*` for backward compatibility
   - Old code can still work with the first slave as target

## Visual Improvements

### Design Features:
- 🎨 **Gradient Header** for database type selector
- 🟢 **Color Coding:**
  - Primary (green/teal) for source/master
  - Warning (yellow) for slaves
- 📦 **Compact Slave Panels** with border and background
- 🗑️ **Delete Buttons** in top-right of each slave
- ✅ **Test Buttons** for each connection
- 📜 **Scrollable Container** for many slaves
- 🔘 **Responsive Layout:** Stacks on mobile, side-by-side on desktop

### Icons:
- 🛠️ Configuration title
- 🖥️ Server icon for database type
- 💾 Database icons for master and slaves
- 🧪 Test tube icons for connection tests
- ➕ Add circle for adding slaves
- 🗑️ Trash bin for removing slaves

## Testing Your Configuration

### Test Source Connection:
1. Configure source database
2. Click "Test Source Connection"
3. Watch for status: ✓ Connected or ✗ Failed

### Test Slave Connections:
1. Configure a slave
2. Click "Test Connection" for that specific slave
3. Each slave tests independently
4. Fix any connection issues before saving

### Common Issues:
- ⚠️ **"Connection Failed":** Check host, port, credentials
- ⚠️ **"Database not found":** Verify database name
- ⚠️ **"Access denied":** Check username/password
- ⚠️ **"Cannot connect":** Use `host.docker.internal` for host databases

## Example Configurations

### MySQL Master → 3 MySQL Slaves

```
Database Type: MySQL

Source (Master):
  Host: 192.168.1.10
  Port: 3306
  Database: production_db
  Username: root
  Password: ********

Slaves:
  Slave #1:
    Host: 192.168.1.20
    Port: 3306
    Database: backup_db_1
    Username: root
    Password: ********
  
  Slave #2:
    Host: 192.168.1.30
    Port: 3306
    Database: backup_db_2
    Username: root
    Password: ********
  
  Slave #3:
    Host: 192.168.1.40
    Port: 3306
    Database: analytics_db
    Username: root
    Password: ********
```

### PostgreSQL Master → 2 PostgreSQL Slaves

```
Database Type: PostgreSQL

Source (Master):
  Host: host.docker.internal
  Port: 5432
  Database: main_db
  Username: postgres
  Password: ********

Slaves:
  Slave #1:
    Host: slave1.example.com
    Port: 5432
    Database: replica_1
    Username: postgres
    Password: ********
  
  Slave #2:
    Host: slave2.example.com
    Port: 5432
    Database: replica_2
    Username: postgres
    Password: ********
```

## Benefits of New Layout

✅ **Clearer Hierarchy:** Master on right, slaves on left  
✅ **Single Source of Truth:** One database type selector  
✅ **Unlimited Slaves:** Add as many as needed  
✅ **Easy Management:** Add/remove slaves with one click  
✅ **Individual Testing:** Test each database independently  
✅ **Better Organization:** Grouped by role (master vs slaves)  
✅ **Responsive Design:** Works on desktop and mobile  
✅ **Visual Feedback:** Clear status for each connection  

## Files Modified

1. **`static/index.html`** - Settings section redesigned
   - New layout structure
   - Updated JavaScript functions
   - Added slave management functions

2. **`SETTINGS_LAYOUT_UPDATE.md`** - This documentation (new)

## Next Steps

1. **Access the Web UI:** http://localhost:5009
2. **Login or Create Account**
3. **Go to Settings Tab**
4. **Select Database Type** (top selector)
5. **Configure Source Database** (right side)
6. **Add Slave Databases** (left side, click "+ Add Slave")
7. **Test All Connections**
8. **Save Configuration**
9. **Start Sync** from Home tab

---

**Last Updated:** December 19, 2025  
**Version:** 2.0.0 (Master-Slave Architecture)  
**Architecture:** One Master → Multiple Slaves  
**Supported Types:** MySQL, PostgreSQL (same type for all)


