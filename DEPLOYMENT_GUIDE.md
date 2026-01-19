# 🚀 Quick Deployment Guide - GitHub Container Registry

## What Was Set Up

Your DB Sync Proxy project is now configured to automatically build and publish Docker images to GitHub Container Registry (ghcr.io) whenever you push code to GitHub.

### ✅ Files Created/Modified:

1. **`.github/workflows/docker-publish.yml`** - Automated CI/CD pipeline
2. **`.dockerignore`** - Optimizes Docker builds by excluding unnecessary files
3. **`.gitignore`** - Updated to exclude data directories
4. **`README.md`** - Updated with GHCR usage instructions
5. **`GHCR_SETUP.md`** - Detailed GHCR setup and troubleshooting guide
6. **`DEPLOYMENT_GUIDE.md`** - This quick reference (you are here)

## 🎯 Next Steps

### 1. Push to GitHub

First, commit and push these changes to your GitHub repository:

```bash
cd "/home/seddek/sadik/projects/DB sync"

# Check git status
git status

# Add all new files
git add .github/workflows/docker-publish.yml
git add .dockerignore
git add GHCR_SETUP.md
git add DEPLOYMENT_GUIDE.md
git add README.md
git add .gitignore

# Commit changes
git commit -m "Add GitHub Container Registry support with automated builds"

# Push to GitHub (adjust branch name if needed)
git push origin main
```

**Note**: If you haven't initialized a git repository yet:

```bash
git init
git add .
git commit -m "Initial commit with GHCR support"
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO.git
git branch -M main
git push -u origin main
```

### 2. Enable GitHub Actions Permissions

To allow GitHub Actions to push to GHCR:

1. Go to your GitHub repository
2. Click **Settings** → **Actions** → **General**
3. Scroll to **Workflow permissions**
4. Select **"Read and write permissions"**
5. Check **"Allow GitHub Actions to create and approve pull requests"**
6. Click **Save**

### 3. Verify the Build

1. Go to your repository on GitHub
2. Click on the **Actions** tab
3. You should see "Build and Push to GHCR" workflow running
4. Wait for it to complete (5-15 minutes for first build)
5. Green checkmark = Success! ✅

### 4. Find Your Image

After the workflow completes:

1. Go to your repository main page
2. Look for **Packages** on the right sidebar
3. Click on your package name
4. You'll see all available tags

### 5. Make Image Public (Optional)

By default, GHCR images are private. To make public:

1. Click on the package in GitHub
2. Click **Package settings** (bottom right)
3. Scroll to **Danger Zone**
4. Click **Change visibility** → **Public**
5. Confirm the change

## 🐳 Using Your Image

### Pull and Run

```bash
# Replace YOUR_USERNAME and YOUR_REPO with your GitHub details
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:latest

# Run the container
docker run -d \
  --name db-sync-proxy \
  -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  -e RUST_LOG=info \
  ghcr.io/YOUR_USERNAME/YOUR_REPO:latest --web-ui

# Access the web UI
open http://localhost:5009
```

### Docker Compose

Create or update `docker-compose.yml`:

```yaml
version: '3.8'

services:
  db-sync:
    image: ghcr.io/YOUR_USERNAME/YOUR_REPO:latest
    ports:
      - "5009:5009"
    volumes:
      - ./db_sync_data:/app/data
    environment:
      - RUST_LOG=info
    command: ["--web-ui"]
    restart: unless-stopped
```

Then run:

```bash
docker-compose up -d
```

## 🏷️ Automatic Version Tags

**Every push to master automatically creates a new version tag!** 🎉

After pushing code, these tags are created automatically:

- **`v1.0.x`** - Semantic version (auto-incremented)
- **`1.0.x`** - Version without 'v' prefix
- **`1.0`** - Major.Minor version
- **`1`** - Major version only
- **`latest`** - Latest build from master branch
- **`master`** - Latest master branch build
- **`master-sha-xxxxxxx`** - Specific commit

### Controlling Version Bumps

Use keywords in your commit messages to control versioning:

```bash
# Patch version (default): v1.0.0 → v1.0.1
git commit -m "Fix database connection bug"
git push origin master

# Minor version: v1.0.1 → v1.1.0
git commit -m "Add new scheduling feature #minor"
git push origin master

# Major version: v1.1.0 → v2.0.0
git commit -m "Breaking API changes #major"
git push origin master

# Skip versioning
git commit -m "Update docs #none"
git push origin master
```

**For more details**, see `VERSIONING.md`

## 🔍 Monitoring

### Check Workflow Status

```bash
# View Actions tab on GitHub
https://github.com/YOUR_USERNAME/YOUR_REPO/actions

# Or use GitHub CLI
gh workflow list
gh run list
gh run watch
```

### View Build Logs

1. Go to **Actions** tab
2. Click on a workflow run
3. Click **build-and-push** job
4. Expand steps to see logs

## 🐛 Common Issues

### Build Fails - Permission Denied

**Fix**: Enable "Read and write permissions" in Settings → Actions → General

### Cannot Pull Image

**Fix**: 
- Make image public (see step 5 above)
- Or login: `echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin`

### Workflow Not Triggered

**Fix**: 
- Check you pushed to `main` or `master` branch
- Verify workflow file is in `.github/workflows/` directory
- Check Actions are enabled in repository settings

## 📋 Environment-Specific Deployments

### Development

```bash
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:main
docker run -d -p 5009:5009 ghcr.io/YOUR_USERNAME/YOUR_REPO:main --web-ui
```

### Staging

```bash
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.0.0-beta
docker run -d -p 5009:5009 ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.0.0-beta --web-ui
```

### Production

```bash
# Always use specific version tags in production
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.0.0
docker run -d -p 5009:5009 ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.0.0 --web-ui
```

## 🔄 Update Workflow

When you want to update your deployment:

```bash
# Make code changes
git add .
git commit -m "Update feature X"
git push origin main

# Wait for GitHub Actions to build new image
# Pull and restart container
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:latest
docker stop db-sync-proxy
docker rm db-sync-proxy
docker run -d --name db-sync-proxy -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  ghcr.io/YOUR_USERNAME/YOUR_REPO:latest --web-ui
```

## 🎉 Benefits

✅ **Automated Builds**: Every push triggers a new build  
✅ **Multi-Platform**: Supports AMD64 and ARM64 architectures  
✅ **Version Control**: Track versions with git tags  
✅ **Fast Deployments**: Pull pre-built images instead of building locally  
✅ **Rollback Support**: Easily revert to previous versions  
✅ **CI/CD Ready**: Integrate with deployment pipelines  

## 📚 More Information

- **Detailed Setup**: See `GHCR_SETUP.md` for comprehensive guide
- **Usage Instructions**: See `README.md` for application features
- **Docker Details**: See `Dockerfile` for build process

## 🆘 Need Help?

1. Check `GHCR_SETUP.md` for detailed troubleshooting
2. Review GitHub Actions logs for build errors
3. Verify Docker and GitHub authentication
4. Open an issue in the repository

---

**Ready to deploy?** Follow steps 1-5 above and you'll have automated Docker builds in minutes! 🚀

