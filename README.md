# Database Synchronization Proxy

A powerful, real-time database synchronization tool that syncs data between identical database types. Built with Rust for high performance and reliability.

## 🚀 Features

### Core Synchronization
- **Same-Type Sync**: Synchronize MySQL-to-MySQL or PostgreSQL-to-PostgreSQL
- **Schema Migration**: Automatic copying of database schemas
- **Initial Data Transfer**: Bulk transfer of existing data with configurable batch sizes
- **Real-time Sync** (MySQL only): Continuous monitoring and replication of INSERT, UPDATE, DELETE operations
- **Catch-up Sync** (MySQL only): Ensures no data is lost during initial migration by replaying changes
- **Three Sync Modes**: Choose between Full Sync, Initial Sync Only, or Real-time Sync Only

### Database Support
- **MySQL to MySQL**: Complete synchronization between MySQL databases
  - Supports real-time change detection using general_log
  - Catch-up sync to replay missed changes
- **PostgreSQL to PostgreSQL**: Schema and data synchronization
  - Initial sync and data transfer
  - Real-time sync (coming soon)

### Web UI
- **Modern Dashboard**: Intuitive web interface for configuration and monitoring
- **Live Statistics**: Real-time operation tracking with hourly analytics
- **Interactive Charts**: Beautiful line graphs showing sync operations over time
- **User Authentication**: Secure login system with session management
- **Persistent Configuration**: All settings saved in SQLite database

### Monitoring & Analytics
- **Operation Statistics**: Track INSERT, UPDATE, DELETE counts by hour
- **Success/Failure Tracking**: Monitor operation success rates
- **Live Logs**: Real-time logging of all sync activities
- **SQLite Storage**: Persistent statistics with queryable data

## 📋 Requirements

### Source Database
- **MySQL**: MySQL 5.7+ or MariaDB 10.2+
- `general_log` must be enabled (tool enables it automatically if you have SUPER privilege)
  - User with permissions: `SELECT` on source database, `SELECT` on `mysql.general_log`, `SUPER` privilege
- **PostgreSQL**: PostgreSQL 10+
  - User with permissions: `SELECT` on source database

### Target Database
- **MySQL**: MySQL 5.7+ or MariaDB 10.2+
  - User with permissions: `CREATE` tables, `INSERT`, `UPDATE`, `DELETE`
- **PostgreSQL**: PostgreSQL 10+
  - User with permissions: `CREATE` tables and schemas, `INSERT`, `UPDATE`, `DELETE`

### System Requirements
- Docker (recommended) or Rust 1.70+
- 512MB RAM minimum (1GB+ recommended)
- Network access between source and target databases

## 🔧 Installation

### Using Docker (Recommended)

1. **Clone the repository**
```bash
cd "DB sync"
```

2. **Run the Web UI**
```bash
./run-web-ui.sh
```

The Web UI will be available at http://localhost:5009

### Using Docker Compose

```bash
docker-compose up -d
```

### From Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build --release

# Run with web UI
./target/release/db_sync_proxy --web-ui
```

## ⚙️ Configuration

### Web UI Configuration

1. Open http://localhost:5009 in your browser
2. Create first user account (appears on first run)
3. Go to **Settings** tab
4. Fill in source and target database connection details:
   - **Source DB Type**: MySQL or PostgreSQL
   - **Source**: Host, Port, Database, Username, Password
   - **Target DB Type**: MySQL or PostgreSQL (must match source type)
   - **Target**: Host, Port, Database, Username, Password
5. Configure sync options:
   - **Batch Size**: Records per batch (default: 100)
   - **Poll Interval**: Seconds between change checks (default: 10)
   - **Sync Mode**: Full Sync / Initial Only / Real-time Only
6. Click **Save Configuration**

### Environment Variables (CLI Mode)

For **MySQL to MySQL** sync:

```bash
# Database types
export SOURCE_DB_TYPE=mysql
export TARGET_DB_TYPE=mysql

# Source MySQL Configuration
export SOURCE_DB_HOST=localhost
export SOURCE_DB_PORT=3306
export SOURCE_DB_DATABASE=sourcedb
export SOURCE_DB_USERNAME=user
export SOURCE_DB_PASSWORD=password

# Target MySQL Configuration
export TARGET_DB_HOST=localhost
export TARGET_DB_PORT=3307
export TARGET_DB_DATABASE=targetdb
export TARGET_DB_USERNAME=user
export TARGET_DB_PASSWORD=password

# Sync Configuration
export BATCH_SIZE=100
export POLL_INTERVAL_SECS=10
```

For **PostgreSQL to PostgreSQL** sync:

```bash
# Database types
export SOURCE_DB_TYPE=postgresql
export TARGET_DB_TYPE=postgresql

# Source PostgreSQL Configuration
export SOURCE_DB_HOST=localhost
export SOURCE_DB_PORT=5432
export SOURCE_DB_DATABASE=sourcedb
export SOURCE_DB_USERNAME=user
export SOURCE_DB_PASSWORD=password

# Target PostgreSQL Configuration
export TARGET_DB_HOST=localhost
export TARGET_DB_PORT=5433
export TARGET_DB_DATABASE=targetdb
export TARGET_DB_USERNAME=user
export TARGET_DB_PASSWORD=password

# Sync Configuration
export BATCH_SIZE=100
```

## 🎯 Usage

### Web UI Mode (Recommended)

1. **Start the application**
   ```bash
   ./run-web-ui.sh
   ```

2. **Access the dashboard** at http://localhost:5009

3. **Configure connections** in the Settings tab

4. **Select sync mode** in the Home tab:
   - **Full Sync**: Complete migration + continuous monitoring (recommended for first-time, MySQL only)
   - **Initial Sync Only**: Schema + data transfer, then stops
   - **Real-time Sync Only**: Only monitors changes (MySQL only, assumes data already migrated)

5. **Start synchronization** by clicking "Start Sync"

6. **Monitor progress**:
   - **Home**: Current status and quick stats
   - **Logs**: Detailed operation logs
   - **Statistics**: Operation counts and performance metrics
   - **Chart**: Hourly operation trends with interactive line graph

### CLI Mode

#### MySQL to MySQL - Full Sync (Initial + Real-time)
```bash
export SOURCE_DB_TYPE=mysql
export TARGET_DB_TYPE=mysql
# ... set other environment variables ...
./rebuild-and-run.sh --full-sync
```

#### PostgreSQL to PostgreSQL - Initial Sync Only
```bash
export SOURCE_DB_TYPE=postgresql
export TARGET_DB_TYPE=postgresql
# ... set other environment variables ...
./rebuild-and-run.sh --initial-sync
```

## 🔄 Sync Modes Explained

### 1. Full Sync (MySQL only - Recommended for First-Time)
**What it does:**
- Phase 1: Migrates schema and transfers all existing data
- Phase 2: Replays changes that occurred during initial transfer (catch-up)
- Phase 3: Starts real-time monitoring of new changes

**Best for:**
- First-time migration between MySQL databases
- Complete database replication
- Ensuring zero data loss

### 2. Initial Sync Only
**What it does:**
- Copies schema to target database
- Transfers all existing data
- Stops after completion

**Best for:**
- One-time data migration
- Creating a snapshot of current data
- Both MySQL and PostgreSQL

### 3. Real-time Sync Only (MySQL only)
**What it does:**
- Assumes schema and data already exist in target database
- Only monitors and replicates new changes
- Uses MySQL general_log for change detection

**Best for:**
- Resuming sync after interruption
- Monitoring ongoing changes after initial migration
- Continuous replication setups

## 📊 Statistics & Monitoring

### Real-Time Statistics
The tool tracks every operation and provides:
- **Hourly aggregation**: Operations grouped by hour
- **Operation breakdown**: Separate counts for INSERT, UPDATE, DELETE
- **Success rates**: Track successful vs failed operations
- **Persistent storage**: All stats saved to SQLite database

### Interactive Chart
- **Line graph** showing operation trends over time
- **Three colored lines**: Green (INSERT), Blue (UPDATE), Red (DELETE)
- **Smooth curves** with filled areas for easy visualization
- **Hover tooltips** showing exact operation counts
- **Last 24 hours** of data displayed

### Accessing Statistics
- **Web UI**: Go to Chart tab for visual representation
- **Database**: Query `operation_stats` table in SQLite for custom analytics
- **API**: `/api/chart-stats` endpoint returns JSON data

## 🔐 Security

### Authentication
- **First-time setup**: Create initial admin user on first launch
- **Password hashing**: bcrypt with secure salt
- **Session management**: HTTP-only cookies with expiration
- **Protected routes**: All API endpoints require authentication

### User Management
- **Profile updates**: Change email and password anytime
- **Session timeout**: Automatic logout after inactivity
- **Secure storage**: User credentials stored in SQLite with encryption

## 🐳 Docker Deployment

### Using Docker Compose

Create `docker-compose.yml`:
```yaml
version: '3.8'

services:
  db-sync:
    image: db_sync_proxy:latest
    ports:
      - "5009:5009"
    volumes:
      - ./db_sync_data:/app/data
    environment:
      - RUST_LOG=info
    command: --web-ui
    restart: unless-stopped
```

Run:
```bash
docker-compose up -d
```

### Volume Persistence
The `/app/data` volume contains:
- `config.db`: SQLite database with all settings
- User credentials and sessions
- Operation statistics
- Configuration history

## 🛠️ Troubleshooting

### General_log Errors (MySQL)
**Problem**: `general_log is not enabled`
**Solution**: The tool enables it automatically if you have SUPER privilege. If not:
```sql
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';
```

### Database Type Mismatch
**Problem**: "Source and target database types must be the same"
**Solution**: This tool only supports same-type synchronization. Ensure SOURCE_DB_TYPE and TARGET_DB_TYPE are identical (both `mysql` or both `postgresql`).

### Connection Errors
**Problem**: Cannot connect to source/target database
**Solutions**:
- Verify credentials in Settings
- Check network connectivity
- Ensure databases are accessible from container
- Use `Test Connection` button in Web UI

### Chart Shows No Data
**Problem**: Operations occur but chart is empty
**Solution**: The tool saves statistics to SQLite. Make sure:
- Real-time sync is running
- Operations are being made in source database
- Wait 2-4 seconds for first detection
- Check Logs tab for operation confirmations

## 📚 API Endpoints

### Authentication
- `POST /api/login` - User login
- `POST /api/logout` - User logout
- `GET /api/check-auth` - Check authentication status
- `POST /api/setup-first-user` - Create first user

### Configuration
- `GET /api/config` - Get current configuration
- `POST /api/config` - Update configuration
- `POST /api/test-connection` - Test database connections

### Sync Control
- `POST /api/sync/start` - Start synchronization
- `POST /api/sync/stop` - Stop synchronization
- `GET /api/status` - Get sync status

### Monitoring
- `GET /api/stats` - Get sync statistics
- `GET /api/logs` - Get operation logs
- `GET /api/chart-stats` - Get hourly chart data

### User Management
- `GET /api/profile` - Get user profile
- `POST /api/update-email` - Update user email
- `POST /api/update-password` - Change password

## 🏗️ Architecture

### Technology Stack
- **Backend**: Rust (Tokio async runtime)
- **Web Framework**: Axum
- **Database Drivers**: sqlx (MySQL, PostgreSQL, SQLite)
- **Frontend**: Vanilla JavaScript, Chart.js
- **Storage**: SQLite for config and stats

### Components
1. **Schema Readers**: Read schema from MySQL or PostgreSQL
2. **Table Creator**: Generates DDL for target database
3. **Data Migrator**: Bulk transfers with batching
4. **Binlog Reader** (MySQL): Monitors MySQL general_log for changes
5. **Database Writers**: Applies operations to MySQL or PostgreSQL
6. **Stats Logger**: Tracks operations in SQLite
7. **Web Server**: Axum HTTP server with authentication

### Data Flow
```
Source DB → Schema Reader → Table Creator → Target DB
  ↓                                         ↑
General Log → Binlog Reader → Event Queue → Writer
                     ↓
              Stats Logger → SQLite → Chart API → Web UI
```

## 🤝 Contributing

Contributions are welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## 📄 License

MIT License

## 🙋 Support

For issues, questions, or feature requests:
- Open an issue on GitHub
- Check logs at `/api/logs` for debugging
- Enable `RUST_LOG=debug` for detailed logging

## 🎉 Acknowledgments

Built with:
- Rust and Tokio
- Axum web framework
- sqlx database toolkit
- Chart.js for visualizations

---

**Version**: 1.0.0  
**Last Updated**: December 2025

## ⚠️ Important Notes

- **Same-Type Only**: This tool only supports synchronization between databases of the same type (MySQL→MySQL or PostgreSQL→PostgreSQL)
- **Real-time Sync**: Currently only available for MySQL using general_log monitoring
- **Performance**: For large databases, consider using initial sync during off-peak hours
- **Testing**: Always test in a development environment before production use
