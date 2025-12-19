# MySQL to PostgreSQL Sync Proxy

A powerful, real-time database synchronization tool that migrates and continuously syncs data from MySQL to PostgreSQL. Built with Rust for high performance and reliability.

## 🚀 Features

### Core Synchronization
- **Schema Migration**: Automatic conversion of MySQL schemas to PostgreSQL-compatible structures
- **Initial Data Transfer**: Bulk transfer of existing data with configurable batch sizes
- **Real-time Sync**: Continuous monitoring and replication of INSERT, UPDATE, DELETE operations
- **Catch-up Sync**: Ensures no data is lost during initial migration by replaying changes
- **Three Sync Modes**: Choose between Full Sync, Initial Sync Only, or Real-time Sync Only

### Database Objects Migration
- **Views**: Migrates MySQL views to PostgreSQL
- **Functions**: Converts MySQL stored functions to PostgreSQL equivalents
- **Procedures**: Migrates stored procedures with parameter mapping
- **Triggers**: Converts MySQL triggers to PostgreSQL trigger functions
- **AI-Powered Conversion**: Optional Gemini AI integration for intelligent SQL conversion

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

### MySQL Database
- MySQL 5.7+ or MariaDB 10.2+
- `general_log` must be enabled (tool enables it automatically if you have SUPER privilege)
- User with permissions:
  - `SELECT` on source database
  - `SELECT` on `mysql.general_log`
  - `SUPER` privilege (for enabling general_log)

### PostgreSQL Database
- PostgreSQL 10+
- User with permissions:
  - `CREATE` tables and schemas
  - `INSERT`, `UPDATE`, `DELETE` on target database

### System Requirements
- Docker (recommended) or Rust 1.70+
- 512MB RAM minimum (1GB+ recommended)
- Network access between source MySQL and target PostgreSQL

## 🔧 Installation

### Using Docker (Recommended)

1. **Clone the repository**
```bash
git clone <repository-url>
cd "mysql to psql proxy"
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
./target/release/mysql_psql_proxy --web-ui
```

## ⚙️ Configuration

### Web UI Configuration

1. Open http://localhost:5009 in your browser
2. Create first user account (appears on first run)
3. Go to **Settings** tab
4. Fill in MySQL and PostgreSQL connection details:
   - **MySQL**: Host, Port, Database, Username, Password
   - **PostgreSQL**: Host, Port, Database, Username, Password
5. Configure sync options:
   - **Batch Size**: Records per batch (default: 100)
   - **Poll Interval**: Seconds between change checks (default: 10)
   - **Sync Mode**: Full Sync / Initial Only / Real-time Only
6. (Optional) Add Gemini API key for intelligent SQL conversion
7. Click **Save Configuration**

### Environment Variables (CLI Mode)

```bash
# MySQL Configuration
export DB_HOST=localhost
export DB_PORT=3306
export DB_DATABASE=mydb
export DB_USERNAME=user
export DB_PASSWORD=password

# PostgreSQL Configuration
export PSQL_DB_HOST=localhost
export PSQL_DB_PORT=5432
export PSQL_DB_DATABASE=mydb
export PSQL_DB_USERNAME=user
export PSQL_DB_PASSWORD=password

# Sync Configuration
export BATCH_SIZE=100
export POLL_INTERVAL_SECS=10

# Optional: Gemini AI for DB objects migration
export GEMINI_API_KEY=your-api-key
export GEMINI_MODEL=gemini-2.0-flash-exp
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
   - **Full Sync**: Complete migration + continuous monitoring (recommended for first-time)
   - **Initial Sync Only**: Schema + data transfer, then stops
   - **Real-time Sync Only**: Only monitors changes (assumes data already migrated)

5. **Start synchronization** by clicking "Start Sync"

6. **Monitor progress**:
   - **Home**: Current status and quick stats
   - **Logs**: Detailed operation logs
   - **Statistics**: Operation counts and performance metrics
   - **Chart**: Hourly operation trends with interactive line graph

### CLI Mode

#### Full Sync (Initial + Real-time)
```bash
./rebuild-and-run.sh --full-sync
```

#### Initial Sync Only
```bash
./rebuild-and-run.sh --initial-sync
```

#### Real-time Sync Only
```bash
./rebuild-and-run.sh --realtime-sync
```

## 🔄 Sync Modes Explained

### 1. Full Sync (Recommended for First-Time)
**What it does:**
- Phase 1: Migrates schema and transfers all existing data
- Phase 2: Replays changes that occurred during initial transfer (catch-up)
- Phase 3: Starts real-time monitoring of new changes

**Best for:**
- First-time migration from MySQL to PostgreSQL
- Complete database replication
- Ensuring zero data loss

### 2. Initial Sync Only
**What it does:**
- Migrates schema to PostgreSQL
- Transfers all existing data
- Stops after completion

**Best for:**
- One-time data migration
- Testing schema conversion
- Creating a snapshot of current data

### 3. Real-time Sync Only
**What it does:**
- Assumes schema and data already exist in PostgreSQL
- Only monitors and replicates new changes
- Uses MySQL general_log for change detection

**Best for:**
- Resuming sync after interruption
- Monitoring ongoing changes after initial migration
- Continuous replication setups

## 🗃️ Database Objects Migration

The tool can migrate complex database objects using two methods:

### 1. Regex-Based Conversion (Default)
Automatically converts basic SQL syntax:
- Data type mapping (INT → INTEGER, VARCHAR → VARCHAR, etc.)
- Function syntax adjustments
- Trigger format conversion

### 2. AI-Powered Conversion (Optional)
With Gemini API key configured:
- Intelligent SQL translation
- Complex syntax handling
- Parameter and variable mapping
- Error-free PostgreSQL generation

**To enable AI conversion:**
1. Get a free API key from [Google AI Studio](https://makersuite.google.com/app/apikey)
2. Add it in Settings → Gemini API Key
3. AI will be used automatically for views, functions, procedures, and triggers

**Rate Limiting:**
- 1 API call per minute to avoid quota issues
- Automatic fallback to regex conversion on quota exceeded

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
  mysql-psql-sync:
    image: mysql_psql_proxy:latest
    ports:
      - "5009:5009"
    volumes:
      - ./mysql_psql_data:/app/data
    environment:
      - RUST_LOG=info
    command: --web-ui
    restart: unless-stopped
```

Run:
```bash
docker-compose up -d
```

### Using Docker Run

```bash
docker run -d \
  -p 5009:5009 \
  -v $(pwd)/mysql_psql_data:/app/data \
  -e RUST_LOG=info \
  --name mysql_psql_ui \
  mysql_psql_proxy:latest --web-ui
```

### Volume Persistence
The `/app/data` volume contains:
- `config.db`: SQLite database with all settings
- User credentials and sessions
- Operation statistics
- Configuration history

## 🛠️ Troubleshooting

### General_log Errors
**Problem**: `general_log is not enabled`
**Solution**: The tool enables it automatically if you have SUPER privilege. If not:
```sql
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';
```

### First Operation Not Detected
**Problem**: First change after starting sync is missed
**Solution**: This is now fixed! The tool waits 2 seconds after initialization before declaring sync active.

### Slow Performance
**Problem**: High CPU or slow queries
**Solutions**:
- Increase `POLL_INTERVAL_SECS` (default: 10)
- Reduce `BATCH_SIZE` if memory is limited
- The tool automatically cleans old general_log entries

### Connection Errors
**Problem**: Cannot connect to MySQL/PostgreSQL
**Solutions**:
- Verify credentials in Settings
- Check network connectivity
- Ensure databases are accessible from container
- Use `Test Connection` button in Web UI

### Chart Shows No Data
**Problem**: Operations occur but chart is empty
**Solution**: The tool now saves statistics to SQLite. Make sure:
- Real-time sync is running
- Operations are being made in MySQL
- Wait 2-4 seconds for first detection
- Check Logs tab for operation confirmations

### Database Object Migration Fails
**Problem**: Views/functions not migrating correctly
**Solutions**:
- Add Gemini API key for AI-powered conversion
- Check logs for specific SQL errors
- Manually adjust complex objects after migration
- Some MySQL-specific features may need manual conversion

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
- **AI Integration**: Google Gemini API

### Components
1. **Schema Reader**: Analyzes MySQL schema structure
2. **Table Creator**: Generates PostgreSQL-compatible DDL
3. **Data Migrator**: Bulk transfers with batching
4. **Binlog Reader**: Monitors MySQL general_log for changes
5. **PG Writer**: Applies operations to PostgreSQL
6. **Stats Logger**: Tracks operations in SQLite
7. **Web Server**: Axum HTTP server with authentication
8. **Routine Migrator**: Converts database objects

### Data Flow
```
MySQL → Schema Reader → Table Creator → PostgreSQL
  ↓                                         ↑
General Log → Binlog Reader → Event Queue → PG Writer
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

[Your License Here]

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
- Google Gemini AI for intelligent SQL conversion

---

**Version**: 1.0.0  
**Author**: [Your Name]  
**Last Updated**: December 2025
