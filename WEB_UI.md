# 🌐 Web UI - MySQL to PostgreSQL Sync Control Panel

## Overview

The Web UI provides a modern, user-friendly interface to configure and monitor your MySQL to PostgreSQL synchronization in real-time.

## Features

✅ **Visual Configuration**
- Configure MySQL and PostgreSQL connections
- Test database connections before starting sync
- Adjust batch size, poll intervals, and other settings
- Configure Gemini AI API for database objects migration

✅ **Real-Time Monitoring**
- Live status indicator (Idle, Running, Stopped, Error)
- Real-time statistics dashboard
- Live operation logs with auto-refresh
- Track inserts, updates, deletes, and errors

✅ **Control Operations**
- Start/stop synchronization with one click
- View detailed progress information
- Monitor sync health and performance

## Quick Start

### 1. Start the Web UI

```bash
./run-web-ui.sh
```

This will:
- Build the Docker image (if needed)
- Start the web server on port **5009**
- Open the control panel at: http://localhost:5009

### 2. Access the Control Panel

Open your web browser and navigate to:
```
http://localhost:5009
```

Or from another machine on your network:
```
http://YOUR_SERVER_IP:5009
```

### 3. Configure Your Databases

#### MySQL Configuration
- **Host**: Your MySQL server IP/hostname (e.g., `192.168.1.237`)
- **Port**: MySQL port (default: `3306`)
- **Database**: Source database name
- **Username**: MySQL user with read permissions
- **Password**: MySQL password

Click **"Test MySQL Connection"** to verify.

#### PostgreSQL Configuration
- **Host**: Your PostgreSQL server IP/hostname
- **Port**: PostgreSQL port (default: `5432`)
- **Database**: Target database name (must exist)
- **Username**: PostgreSQL user with write permissions
- **Password**: PostgreSQL password

Click **"Test PostgreSQL Connection"** to verify.

#### Sync Configuration
- **Batch Size**: Number of rows to transfer at once (default: `100`)
- **Poll Interval**: Seconds between change checks (default: `10`)

#### AI Configuration (Optional)
- **Gemini API Key**: Your Google Gemini API key (leave empty to skip AI conversion)
- **Gemini Model**: Model to use (default: `gemini-2.0-flash-exp`)

### 4. Start Synchronization

1. Click **"💾 Save Configuration"** to save your settings
2. Click **"▶️ Start Sync"** to begin the migration

The system will run through three phases:
1. **Initial Sync**: Copy all tables, schemas, and data
2. **Catch-Up Sync**: Apply changes that occurred during initial sync
3. **Real-Time Sync**: Continuously replicate live changes

### 5. Monitor Progress

Watch the **Statistics Dashboard** for live updates:
- **Tables Synced**: Number of tables migrated
- **Rows Synced**: Total rows transferred
- **Views/Functions/Procedures/Triggers**: Database objects migrated
- **Inserts/Updates/Deletes**: Real-time operations applied
- **Errors**: Any issues encountered

The **Logs Panel** shows detailed operation messages in real-time.

## API Endpoints

The Web UI exposes a REST API for programmatic access:

### Configuration
- `GET /api/config` - Get current configuration
- `POST /api/config` - Update configuration

### Status & Statistics
- `GET /api/status` - Get sync status (idle/running/stopped/error)
- `GET /api/stats` - Get operation statistics
- `GET /api/logs` - Get operation logs

### Control
- `POST /api/sync/start` - Start synchronization
- `POST /api/sync/stop` - Stop synchronization
- `POST /api/test-connection` - Test database connection

### Example API Usage

```bash
# Get current status
curl http://localhost:5009/api/status

# Get statistics
curl http://localhost:5009/api/stats | jq .

# Start sync
curl -X POST http://localhost:5009/api/sync/start

# Stop sync
curl -X POST http://localhost:5009/api/sync/stop
```

## Architecture

### Frontend
- **Pure HTML/CSS/JavaScript** - No framework dependencies
- **Auto-refresh** - Status updates every 2s, stats every 3s, logs every 5s
- **Responsive Design** - Works on desktop, tablet, and mobile

### Backend
- **Axum Web Framework** - High-performance async web server
- **Tokio Runtime** - Non-blocking I/O for concurrent operations
- **Shared State** - Thread-safe state management with RwLock/Mutex

### Components
```
src/web/
├── mod.rs           # Module exports
├── state.rs         # Application state (config, status, stats, logs)
└── server.rs        # Web server and API endpoints

static/
└── index.html       # Single-page web application
```

## Troubleshooting

### Port Already in Use
```bash
# Find what's using port 5009
sudo lsof -i :5009

# Kill the process
docker stop $(docker ps -q --filter ancestor=mysql_psql_proxy:latest)
```

### Can't Connect to Web UI
```bash
# Check if container is running
docker ps | grep mysql_psql_proxy

# Check container logs
docker logs CONTAINER_ID

# Ensure port is exposed
docker run --rm -p 5009:5009 mysql_psql_proxy:latest --web-ui
```

### Sync Not Starting
1. Verify all configuration fields are filled
2. Test both MySQL and PostgreSQL connections
3. Check logs panel for error messages
4. Ensure databases exist and credentials are correct

## Advanced Usage

### Running in Production

For production deployments, consider:

1. **Use Docker Compose** for better management:

```yaml
version: '3.8'
services:
  sync-ui:
    image: mysql_psql_proxy:latest
    command: --web-ui
    ports:
      - "5009:5009"
    restart: unless-stopped
```

2. **Add Reverse Proxy** (nginx/traefik) for:
   - HTTPS/SSL encryption
   - Authentication/authorization
   - Rate limiting
   - Load balancing

3. **Persistent Logs** with volume mounts:

```bash
docker run -d \
  -p 5009:5009 \
  -v $(pwd)/logs:/app/logs \
  mysql_psql_proxy:latest --web-ui
```

### Custom Port

To use a different port, modify `src/web/server.rs`:

```rust
let listener = tokio::net::TcpListener::bind("0.0.0.0:YOUR_PORT")
```

Then rebuild the Docker image.

## Security Considerations

⚠️ **Important**: The Web UI is designed for **local/trusted networks** only!

**DO NOT expose directly to the internet** without:
- Authentication (basic auth, OAuth, etc.)
- HTTPS/TLS encryption
- Network firewall rules
- Rate limiting
- Input validation/sanitization

Passwords are transmitted in plain text over HTTP. Use a reverse proxy with HTTPS in production.

## Performance

The Web UI is lightweight and efficient:
- **Memory**: ~50MB (including Rust runtime)
- **CPU**: < 1% idle, ~5-10% during active sync
- **Network**: Minimal (only API polling traffic)

Auto-refresh intervals are tuned for real-time updates without overwhelming the server.

## Comparison with CLI

| Feature | Web UI | CLI (`rebuild-and-run.sh`) |
|---------|--------|----------------------------|
| Ease of Use | ✅ Visual, user-friendly | ⚠️ Requires terminal knowledge |
| Real-time Monitoring | ✅ Dashboard & live logs | ⚠️ Text logs only |
| Configuration | ✅ Form-based | ⚠️ Environment variables |
| Connection Testing | ✅ Built-in | ❌ Manual |
| Start/Stop Control | ✅ One-click | ⚠️ Ctrl+C / restart |
| API Access | ✅ REST API | ❌ No API |
| Multi-user | ✅ Web accessible | ❌ Local only |
| Performance | ✅ Lightweight | ✅ Minimal overhead |

## Future Enhancements

Planned features:
- [ ] Authentication/authorization
- [ ] HTTPS/TLS support
- [ ] WebSocket for real-time log streaming
- [ ] Configuration profiles (save/load)
- [ ] Sync scheduling/cron
- [ ] Email/Slack notifications
- [ ] Database schema visualization
- [ ] Historical statistics/graphs
- [ ] Multi-database sync management

## Support

For issues, questions, or feature requests:
1. Check the logs panel in the UI
2. Review container logs: `docker logs CONTAINER_ID`
3. See main README.md for general troubleshooting

## License

Same as the main project.

