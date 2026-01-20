# Database Synchronization Proxy

A powerful, production-grade database synchronization tool that replicates data between MySQL or PostgreSQL databases. Built with Rust for high performance, reliability, and parallel processing.

[![Docker Hub](https://img.shields.io/badge/docker-sadiksajid%2Fdb--sync-blue?logo=docker)](https://hub.docker.com/r/sadiksajid/db-sync)
[![GitHub](https://img.shields.io/badge/github-container%20registry-green?logo=github)](https://github.com)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org)

## 📑 Table of Contents

- [Key Features](#-key-features)
- [Requirements](#-requirements)
- [Installation](#-installation)
- [Configuration](#️-configuration)
- [Usage](#-usage)
- [Docker Deployment](#-docker-deployment)
- [Troubleshooting](#️-troubleshooting)
- [API Endpoints](#-api-endpoints)

## 🚀 Key Features

<details open>
<summary><b>🔄 Multi-Slave Synchronization</b></summary>

<br>

- **Parallel Sync**: Synchronize to multiple slave databases simultaneously
- **Automatic Status Tracking**: Real-time monitoring of each slave database connection
- **Individual Slave Management**: Add, edit, or remove slave databases independently
- **Concurrent Processing**: Leverages Tokio for efficient parallel operations

</details>

<details>
<summary><b>⚡ Synchronization Modes</b></summary>

<br>

- **Full Sync** (MySQL only): Initial migration + continuous real-time monitoring
- **Initial Sync Only**: One-time schema and data transfer (MySQL & PostgreSQL)
- **Real-time Sync Only** (MySQL only): Continuous change monitoring via binlog
- **Reset Mode**: Optionally DROP and recreate slave databases before sync

</details>

<details>
<summary><b>📅 Automated Scheduling</b></summary>

<br>

- **Cron-Based Scheduler**: Schedule automatic syncs at specific times or intervals
- **User-Friendly Interface**: Define schedules with:
  - **Time Delay**: Run every X hours or days
  - **Exact Time**: Run at specific times (e.g., daily at 15:30, weekdays at 09:00)
- **Multiple Schedules**: Create unlimited scheduled tasks
- **Automatic Reset**: Scheduled syncs automatically reset slave databases for fresh data
- **Persistent**: Schedules continue running even after page refresh or system restart

</details>

<details>
<summary><b>💾 Database Support</b></summary>

<br>

- **MySQL to MySQL**: Complete synchronization with real-time monitoring
  - Uses `general_log` for change detection
  - Supports `mysqldump` for reliable data transfer
- **PostgreSQL to PostgreSQL**: Schema and data synchronization
  - Uses `pg_dump` and `psql` for production-grade transfers
  - Initial sync only (real-time not available)

</details>

<details>
<summary><b>🎨 Modern Web UI</b></summary>

<br>

- **Responsive Dashboard**: Works on desktop, tablet, and mobile devices
- **Live Statistics**: Real-time operation tracking with hourly analytics
- **Interactive Charts**: Beautiful visualizations of sync operations over time
- **User Authentication**: Secure login system with bcrypt password hashing
- **Persistent Configuration**: All settings saved in SQLite database
- **Dark Theme**: Modern, eye-friendly interface
- **Ad Integration**: Built-in ad display system

</details>

<details>
<summary><b>✅ Production Features</b></summary>

<br>

- **Data Integrity**: Uses standard tools (`mysqldump`, `pg_dump`) for reliable transfers
- **Error Handling**: Comprehensive error reporting with detailed logs
- **Connection Testing**: Test database connections before starting sync
- **Session Management**: Secure authentication with HTTP-only cookies
- **Docker Ready**: Full Docker and Docker Compose support

</details>

## 📋 Requirements

<details>
<summary><b>💾 Source Database</b></summary>

<br>

- **MySQL**: MySQL 5.7+ or MariaDB 10.2+
  - `general_log` must be enabled for real-time sync (tool enables it automatically with SUPER privilege)
  - User permissions: `SELECT` on source database, `SELECT` on `mysql.general_log`, `SUPER` privilege
- **PostgreSQL**: PostgreSQL 10+
  - User permissions: `SELECT` on source database

</details>

<details>
<summary><b>🔄 Slave Databases</b></summary>

<br>

- **MySQL**: MySQL 5.7+ or MariaDB 10.2+
  - User permissions: `CREATE DATABASE`, `DROP DATABASE`, `CREATE` tables, `INSERT`, `UPDATE`, `DELETE`
- **PostgreSQL**: PostgreSQL 10+
  - User permissions: `CREATE DATABASE`, `DROP DATABASE`, `CREATE` tables and schemas, `INSERT`, `UPDATE`, `DELETE`

</details>

<details>
<summary><b>🖥️ System Requirements</b></summary>

<br>

- Docker (recommended) or Rust 1.70+
- 512MB RAM minimum (1GB+ recommended for multiple slaves)
- Network access between source and all slave databases
- `mysql` client for MySQL sync
- `psql` and `pg_dump` clients for PostgreSQL sync

</details>

## 🔧 Installation

<details open>
<summary><b>🐳 Using Docker (Recommended)</b></summary>

<br>

1. **Clone the repository**
```bash
cd "DB sync"
```

2. **Start with Docker Compose**
```bash
docker-compose up -d
```

The Web UI will be available at http://localhost:5009

</details>

<details>
<summary><b>🏃 Using Docker Run</b></summary>

<br>

```bash
docker build -t db-sync-proxy .
docker run -d -p 5009:5009 -v ./db_sync_data:/app/data db-sync-proxy --web-ui
```

</details>

<details>
<summary><b>⚙️ From Source</b></summary>

<br>

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install database clients (Ubuntu/Debian)
sudo apt-get install mysql-client postgresql-client

# Build the project
cargo build --release

# Run with web UI
./target/release/db_sync_proxy --web-ui
```

</details>

## ⚙️ Configuration

### Initial Setup

1. **Open** http://localhost:5009 in your browser
2. **Create** first user account (appears on first run)
3. **Login** with your credentials

### Configure Master Database

1. Go to **Settings** tab
2. Select **Database Type** (MySQL or PostgreSQL)
3. Fill in **Source Database** details:
   - Host, Port, Database Name
   - Username, Password
4. Click **Test Connection** to verify

### Add Slave Databases

1. In the **Slave Databases** section
2. Fill in slave database details:
   - Host, Port, Database Name
   - Username, Password
3. Click **Add Slave Database**
4. Repeat for multiple slaves (supports unlimited slaves)
5. Each slave's status is displayed with color indicators

### Configure Sync Options

- **Batch Size**: Records per batch for data transfer (default: 100)
- **Poll Interval**: Seconds between change checks for real-time mode (default: 10)
- **Reset Database**: Enable to DROP and recreate slave databases before each sync

## 🎯 Usage

### Manual Synchronization

1. **Go to Home tab**
2. **Select Sync Mode**:
   - **Full Sync** (MySQL only): Initial migration + real-time monitoring
   - **Initial Sync Only**: One-time schema and data transfer
   - **Real-time Sync Only** (MySQL only): Monitor changes only
3. **Check Reset Database** if you want fresh slaves (⚠️ Warning: Deletes all data in slaves)
4. **Click Start Sync**
5. **Monitor progress** in real-time:
   - Live logs showing each operation
   - Slave database status indicators
   - Quick stats (tables synced, rows processed)

### Scheduled Synchronization

1. **Go to Schedules tab**
2. **Click Add Schedule**
3. **Choose Schedule Type**:
   
   **Time Delay:**
   - Repeat every X hours or days
   - Examples: Every 12 hours, Every 2 days
   
   **Exact Time:**
   - Run at specific time with recurrence
   - Examples: Daily at 15:30, Weekdays at 09:00, Every Monday at 08:00

4. **Name your schedule** (e.g., "Daily Backup", "Hourly Sync")
5. **Enable/Disable** with toggle switch
6. **Save** - Schedule runs automatically in background

**Important Notes:**
- Schedules run "Initial Sync Only" mode (Schema + Data)
- All configured slave databases are synced automatically
- Scheduled syncs always reset slave databases (fresh data)
- Schedules persist across restarts and page refreshes

### Monitoring

- **Home Tab**: Current status, active schedules, connected databases
- **Statistics Tab**: Detailed operation counts and success rates
- **Chart Tab**: Visual graphs of operations over time
- **Schedules Tab**: Manage all scheduled tasks

## 🔄 Sync Modes Explained

<details>
<summary><b>1️⃣ Full Sync (MySQL Only)</b></summary>

<br>

**What it does:**
- Phase 1: Schema migration and initial data transfer using `mysqldump`
- Phase 2: Catch-up sync (replays changes during transfer)
- Phase 3: Real-time monitoring via general_log

**Best for:**
- First-time MySQL migration
- Complete database replication
- Continuous synchronization

</details>

<details>
<summary><b>2️⃣ Initial Sync Only (MySQL & PostgreSQL)</b></summary>

<br>

**What it does:**
- Exports schema using `mysqldump` or `pg_dump`
- Transfers all data in batches
- Stops after completion

**Best for:**
- One-time migration
- Creating database snapshots
- Scheduled backups
- Both MySQL and PostgreSQL

</details>

<details>
<summary><b>3️⃣ Real-time Sync Only (MySQL Only)</b></summary>

<br>

**What it does:**
- Monitors MySQL general_log for changes
- Replicates INSERT, UPDATE, DELETE operations
- Assumes schema and initial data already exist

**Best for:**
- Resuming after interruption
- Ongoing replication
- After initial migration

</details>

## 📅 Scheduling Examples

### Time Delay Examples
```
Every 6 hours    → Runs every 6 hours from now
Every 12 hours   → Runs twice daily
Every 2 days     → Runs every 48 hours
```

### Exact Time Examples
```
Daily at 02:00           → Every day at 2 AM
Weekdays at 09:00        → Monday-Friday at 9 AM
Weekends at 23:00        → Saturday-Sunday at 11 PM
Every Monday at 08:00    → Weekly backup
Every Friday at 18:00    → End of week sync
```

**Cron Format (Auto-Generated):**
The system automatically converts your selections to cron expressions (6-field format with seconds).

## 📊 Statistics & Monitoring

### Real-Time Statistics
- **Hourly aggregation**: Operations grouped by hour
- **Operation breakdown**: INSERT, UPDATE, DELETE counts
- **Success rates**: Track successful vs failed operations
- **Per-slave tracking**: Monitor each slave individually
- **Persistent storage**: SQLite database for historical data

### Interactive Charts
- **Line graphs**: Operation trends over time
- **Color-coded**: Green (INSERT), Blue (UPDATE), Red (DELETE)
- **Smooth curves**: Easy-to-read visualizations
- **Hover tooltips**: Exact counts on demand
- **Last 24 hours**: Recent activity at a glance

### Slave Database Status
Each slave shows:
- **Connection status**: Connected / Disconnected indicator
- **Last sync time**: When last synchronized
- **Current state**: Syncing / Idle / Error

## 🔐 Security

### Authentication
- **First-time setup**: Create admin user on first launch
- **Password hashing**: bcrypt with secure salt (cost: 12)
- **Session management**: HTTP-only cookies with expiration
- **Protected routes**: All API endpoints require authentication
- **Logout**: Secure session termination

### User Management
- **Profile**: View and update email
- **Password change**: Secure password updates with re-hashing
- **Session timeout**: Automatic logout after inactivity

### Database Security
- **Credentials encryption**: Sensitive data stored securely in SQLite
- **No plaintext passwords**: All passwords hashed before storage
- **Secure connections**: Supports SSL/TLS for database connections

## 🐳 Docker Deployment

### Using Pre-built Images

The easiest way to get started is using pre-built images from Docker Hub or GitHub Container Registry:

<details open>
<summary><b>🐋 Docker Hub (Recommended)</b></summary>

<br>

**Pull and Run:**
```bash
# Pull the latest image
docker pull sadiksajid/db-sync:latest

# Run the container
docker run -d \
  --name db-sync-proxy \
  -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  -e RUST_LOG=info \
  sadiksajid/db-sync:latest --web-ui
```

**Docker Compose:**
```yaml
version: '3.8'

services:
  db-sync:
    image: sadiksajid/db-sync:latest
    ports:
      - "5009:5009"
    volumes:
      - ./db_sync_data:/app/data
    environment:
      - RUST_LOG=info
    command: ["--web-ui"]
    restart: unless-stopped
```

</details>

<details>
<summary><b>📦 GitHub Container Registry (GHCR)</b></summary>

<br>

**Pull and Run:**
```bash
# Pull the latest image
docker pull ghcr.io/GITHUB_USERNAME/REPO:latest

# Run the container
docker run -d \
  --name db-sync-proxy \
  -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  -e RUST_LOG=info \
  ghcr.io/GITHUB_USERNAME/REPO:latest --web-ui
```

**Docker Compose:**
```yaml
version: '3.8'

services:
  db-sync:
    image: ghcr.io/GITHUB_USERNAME/REPO:latest
    ports:
      - "5009:5009"
    volumes:
      - ./db_sync_data:/app/data
    environment:
      - RUST_LOG=info
    command: ["--web-ui"]
    restart: unless-stopped
```

</details>

> **Note**: Images are automatically built and published on every push to master. Each push creates a GitHub Release with version tag and pull commands.

### Available Image Tags
- `latest` - Latest stable release from master branch
- `v*` - Semantic version tags (e.g., `v1.0.0`, `v1.0.1`) - auto-generated
- `master` - Latest build from the master branch
- `1`, `1.0`, `1.0.0` - Major, minor, and patch version tags

> 💡 **Tip**: Check the [Releases](../../releases) page for all available versions with Docker pull commands

### Docker Compose (Build from Source)

```yaml
version: '3.8'

services:
  db-sync:
    build: .
    ports:
      - "5009:5009"
    volumes:
      - ./data:/app/data
    environment:
      - RUST_LOG=info
    command: ["--web-ui"]
    restart: unless-stopped
```

### Volume Persistence
The `/app/data` directory contains:
- `config.db`: SQLite database with all settings
- User credentials and sessions
- Operation statistics and logs
- Schedule configurations
- Connection history

### Environment Variables
- `RUST_LOG`: Logging level (debug, info, warn, error)
- All database config done via Web UI

## 🛠️ Troubleshooting

<details>
<summary><b>General_log Errors (MySQL)</b></summary>

<br>

**Problem**: `general_log is not enabled`

**Solution**: Tool enables it automatically with SUPER privilege. Manual enable:
```sql
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';
```

</details>

<details>
<summary><b>Database Type Mismatch</b></summary>

<br>

**Problem**: "Source and target database types must be the same"

**Solution**: Only same-type sync supported. All databases must be either MySQL or PostgreSQL.

</details>

<details>
<summary><b>Connection Errors</b></summary>

<br>

**Solutions**:
- Verify credentials in Settings
- Check network connectivity between container and databases
- Ensure user has required permissions
- Use **Test Connection** button
- Check firewall rules

</details>

<details>
<summary><b>Slave Not Syncing</b></summary>

<br>

**Solutions**:
- Verify slave is added in Settings and saved
- Check slave connection status on Home page
- Review logs for specific error messages
- Ensure Reset Database permissions if enabled

</details>

<details>
<summary><b>Schedule Not Running</b></summary>

<br>

**Solutions**:
- Verify schedule is enabled (toggle switch on)
- Check cron expression is valid
- Ensure at least one slave database is configured
- Review logs during scheduled time
- Check system time is correct

</details>

<details>
<summary><b>Chart Shows No Data</b></summary>

<br>

**Solutions**:
- Start real-time sync (not available for PostgreSQL)
- Make changes in source database
- Wait 2-4 seconds for detection
- Check Statistics tab for operation counts
- Verify general_log is enabled (MySQL)

</details>

## 📚 API Endpoints

<details>
<summary><b>🔐 Authentication</b></summary>

<br>

- `GET /api/auth/check` - Check if authenticated
- `GET /api/auth/has-users` - Check if users exist
- `POST /api/auth/setup` - Create first user
- `POST /api/auth/login` - User login
- `POST /api/auth/logout` - User logout

</details>

<details>
<summary><b>⚙️ Configuration</b></summary>

<br>

- `GET /api/config` - Get current configuration
- `POST /api/config` - Update configuration
- `POST /api/test-connection` - Test database connections

</details>

<details>
<summary><b>🔄 Sync Control</b></summary>

<br>

- `POST /api/sync/start` - Start synchronization
- `POST /api/sync/stop` - Stop synchronization
- `GET /api/status` - Get current sync status

</details>

<details>
<summary><b>📊 Monitoring</b></summary>

<br>

- `GET /api/stats` - Get sync statistics
- `GET /api/logs` - Get operation logs
- `GET /api/chart-stats` - Get hourly chart data

</details>

<details>
<summary><b>📅 Scheduling</b></summary>

<br>

- `GET /api/schedules` - Get all schedules
- `GET /api/schedules/active` - Get active schedules only
- `POST /api/schedules` - Create new schedule
- `POST /api/schedules/:id` - Update schedule
- `DELETE /api/schedules/:id` - Delete schedule
- `POST /api/schedules/:id/toggle` - Enable/disable schedule

</details>

<details>
<summary><b>👤 User Management</b></summary>

<br>

- `GET /api/profile/me` - Get user profile
- `POST /api/profile/update-email` - Update email
- `POST /api/profile/update-password` - Change password

</details>

## 🏗️ Architecture

### Technology Stack
- **Backend**: Rust with Tokio async runtime
- **Web Framework**: Axum 0.7
- **Database Drivers**: sqlx 0.7 (MySQL, PostgreSQL, SQLite)
- **Scheduler**: tokio-cron-scheduler
- **Frontend**: Vanilla JavaScript, Chart.js, Tailwind CSS
- **Icons**: Iconify
- **Storage**: SQLite for config, stats, and schedules

### Components
1. **Schema Readers**: Extract schema from MySQL or PostgreSQL
2. **Data Migrators**: Use `mysqldump` or `pg_dump` for reliable transfers
3. **Binlog Reader** (MySQL): Monitors general_log for real-time changes
4. **Database Writers**: Apply operations to multiple slaves in parallel
5. **Stats Logger**: Track operations in SQLite with hourly aggregation
6. **Scheduler Service**: Cron-based task scheduler with job management
7. **Web Server**: Axum HTTP server with middleware authentication
8. **Configuration Store**: SQLite-backed persistent configuration

### Data Flow
```
Master DB → Schema Reader → mysqldump/pg_dump → Slaves (Parallel)
    ↓                                              ↑
General Log → Binlog Reader → Event Queue → Writers (Parallel)
                    ↓
            Stats Logger → SQLite → Web API → Dashboard
                                      ↓
                            Scheduler → Automated Syncs
```

### Parallel Processing
- Multiple slave syncs run concurrently using `tokio::spawn`
- Each slave gets independent sync task
- Results aggregated for unified reporting
- Errors isolated per slave (one failure doesn't stop others)

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

MIT License - See LICENSE file for details

## 🙋 Support

For issues, questions, or feature requests:
- Open an issue on GitHub
- Check logs at `/api/logs` endpoint
- Enable `RUST_LOG=debug` for detailed logging
- Review Troubleshooting section above

## 🎉 Acknowledgments

Built with:
- **Rust** - Systems programming language
- **Tokio** - Async runtime
- **Axum** - Web framework
- **sqlx** - Database toolkit
- **Chart.js** - Data visualization
- **Tailwind CSS** - Styling framework
- **tokio-cron-scheduler** - Task scheduling

## 💡 Use Cases

<details>
<summary><b>👨‍💻 Development</b></summary>

<br>

- **Database Cloning**: Quickly clone production to staging
- **Test Data**: Keep test databases in sync with development
- **Team Collaboration**: Share database state across team members

</details>

<details>
<summary><b>🏭 Production</b></summary>

<br>

- **Read Replicas**: Create read-only replicas for load distribution
- **Backup**: Scheduled backups to separate servers
- **Disaster Recovery**: Maintain hot standby databases
- **Multi-Region**: Sync databases across geographical regions

</details>

<details>
<summary><b>📊 Analytics</b></summary>

<br>

- **Reporting Databases**: Separate analytics workload from production
- **Data Warehousing**: Feed data to warehouse systems
- **Business Intelligence**: Real-time data for BI tools

</details>

---

**Version**: 2.0.0  
**Last Updated**: December 2025

## ⚠️ Important Notes

- **Same-Type Only**: Synchronization only between identical database types (MySQL→MySQL or PostgreSQL→PostgreSQL)
- **Real-time Limitations**: Real-time sync only available for MySQL via general_log
- **PostgreSQL**: Supports initial sync only (no real-time monitoring)
- **Reset Mode**: USE WITH CAUTION - Drops and recreates slave databases (all data lost)
- **Scheduled Syncs**: Always use reset mode for data consistency
- **Performance**: Large databases may take time for initial sync
- **Testing**: Always test in development before production deployment
- **Permissions**: Ensure database users have all required privileges
- **Network**: Verify connectivity between sync server and all databases
- **Monitoring**: Review logs regularly for errors or warnings

## 🚧 Limitations

1. **Cross-Database Sync**: MySQL ↔ PostgreSQL not supported
2. **DDL Changes**: Schema changes during real-time sync may cause issues
3. **PostgreSQL Real-time**: Not implemented (architecture limitation)
4. **Binary Data**: Large binary objects may slow sync performance
5. **Conflict Resolution**: Last-write-wins strategy (no conflict detection)

## 🔮 Future Enhancements

- WAL-based PostgreSQL real-time sync
- Cross-database type synchronization (MySQL ↔ PostgreSQL)
- Bidirectional sync support
- Conflict detection and resolution
- Custom transformation rules
- Webhook notifications
- Prometheus metrics export
- CLI improvements
- Web UI enhancements

## 📝 Changelog

### Version 2.0.0 (December 2025)
- ✨ Added multi-slave parallel synchronization
- ✨ Implemented automated scheduling with cron
- ✨ Added reset database mode for fresh syncs
- ✨ Enhanced UI with responsive design
- ✨ Added active schedules display on home page
- 🔧 Improved error handling and reporting
- 🔧 Optimized data transfer with mysqldump/pg_dump
- 🗑️ Removed Gemini AI integration
- 🗑️ Removed PostgreSQL real-time sync (not production-ready)
- 🐛 Fixed cron expression parsing (6-field format)
- 🐛 Fixed schedule list refresh issues
- 📚 Updated documentation

### Version 1.0.0 (Initial Release)
- Basic MySQL and PostgreSQL sync
- Web UI with authentication
- Real-time monitoring (MySQL)
- Statistics and charts
