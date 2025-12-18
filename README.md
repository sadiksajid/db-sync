# MySQL to PostgreSQL Proxy

A high-performance, production-ready Rust application that synchronizes data and schema from MySQL to PostgreSQL in real-time. This proxy supports both initial bulk migration and continuous real-time synchronization.

## Table of Contents

- [Features](#features)
- [Statistics & Migration Planning](#-statistics--migration-planning)
- [Architecture](#architecture)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Catch-Up Sync](#catch-up-sync)
- [Real-Time Sync Setup](#real-time-sync-setup)
- [Database Objects Migration](#database-objects-migration)
- [Docker Deployment](#docker-deployment)
- [Data Type Mappings](#data-type-mappings)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)
- [Performance](#performance)
- [Contributing](#contributing)

## Features

### Core Capabilities

- **Schema Migration**: Automatically reads MySQL schema (tables, columns, indexes, foreign keys, data types) and converts to PostgreSQL-compatible schema
- **Data Transfer**: Efficiently transfers data in configurable batches with automatic dependency resolution
- **Database Objects**: Migrates views, functions, stored procedures, and triggers with automatic syntax conversion
- **Catch-Up Sync**: Automatically detects and replays changes that occurred during initial data transfer, ensuring zero data loss
- **Real-Time Synchronization**: Monitors MySQL `general_log` for INSERT/UPDATE/DELETE operations and replicates them to PostgreSQL in real-time
- **Dependency Resolution**: Uses topological sorting to ensure tables are created and populated in the correct order based on foreign key relationships
- **Error Handling**: Robust error handling with automatic retry logic and detailed logging
- **Non-Blocking Queue**: Asynchronous job queue system prevents listener blocking on slow PostgreSQL operations

### Advanced Features

- **Type Conversion**: Automatic conversion of MySQL-specific types to PostgreSQL equivalents (e.g., `AUTO_INCREMENT` → `SERIAL`/`IDENTITY`)
- **Invalid Date Handling**: Automatically handles MySQL's invalid dates (`0000-00-00`) by converting to NULL
- **Batch Processing**: Configurable batch size for optimal performance
- **Connection Pooling**: Uses connection pools for both MySQL and PostgreSQL for better performance
- **Comprehensive Logging**: Structured logging with configurable log levels
- **📊 Statistics Tracking**: Automatic operation logging to identify optimal migration windows (see [STATISTICS.md](STATISTICS.md))

## 📊 Statistics & Migration Planning

The proxy includes built-in statistics tracking to help you identify the **best time to migrate** with minimal impact on your users:

**✅ Automatic Tracking**: Logs all INSERT/UPDATE/DELETE operations during real-time sync  
**✅ Hourly Summaries**: Console displays stats every 5 minutes  
**✅ JSON Export**: Complete log saved to `sync_operations_stats.json`  
**✅ Best Window**: Automatically recommends the quietest hour for migration  

**Example Output:**
```
📊 2025-12-09 08:00:00 - 245 inserts, 120 updates, 15 deletes (total: 380)
📊 2025-12-09 12:00:00 - 56 inserts, 22 updates, 2 deletes (total: 80)
📊 💡 BEST TIME TO SWITCH: 2025-12-09 12:00:00 (only 80 operations)
```

**📖 See [STATISTICS.md](STATISTICS.md) for the complete guide including:**
- How to use statistics for migration planning
- How to visualize data with charts
- Best practices for minimizing downtime

## Architecture

```
┌─────────────┐
│   MySQL     │
│  Database   │
└──────┬──────┘
       │
       │ Schema + Data
       │
┌──────▼──────────────────────────────────────┐
│         MySQL to PostgreSQL Proxy            │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │      Schema Reader & Converter        │  │
│  │  - Reads INFORMATION_SCHEMA          │  │
│  │  - Converts MySQL → PostgreSQL types │  │
│  │  - Builds dependency graph           │  │
│  └──────────────────────────────────────┘  │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │         Data Transfer Engine         │  │
│  │  - Records start timestamp           │  │
│  │  - Batch processing                  │  │
│  │  - Topological ordering              │  │
│  │  - Error recovery                    │  │
│  └──────────────────────────────────────┘  │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │        Catch-Up Sync (NEW!)          │  │
│  │  - Queries general_log from start    │  │
│  │  - Replays changes during transfer   │  │
│  │  - Loops until synchronized          │  │
│  │  - Ensures zero data loss            │  │
│  └──────────────────────────────────────┘  │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │         Real-Time Sync               │  │
│  │  - Monitors general_log              │  │
│  │  - Event queue (non-blocking)        │  │
│  │  - Async PostgreSQL writer           │  │
│  └──────────────────────────────────────┘  │
└──────┬──────────────────────────────────────┘
       │
       │ Replicated Data
       │
┌──────▼──────┐
│ PostgreSQL  │
│  Database   │
└─────────────┘
```

### Components

1. **Schema Reader** (`src/schema/mysql_reader.rs`): Extracts schema from MySQL's `INFORMATION_SCHEMA`
2. **Schema Converter** (`src/schema/pg_converter.rs`): Converts MySQL schema to PostgreSQL DDL
3. **Dependency Graph** (`src/schema/dependency.rs`): Builds and sorts tables by foreign key dependencies
4. **Data Transfer** (`src/migrator/data_transfer.rs`): Handles batch data migration with timestamp recording
5. **Catch-Up Sync** (`src/realtime/binlog_reader.rs::catchup_from_timestamp`): Replays changes from initial transfer
6. **Real-Time Sync** (`src/realtime/`): Monitors MySQL changes and replicates to PostgreSQL
7. **PostgreSQL Writer** (`src/realtime/pg_writer.rs`): Applies changes to PostgreSQL asynchronously

## Installation

### Prerequisites

- Rust 1.75+ (or use Docker)
- MySQL 5.7+ or 8.0+
- PostgreSQL 12+
- Docker and Docker Compose (optional, for containerized deployment)

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd mysql_psql_proxy

# Build the project
cargo build --release

# The binary will be at: target/release/mysql_psql_proxy
```

### Using Docker

```bash
# Build the Docker image
docker build -t mysql_psql_proxy:latest .

# Or use the provided script
./rebuild-and-run.sh --initial-sync
```

## Configuration

The proxy uses environment variables for configuration. Multiple patterns are supported for flexibility.

### Environment Variables

#### Option 1: Full Connection URLs

```bash
MYSQL_URL=mysql://user:password@host:3306/database
PG_URL=postgres://user:password@host:5432/database
```

#### Option 2: Individual MySQL Variables

```bash
MYSQL_HOST=192.168.1.237
MYSQL_PORT=3306
MYSQL_USER=root
MYSQL_PASSWORD=password
MYSQL_DATABASE=my_database
```

#### Option 3: Individual PostgreSQL Variables

```bash
POSTGRES_HOST=192.168.1.237
POSTGRES_PORT=5432
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_DB=my_database
```

#### Option 4: Unified Pattern (Recommended)

```bash
# MySQL Configuration
DB_HOST=192.168.1.237
DB_PORT=3306
DB_USERNAME=root
DB_PASSWORD=password
DB_DATABASE=my_database

# PostgreSQL Configuration
PSQL_DB_HOST=192.168.1.237
PSQL_DB_PORT=5432
PSQL_DB_USERNAME=postgres
PSQL_DB_PASSWORD=postgres
PSQL_DB_DATABASE=my_database
```

### Additional Configuration

```bash
# Sync mode: initial, realtime, or both (default: both)
SYNC_MODE=both

# Batch size for data transfer (default: 1000)
BATCH_SIZE=1000

# Log level: trace, debug, info, warn, error (default: info)
RUST_LOG=info
```

### Priority Order

The proxy checks environment variables in this order:

1. **MySQL**: `MYSQL_URL` → `MYSQL_*` → `DB_*`
2. **PostgreSQL**: `PG_URL` → `POSTGRES_*` → `PSQL_DB_*`

## Usage

### Command-Line Arguments

```bash
# Initial sync only (schema + data)
./mysql_psql_proxy --initial-sync

# Real-time sync only (monitor changes)
./mysql_psql_proxy --realtime-sync

# Full sync (initial + real-time)
./mysql_psql_proxy --full-sync
```

### Examples

#### Initial Migration

```bash
# Set environment variables
export DB_HOST=192.168.1.237
export DB_PORT=3306
export DB_USERNAME=root
export DB_PASSWORD=password
export DB_DATABASE=source_db

export PSQL_DB_HOST=192.168.1.237
export PSQL_DB_PORT=5432
export PSQL_DB_USERNAME=postgres
export PSQL_DB_PASSWORD=postgres
export PSQL_DB_DATABASE=target_db

# Run initial sync
./mysql_psql_proxy --initial-sync
```

#### Real-Time Sync

```bash
# Run real-time sync (requires general_log enabled)
./mysql_psql_proxy --realtime-sync
```

#### Using Docker

```bash
docker run --rm \
  -e DB_HOST=192.168.1.237 \
  -e DB_PORT=3306 \
  -e DB_DATABASE=my_db \
  -e DB_USERNAME=root \
  -e DB_PASSWORD=password \
  -e PSQL_DB_HOST=192.168.1.237 \
  -e PSQL_DB_PORT=5432 \
  -e PSQL_DB_DATABASE=my_db \
  -e PSQL_DB_USERNAME=postgres \
  -e PSQL_DB_PASSWORD=postgres \
  -e RUST_LOG=info \
  mysql_psql_proxy:latest \
  --full-sync
```

## Catch-Up Sync

**🎯 Zero Data Loss Guarantee**: The catch-up sync mechanism ensures no data is lost during the initial transfer.

### The Problem

During initial data transfer (which can take minutes or hours for large databases), changes may occur in MySQL:

```
Time 0:00 - Initial sync starts
Time 0:05 - User INSERT new row (during transfer)
Time 2:30 - User UPDATE a row (during transfer)
Time 5:00 - Initial sync completes
```

Without catch-up, these changes at 0:05 and 2:30 would be **lost**.

### The Solution

When using `--full-sync`, the proxy automatically:

1. **Records** the start timestamp before data transfer
2. **Transfers** all data in batches
3. **Queries** `mysql.general_log` for changes since start timestamp
4. **Applies** all detected changes to PostgreSQL
5. **Repeats** steps 3-4 until no more changes found
6. **Starts** real-time sync for ongoing synchronization

This ensures **complete data consistency** between MySQL and PostgreSQL.

### How to Enable

Catch-up sync is **automatically enabled** when using `--full-sync`:

```bash
docker run --rm \
  -e DB_HOST=192.168.1.237 \
  -e DB_PORT=3306 \
  -e DB_DATABASE=my_db \
  -e DB_USERNAME=root \
  -e DB_PASSWORD=password \
  -e PSQL_DB_HOST=192.168.1.237 \
  -e PSQL_DB_PORT=5432 \
  -e PSQL_DB_DATABASE=my_db \
  -e PSQL_DB_USERNAME=postgres \
  -e PSQL_DB_PASSWORD=postgres \
  mysql_psql_proxy:latest \
  --full-sync
```

### Log Output

You'll see these messages during catch-up:

```
INFO  📍 Recording start timestamp for catch-up sync...
INFO  📍 Start timestamp: 2024-12-08 10:00:00.123456

INFO  🔄 Starting catch-up sync to replay changes from initial transfer
INFO  🔄 Catch-up iteration #1
INFO  ⚠️  Found 42 changes that occurred during initial transfer
INFO  Applying catch-up changes...
INFO  ✓ Catch-up complete: applied 42 changes out of 42 found

INFO  🔄 Catch-up iteration #2
INFO  ✓ No changes detected during initial transfer
INFO  ✓ Catch-up complete: databases are synchronized
```

### Requirements

Catch-up sync requires the same `general_log` setup as real-time sync (see below).

For detailed information, see [CATCHUP_SYNC.md](CATCHUP_SYNC.md).

## Database Objects Migration

**🎯 Complete Database Migration**: The proxy can migrate not just tables and data, but also:

- ✅ **Views**: Virtual tables based on SELECT queries
- ✅ **Functions**: Stored functions with return values
- ✅ **Procedures**: Stored procedures (converted to PostgreSQL functions)
- ✅ **Triggers**: Automatic event handlers on tables

### ⚠️ Gemini AI Required

Database objects migration requires **Google Gemini API** for accurate conversion:

```bash
export GEMINI_API_KEY="your-api-key-here"
export GEMINI_MODEL="gemini-2.0-flash-exp"  # Optional, defaults to gemini-2.0-flash-exp
```

Or edit `rebuild-and-run.sh` and replace the empty values:
```bash
-e GEMINI_API_KEY=""  # <- Add your API key here
-e GEMINI_MODEL="gemini-2.0-flash-exp"
```

**Without Gemini API key**: Database objects (views, functions, procedures, triggers) will be **SKIPPED** to avoid creating broken objects.

**With Gemini API key**: ~90-95% success rate with AI-powered intelligent conversion.

Get your free API key at: https://makersuite.google.com/app/apikey

### Automatic Migration

During `--initial-sync` or `--full-sync` (with GEMINI_API_KEY set), the proxy automatically:

1. **Reads** all database objects from MySQL
2. **Converts** MySQL syntax to PostgreSQL using Gemini AI
3. **Creates** the objects in PostgreSQL
4. **Reports** success/failure for each object

### Syntax Conversion

The proxy performs intelligent conversions:

- Backticks `` `name` `` → Double quotes `"name"`
- `IFNULL()` → `COALESCE()`
- `NOW()` → `CURRENT_TIMESTAMP`
- `TINYINT` → `SMALLINT`
- `DATETIME` → `TIMESTAMP`
- MySQL functions/procedures → PostgreSQL PL/pgSQL
- MySQL triggers → PostgreSQL trigger functions + triggers

### Success Rate

- **Views**: ~95% automatic success
- **Functions**: ~70% automatic success
- **Procedures**: ~70% automatic success
- **Triggers**: ~80% automatic success

Complex objects with MySQL-specific features may require manual adjustment. The proxy logs detailed warnings for objects that need review.

### Example

```
INFO  Reading database objects from MySQL...
INFO  Found 5 views, 3 functions, 2 procedures, 4 triggers
INFO  Migrating database objects to PostgreSQL...
INFO  Using Gemini AI to convert view: customer_summary
INFO  ✓ Created view: customer_summary
INFO  ⏳ Rate limiting: waiting 60s before next Gemini API call...
INFO  Using Gemini AI to convert function: calculate_discount
INFO  ✓ Created function: calculate_discount
INFO  ⏳ Rate limiting: waiting 60s before next Gemini API call...
INFO  Using Gemini AI to convert procedure: update_inventory
INFO  ✓ Created procedure: update_inventory
INFO  View migration complete: 5 successful, 0 failed
INFO  Function migration complete: 3 successful, 0 failed
INFO  Procedure migration complete: 2 successful, 0 failed
INFO  Trigger migration complete: 4 successful, 0 failed
```

**Note**: Gemini API calls are rate-limited to **1 call per minute** to stay within free tier limits. For 14 objects, migration takes ~14 minutes.

For detailed information, examples, and troubleshooting, see [DATABASE_OBJECTS.md](DATABASE_OBJECTS.md).

## Real-Time Sync Setup

Real-time synchronization requires MySQL's `general_log` to be enabled. The proxy will attempt to enable it automatically, but you may need to configure it manually.

### Automatic Setup

The proxy attempts to enable `general_log` automatically when starting real-time sync. This requires the MySQL user to have `SUPER` privilege.

### Manual Setup

If automatic setup fails, enable it manually:

```sql
-- Connect to MySQL as root or user with SUPER privilege
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';

-- Verify
SHOW VARIABLES LIKE 'general_log';
SHOW VARIABLES LIKE 'log_output';
```

### Granting SUPER Privilege

If your MySQL user doesn't have `SUPER` privilege:

```sql
-- Grant SUPER privilege (MySQL 8.0+)
GRANT SUPER ON *.* TO 'your_user'@'%';
FLUSH PRIVILEGES;

-- Or for MySQL 5.7
GRANT SUPER ON *.* TO 'your_user'@'%' IDENTIFIED BY 'your_password';
FLUSH PRIVILEGES;
```

### How It Works

1. The proxy polls `mysql.general_log` table every second
2. It filters for INSERT/UPDATE/DELETE queries
3. Parses queries to extract table names, columns, and values
4. Enqueues events to a non-blocking job queue
5. A background worker applies changes to PostgreSQL asynchronously

### Performance Considerations

- **Polling Interval**: Default is 1 second. Can be adjusted in code.
- **Queue Size**: Default is 1000 events. Events are dropped if queue is full (logged as warning).
- **Query Performance**: The `general_log` table can grow large. Consider periodic cleanup:
  ```sql
  TRUNCATE TABLE mysql.general_log;
  ```

## Docker Deployment

### Using Docker Compose

The `docker-compose.yml` file is configured to connect to existing databases. Update the environment variables:

```yaml
services:
  proxy:
    build: .
    environment:
      DB_HOST: 192.168.1.237
      DB_PORT: 3306
      DB_DATABASE: my_database
      DB_USERNAME: root
      DB_PASSWORD: password
      
      PSQL_DB_HOST: 192.168.1.237
      PSQL_DB_PORT: 5432
      PSQL_DB_DATABASE: my_database
      PSQL_DB_USERNAME: postgres
      PSQL_DB_PASSWORD: postgres
      
      SYNC_MODE: both
      BATCH_SIZE: 1000
      RUST_LOG: info
    command: ["--full-sync"]
    restart: unless-stopped
```

### Build and Run Script

Use the provided `rebuild-and-run.sh` script:

```bash
# Make executable
chmod +x rebuild-and-run.sh

# Run with initial sync
./rebuild-and-run.sh --initial-sync

# Run with real-time sync
./rebuild-and-run.sh --realtime-sync

# Run with full sync
./rebuild-and-run.sh --full-sync
```

## Data Type Mappings

The proxy automatically converts MySQL data types to PostgreSQL equivalents:

| MySQL Type | PostgreSQL Type | Notes |
|------------|----------------|-------|
| `TINYINT` | `SMALLINT` | Signed values |
| `TINYINT UNSIGNED` | `SMALLINT` | Unsigned converted to signed |
| `SMALLINT` | `SMALLINT` | Direct mapping |
| `MEDIUMINT` | `INTEGER` | |
| `INT` / `INTEGER` | `INTEGER` | Direct mapping |
| `BIGINT` | `BIGINT` | Direct mapping |
| `FLOAT` | `REAL` | |
| `DOUBLE` | `DOUBLE PRECISION` | |
| `DECIMAL(p,s)` | `NUMERIC(p,s)` | Precision preserved |
| `NUMERIC(p,s)` | `NUMERIC(p,s)` | Direct mapping |
| `CHAR(n)` | `CHAR(n)` | Length preserved |
| `VARCHAR(n)` | `VARCHAR(n)` | Length preserved (max 1GB) |
| `TEXT` | `TEXT` | All TEXT variants → TEXT |
| `TINYTEXT` | `TEXT` | |
| `MEDIUMTEXT` | `TEXT` | |
| `LONGTEXT` | `TEXT` | |
| `DATE` | `DATE` | Invalid dates → NULL |
| `DATETIME` | `TIMESTAMP` | Invalid dates → NULL |
| `TIMESTAMP` | `TIMESTAMP` | Invalid dates → NULL |
| `TIME` | `TIME` | |
| `YEAR` | `SMALLINT` | |
| `BINARY(n)` | `BYTEA` | |
| `VARBINARY(n)` | `BYTEA` | |
| `BLOB` | `BYTEA` | All BLOB variants → BYTEA |
| `TINYBLOB` | `BYTEA` | |
| `MEDIUMBLOB` | `BYTEA` | |
| `LONGBLOB` | `BYTEA` | |
| `JSON` | `JSONB` | |
| `ENUM` | `VARCHAR` | Values preserved |
| `SET` | `TEXT` | Comma-separated values |
| `AUTO_INCREMENT` | `SERIAL` / `IDENTITY` | Based on column type |

### Special Handling

- **AUTO_INCREMENT**: Converted to `SERIAL` for INTEGER columns or `IDENTITY` for BIGINT
- **Invalid Dates**: MySQL's `0000-00-00` dates are converted to `NULL` in PostgreSQL
- **VARCHAR Length**: If length > 10MB, converted to `TEXT` for safety
- **Default Values**: Invalid date defaults (`0000-00-00`) are skipped during table creation

## Troubleshooting

### Common Issues

#### 1. "Cannot access mysql.general_log"

**Problem**: Real-time sync cannot read `general_log` table.

**Solution**:
```sql
-- Grant SUPER privilege
GRANT SUPER ON *.* TO 'your_user'@'%';
FLUSH PRIVILEGES;

-- Or enable general_log manually
SET GLOBAL general_log = 'ON';
SET GLOBAL log_output = 'TABLE';
```

#### 2. "date/time field value out of range"

**Problem**: MySQL has invalid dates (`0000-00-00`) that PostgreSQL rejects.

**Solution**: The proxy automatically handles this by converting invalid dates to NULL. If you still see errors, check:
- Column allows NULL in PostgreSQL schema
- Default values don't contain invalid dates

#### 3. "operator does not exist: integer = text"

**Problem**: Type mismatch in WHERE clauses during real-time sync.

**Solution**: The proxy automatically detects and converts integer values. If issues persist:
- Check column types match between MySQL and PostgreSQL
- Verify primary key columns are correctly identified

#### 4. "Event queue is full"

**Problem**: PostgreSQL writer is too slow, queue fills up.

**Solution**:
- Increase queue size in code (default: 1000)
- Check PostgreSQL performance
- Reduce batch size or increase PostgreSQL connection pool

#### 5. "Duplicate key value"

**Problem**: Trying to insert rows that already exist.

**Solution**: This is expected if:
- Initial sync was run multiple times
- Real-time sync is catching up on old changes

The proxy logs warnings and skips duplicate rows.

#### 6. "value too long for type character varying(255)"

**Problem**: VARCHAR length mismatch between MySQL and PostgreSQL.

**Solution**: The proxy should handle this automatically. If not:
- Check `CHARACTER_MAXIMUM_LENGTH` is correctly read from MySQL
- Verify `pg_converter.rs` is using the correct length

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug ./mysql_psql_proxy --realtime-sync
```

Or in Docker:

```bash
docker run --rm \
  -e RUST_LOG=debug \
  ... \
  mysql_psql_proxy:latest \
  --realtime-sync
```

### Log Levels

- `trace`: Very detailed logs (not recommended for production)
- `debug`: Detailed logs for debugging
- `info`: Normal operation logs (default)
- `warn`: Warnings only
- `error`: Errors only

## Limitations

### Known Limitations

1. **Schema Differences**:
   - MySQL-specific features (e.g., `ENUM`, `SET`) are converted but may not preserve exact behavior
   - Some MySQL functions in default values may not work in PostgreSQL

2. **Real-Time Sync**:
   - Uses polling (1-second interval) rather than true binlog streaming
   - Requires `general_log` to be enabled (performance impact)
   - May miss changes if `general_log` table is truncated

3. **Data Types**:
   - `YEAR` type converted to `SMALLINT` (loses type semantics)
   - `SET` type converted to `TEXT` (loses set operations)

4. **Performance**:
   - Large `general_log` table can slow down queries
   - No built-in cleanup of `general_log` (manual cleanup required)

5. **Transactions**:
   - Real-time sync processes each change individually (not transactional)
   - If a change fails, it's retried but not rolled back with related changes

### Not Supported

- MySQL replication-specific features
- Stored procedures, functions, triggers (schema only)
- Views (not migrated)
- Partitioning (not migrated)
- Full-text indexes (converted to regular indexes)

## Performance

### Benchmarks

- **Schema Migration**: ~100-200 tables/second
- **Data Transfer**: ~10,000-50,000 rows/second (depends on row size and network)
- **Real-Time Sync**: <100ms latency (1-second polling + processing time)

### Optimization Tips

1. **Batch Size**: Increase `BATCH_SIZE` for faster initial sync (default: 1000)
   ```bash
   BATCH_SIZE=5000 ./mysql_psql_proxy --initial-sync
   ```

2. **Connection Pooling**: Already enabled by default

3. **PostgreSQL Tuning**: 
   - Increase `shared_buffers`
   - Tune `work_mem` for large batches
   - Consider disabling `fsync` during initial sync (not recommended for production)

4. **MySQL general_log**:
   - Periodically truncate to maintain performance
   - Consider using file-based logging instead of table (not supported by proxy)

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

### Development Setup

```bash
# Clone and build
git clone <repository-url>
cd mysql_psql_proxy
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- --initial-sync
```

## License

[Add your license here]

## Support

For issues, questions, or contributions, please open an issue on the repository.

---

**Note**: This proxy is designed for one-way synchronization from MySQL to PostgreSQL. It does not handle bidirectional sync or conflict resolution.
