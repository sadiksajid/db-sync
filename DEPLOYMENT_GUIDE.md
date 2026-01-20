# 🚀 Quick Deployment Guide - GHCR & Docker Hub

## What Was Set Up

Your DB Sync Proxy project is now configured to automatically build and publish Docker images to **both** GitHub Container Registry (ghcr.io) **and Docker Hub** whenever you push code to GitHub.

### ✅ Files Created/Modified:

1. **`.github/workflows/docker-publish.yml`** - Automated CI/CD pipeline
2. **`.dockerignore`** - Optimizes Docker builds by excluding unnecessary files
3. **`.gitignore`** - Updated to exclude data directories
4. **`README.md`** - Updated with GHCR usage instructions
5. **`GHCR_SETUP.md`** - Detailed GHCR setup and troubleshooting guide
6. **`DEPLOYMENT_GUIDE.md`** - This quick reference (you are here)

## 🎯 Next Steps

### 1. Setup Docker Hub (Required)

Before pushing to GitHub, you need to configure Docker Hub credentials:

#### A. Create Docker Hub Access Token

1. Go to [hub.docker.com](https://hub.docker.com) and login
2. Click your username → **Account Settings** → **Security**
3. Click **New Access Token**
4. Name: `GitHub Actions`, Permissions: **Read, Write, Delete**
5. Click **Generate** and **copy the token** (you won't see it again!)

#### B. Add Secrets to GitHub

1. Go to your repository on GitHub
2. **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**:
   - Name: `DOCKERHUB_USERNAME`
   - Value: `sadiksajid`
   - Click **Add secret**
4. Click **New repository secret** again:
   - Name: `DOCKERHUB_TOKEN`
   - Value: Paste the access token from step A
   - Click **Add secret**

**📚 Detailed Guide**: See `DOCKERHUB_SETUP.md` for complete instructions

### 2. Push to GitHub

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

To allow GitHub Actions to push to GHCR and create version tags:

1. Go to your GitHub repository
2. Click **Settings** → **Actions** → **General**
3. Scroll to **Workflow permissions**
4. Select **"Read and write permissions"**
5. Check **"Allow GitHub Actions to create and approve pull requests"**
6. Click **Save**

### 3. Verify the Build

1. Go to your repository on GitHub
2. Click on the **Actions** tab
3. You should see "Build and Push to GHCR & Docker Hub" workflow running
4. Wait for it to complete (5-15 minutes for first build)
5. Green checkmark = Success! ✅

### 4. View the Release

After the workflow completes:

1. Go to your repository main page
2. Click **Releases** (right sidebar)
3. You'll see the new release (e.g., "Release v1.0.0")
4. Release includes:
   - Docker pull commands
   - Quick start instructions
   - Automatic changelog
   - Links to both registries

Example:
```
Release v1.0.3
Latest
17 hours ago

🚀 Docker Images Available
- Docker Hub: sadiksajid/db-sync:v1.0.3
- GHCR: ghcr.io/OWNER/REPO:v1.0.3
```

### 6. Find Your Images

After the workflow completes, images are available in **two** places:

#### A. GitHub Container Registry (GHCR)
1. Go to your repository main page
2. Look for **Packages** on the right sidebar
3. Click on your package name
4. You'll see all available tags

#### B. Docker Hub
1. Go to [hub.docker.com](https://hub.docker.com)
2. Click on your username
3. You'll see your repository listed
4. Click it to see all tags

### 7. Make Images Public (Optional)

#### Make GHCR Image Public
By default, GHCR images are private. To make public:
1. Click on the package in GitHub
2. Click **Package settings** (bottom right)
3. Scroll to **Danger Zone**
4. Click **Change visibility** → **Public**
5. Confirm the change

#### Make Docker Hub Image Public
By default, Docker Hub images are public. To make private:
1. Go to your repository on Docker Hub
2. Click **Settings** tab
3. Change visibility to **Private** (requires paid plan)

## 🐳 Using Your Images

You can now pull from **either** registry:

### Option 1: Pull from Docker Hub (Recommended for Public Use)

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

# Access the web UI
open http://localhost:5009
```

### Option 2: Pull from GitHub Container Registry

```bash
# Replace YOUR_GITHUB_USERNAME and YOUR_REPO with your GitHub details
docker pull ghcr.io/YOUR_GITHUB_USERNAME/YOUR_REPO:latest

# Run the container
docker run -d \
  --name db-sync-proxy \
  -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  -e RUST_LOG=info \
  ghcr.io/YOUR_GITHUB_USERNAME/YOUR_REPO:latest --web-ui
```

### Docker Compose

Create or update `docker-compose.yml`:

**Using Docker Hub:**
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

**Using GHCR:**
```yaml
version: '3.8'

services:
  db-sync:
    image: ghcr.io/YOUR_GITHUB_USERNAME/YOUR_REPO:latest
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

## 🎯 Which Registry to Use?

### Docker Hub
✅ **Best for**: Public images, traditional Docker users, wider distribution  
✅ **Pros**: Well-known, integrated with Docker CLI, large community  
✅ **Cons**: Rate limits on free tier, requires account setup  

### GitHub Container Registry (GHCR)
✅ **Best for**: Private repos, GitHub-centric workflows, organization packages  
✅ **Pros**: Integrated with GitHub, unlimited bandwidth, free for public repos  
✅ **Cons**: Requires GitHub authentication for private images  

**💡 Tip**: Both registries receive the exact same images with the same tags!

## 🐛 Common Issues

### Build Fails - Permission Denied

**Fix**: Enable "Read and write permissions" in Settings → Actions → General

### Docker Hub Login Fails

**Fix**: 
- Verify `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN` secrets are set correctly
- Regenerate Docker Hub access token
- Ensure token has "Read, Write, Delete" permissions
- Check username is correct (not email)

### Cannot Pull Image

**Fix**: 
- **GHCR**: Make image public or login: `echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin`
- **Docker Hub**: Images are public by default, just pull directly

### Workflow Not Triggered

**Fix**: 
- Check you pushed to `main` or `master` branch
- Verify workflow file is in `.github/workflows/` directory
- Check Actions are enabled in repository settings

### Only Pushes to One Registry

**Fix**:
- Check workflow logs to see which step failed
- For Docker Hub: Verify secrets are set
- For GHCR: Verify GitHub Actions permissions

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
✅ **Auto-Versioning**: Automatic semantic version tags on each push  
✅ **Dual Registry**: Images available on both GHCR and Docker Hub  
✅ **Fast Deployments**: Pull pre-built images instead of building locally (saves 10-15 min)  
✅ **Rollback Support**: Easily revert to previous versions  
✅ **CI/CD Ready**: Integrate with deployment pipelines  
✅ **Free Hosting**: Both registries free for public repos  

## 📚 More Information

- **Docker Hub Setup**: See `DOCKERHUB_SETUP.md` for Docker Hub configuration
- **GHCR Setup**: See `GHCR_SETUP.md` for GitHub Container Registry guide
- **Auto Versioning**: See `VERSIONING.md` for version control details
- **Usage Instructions**: See `README.md` for application features
- **Docker Details**: See `Dockerfile` for build process

## 🆘 Need Help?

1. Check `GHCR_SETUP.md` for detailed troubleshooting
2. Review GitHub Actions logs for build errors
3. Verify Docker and GitHub authentication
4. Open an issue in the repository

---

**Ready to deploy?** Follow steps 1-5 above and you'll have automated Docker builds in minutes! 🚀

