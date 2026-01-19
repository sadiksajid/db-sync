# GitHub Container Registry (GHCR) Setup Guide

This guide explains how to set up automated Docker image builds and publishing to GitHub Container Registry (ghcr.io) for this project.

## 🚀 Quick Start

The project is already configured with GitHub Actions to automatically build and push Docker images to GHCR. Here's what you need to know:

## 📋 Prerequisites

1. **GitHub Repository**: This project must be hosted on GitHub
2. **GitHub Packages**: Enabled by default for all repositories
3. **GitHub Token**: Automatically provided by GitHub Actions (no setup needed)

## 🔧 Initial Setup

### 1. Push Your Code to GitHub

If you haven't already, push this repository to GitHub:

```bash
# Initialize git repository (if not already done)
git init

# Add all files
git add .

# Commit changes
git commit -m "Initial commit with GHCR setup"

# Add remote repository
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO.git

# Push to GitHub
git push -u origin main
```

### 2. Enable GitHub Actions

GitHub Actions is enabled by default. The workflow will trigger automatically on:
- Push to `main` or `master` branch
- Creating version tags (e.g., `v1.0.0`)
- Pull requests
- Manual workflow dispatch

### 3. Verify Workflow Execution

1. Go to your GitHub repository
2. Click on **Actions** tab
3. You should see "Build and Push to GHCR" workflow running
4. Wait for the workflow to complete (usually 5-15 minutes)

## 📦 Using the Published Images

### Pull the Image

Once the workflow completes, you can pull your image:

```bash
# Pull latest version
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:latest

# Pull specific version
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.0.0

# Pull specific commit
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:main-sha-abc1234
```

### Run the Container

```bash
docker run -d \
  --name db-sync-proxy \
  -p 5009:5009 \
  -v ./db_sync_data:/app/data \
  -e RUST_LOG=info \
  ghcr.io/YOUR_USERNAME/YOUR_REPO:latest --web-ui
```

### Docker Compose

Update your `docker-compose.yml` to use the GHCR image:

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

## 🏷️ Image Tagging Strategy

The GitHub Actions workflow automatically creates the following tags:

### 1. Branch Tags
- `main` - Latest commit from main branch
- `master` - Latest commit from master branch

### 2. Latest Tag
- `latest` - Points to the latest commit on the default branch

### 3. Version Tags (when pushing git tags)
```bash
# Create and push a version tag
git tag v1.0.0
git push origin v1.0.0

# This creates tags:
# - ghcr.io/OWNER/REPO:v1.0.0
# - ghcr.io/OWNER/REPO:1.0.0
# - ghcr.io/OWNER/REPO:1.0
# - ghcr.io/OWNER/REPO:1
```

### 4. Commit SHA Tags
- `main-sha-abc1234` - Specific commit reference

## 🔒 Access Control

### Public Images

By default, GHCR images are private. To make them public:

1. Go to your GitHub profile
2. Click on **Packages** tab
3. Click on your package (repository name)
4. Click **Package settings** (bottom right)
5. Scroll down to **Danger Zone**
6. Click **Change visibility** → **Public**

### Private Images

To pull private images, you need to authenticate:

```bash
# Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin

# Pull private image
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:latest
```

### Generate Personal Access Token

If you need a token for CI/CD or local development:

1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click **Generate new token (classic)**
3. Select scopes: `read:packages` (for pulling), `write:packages` (for pushing)
4. Generate and save the token

```bash
# Use the token
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin
```

## 🛠️ Workflow Details

The GitHub Actions workflow (`.github/workflows/docker-publish.yml`) performs the following:

1. **Checkout Code**: Clones the repository
2. **Setup Docker Buildx**: Enables multi-platform builds
3. **Login to GHCR**: Authenticates using `GITHUB_TOKEN`
4. **Extract Metadata**: Generates tags and labels
5. **Build & Push**: Builds Docker image for multiple architectures (amd64, arm64)
6. **Cache**: Uses GitHub Actions cache for faster builds

### Supported Platforms

The workflow builds images for:
- `linux/amd64` - Standard x86_64 systems
- `linux/arm64` - ARM-based systems (Apple Silicon, AWS Graviton, Raspberry Pi)

## 🔄 Manual Workflow Trigger

You can manually trigger the workflow:

1. Go to **Actions** tab in your repository
2. Click on **Build and Push to GHCR** workflow
3. Click **Run workflow** button
4. Select branch and click **Run workflow**

## 📊 Monitoring Builds

### Check Workflow Status

```bash
# Using GitHub CLI
gh workflow view "Build and Push to GHCR"
gh run list --workflow="Build and Push to GHCR"
gh run watch
```

### View Logs

1. Go to **Actions** tab
2. Click on a workflow run
3. Click on **build-and-push** job
4. Expand steps to see detailed logs

## 🐛 Troubleshooting

### Workflow Fails to Authenticate

**Error**: `denied: permission_denied`

**Solution**: 
- Ensure GitHub Actions has write permissions
- Go to Repository Settings → Actions → General
- Under "Workflow permissions", select "Read and write permissions"

### Image Build Fails

**Error**: Build timeouts or out of memory

**Solution**:
- GitHub Actions runners have limited resources
- Consider optimizing Dockerfile
- Use multi-stage builds (already implemented)
- Cache dependencies effectively

### Cannot Pull Image

**Error**: `denied: access forbidden`

**Solution**:
- Check if image is public (see Access Control section)
- Verify you're authenticated: `docker login ghcr.io`
- Ensure token has `read:packages` scope

### Wrong Image Tag

**Error**: Expected tag not created

**Solution**:
- Check the metadata step in workflow logs
- Verify git tag format (should be `vX.Y.Z`)
- Ensure you pushed tags: `git push --tags`

## 🔐 Security Best Practices

1. **Never commit secrets**: Use GitHub Secrets for sensitive data
2. **Use specific versions**: Pin image versions in production
3. **Scan images**: Enable Dependabot and security scanning
4. **Minimal permissions**: Use least-privilege tokens
5. **Review workflows**: Audit GitHub Actions before running

## 📚 Additional Resources

- [GitHub Container Registry Documentation](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Docker Build Push Action](https://github.com/docker/build-push-action)
- [Docker Metadata Action](https://github.com/docker/metadata-action)

## 🎯 Example Workflows

### Development Workflow

```bash
# Make changes
git add .
git commit -m "Add new feature"
git push origin main

# Wait for workflow to complete
# Use latest image with your changes
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:latest
```

### Release Workflow

```bash
# Create release
git tag v1.2.0
git push origin v1.2.0

# Wait for workflow to complete
# Pull production image
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.2.0

# Update production
docker service update --image ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.2.0 db-sync
```

### Rollback Workflow

```bash
# Rollback to previous version
docker pull ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.1.0
docker service update --image ghcr.io/YOUR_USERNAME/YOUR_REPO:v1.1.0 db-sync
```

## 💡 Tips

1. **Tag Strategy**: Use semantic versioning for releases (v1.0.0, v1.0.1)
2. **Testing**: Test workflow in a fork or feature branch first
3. **Documentation**: Keep README updated with current image names
4. **Automation**: Use Dependabot to keep dependencies updated
5. **Monitoring**: Set up notifications for failed workflows

## ✅ Checklist

- [ ] Repository pushed to GitHub
- [ ] GitHub Actions enabled
- [ ] Workflow permissions set to "Read and write"
- [ ] First workflow run completed successfully
- [ ] Image visible in GitHub Packages
- [ ] Image visibility set (public/private)
- [ ] README updated with correct image names
- [ ] docker-compose.yml updated to use GHCR image
- [ ] Successfully pulled and tested image locally
- [ ] Documentation updated for team members

---

**Need Help?** Open an issue in the repository or check GitHub Actions logs for detailed error messages.

