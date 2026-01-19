# 🏷️ Automatic Versioning Guide

This project uses **automatic semantic versioning** - every push to the `master` branch creates a new version tag automatically!

## How It Works

When you push code to the `master` branch:
1. GitHub Actions automatically creates a new version tag
2. The version follows semantic versioning (MAJOR.MINOR.PATCH)
3. Docker images are built with the new version tag
4. Images are pushed to GHCR with multiple tags

## Default Behavior

By default, **every push increments the PATCH version**:

```
v1.0.0 → v1.0.1 → v1.0.2 → v1.0.3 ...
```

## Controlling Version Bumps

You can control which version number gets bumped using **commit message keywords**:

### Patch Version (default)

Any regular commit message:
```bash
git commit -m "Fix bug in database sync"
git push origin master
# Creates: v1.0.1 → v1.0.2
```

### Minor Version

Include `#minor` in your commit message:
```bash
git commit -m "Add new scheduling feature #minor"
git push origin master
# Creates: v1.0.2 → v1.1.0
```

### Major Version

Include `#major` in your commit message:
```bash
git commit -m "Breaking change: Rewrite API #major"
git push origin master
# Creates: v1.1.0 → v2.0.0
```

### Skip Version Bump

Include `#none` in your commit message:
```bash
git commit -m "Update documentation #none"
git push origin master
# No new version tag created
```

## Commit Message Keywords

| Keyword | Version Bump | Example |
|---------|--------------|---------|
| `#major` | MAJOR (v1.0.0 → v2.0.0) | Breaking changes |
| `#minor` | MINOR (v1.0.0 → v1.1.0) | New features |
| `#patch` | PATCH (v1.0.0 → v1.0.1) | Bug fixes (default) |
| `#none` | No bump | Documentation, formatting |

## Docker Image Tags

Each version automatically creates multiple Docker image tags:

```bash
# If new version is v1.2.3, these tags are created:
ghcr.io/OWNER/REPO:v1.2.3    # Full version
ghcr.io/OWNER/REPO:1.2.3     # Without 'v' prefix
ghcr.io/OWNER/REPO:1.2       # Major.Minor
ghcr.io/OWNER/REPO:1         # Major only
ghcr.io/OWNER/REPO:latest    # Latest release
ghcr.io/OWNER/REPO:master    # Latest master build
```

## Examples

### Bug Fix Release

```bash
git add .
git commit -m "Fix connection timeout issue"
git push origin master
# Auto-creates: v1.0.5 → v1.0.6
```

### New Feature Release

```bash
git add .
git commit -m "Add PostgreSQL replication support #minor"
git push origin master
# Auto-creates: v1.0.6 → v1.1.0
```

### Breaking Change Release

```bash
git add .
git commit -m "Rewrite configuration system (breaking change) #major"
git push origin master
# Auto-creates: v1.1.0 → v2.0.0
```

### Documentation Update (No Version)

```bash
git add README.md
git commit -m "Update installation instructions #none"
git push origin master
# No version tag created, only 'latest' and 'master' tags updated
```

## Viewing Versions

### In GitHub

1. Go to your repository
2. Click **Releases** tab
3. See all versions with timestamps

### Using Git

```bash
# List all version tags
git tag -l "v*"

# Show latest tag
git describe --tags --abbrev=0

# Show tag with commit info
git log --oneline --decorate
```

### In GitHub Actions

1. Go to **Actions** tab
2. Click on a workflow run
3. Look for "Display new version" step
4. See the new version that was created

### Using GitHub CLI

```bash
# List all releases
gh release list

# View latest release
gh release view
```

## Pulling Specific Versions

```bash
# Latest version
docker pull ghcr.io/OWNER/REPO:latest

# Specific version
docker pull ghcr.io/OWNER/REPO:v1.2.3

# Major version (gets latest 1.x.x)
docker pull ghcr.io/OWNER/REPO:1
```

## Initial Version

The first push to master will create **v1.0.0** if no tags exist yet.

## Best Practices

### 1. Use Meaningful Commit Messages

```bash
# Good
git commit -m "Add authentication to web UI #minor"

# Bad
git commit -m "updates"
```

### 2. Version Strategy

- **PATCH** (v1.0.x): Bug fixes, small improvements
- **MINOR** (v1.x.0): New features, backward-compatible changes
- **MAJOR** (vx.0.0): Breaking changes, major rewrites

### 3. Production Deployments

Always use specific version tags in production:

```bash
# Good - predictable
docker pull ghcr.io/OWNER/REPO:v1.2.3

# Bad - could change unexpectedly
docker pull ghcr.io/OWNER/REPO:latest
```

### 4. Testing Workflow

```bash
# 1. Develop locally
git add feature.rs
git commit -m "Add new feature"

# 2. Push to feature branch first
git checkout -b feature/my-feature
git push origin feature/my-feature

# 3. Test in CI/CD (PR builds)

# 4. Merge to master when ready
git checkout master
git merge feature/my-feature
git push origin master

# 5. Version tag is created automatically!
```

## Troubleshooting

### No Tag Created

**Problem**: Pushed to master but no version tag appeared

**Solutions**:
- Check if commit message contains `#none`
- Verify workflow permissions (needs `contents: write`)
- Check GitHub Actions logs for errors
- Ensure you're on `master` or `main` branch

### Wrong Version Bump

**Problem**: Expected minor bump but got patch

**Solutions**:
- Check commit message contains correct keyword (`#minor`)
- Keyword must be in the commit message
- Case-sensitive: use lowercase `#minor`, not `#Minor`

### Duplicate Tags

**Problem**: Tag already exists

**Solutions**:
- Don't manually create tags with same names
- Delete duplicate tag: `git tag -d v1.0.0 && git push origin :refs/tags/v1.0.0`
- Let the automation handle versioning

## Manual Override

If you need to create a specific version manually:

```bash
# Create tag locally
git tag v2.5.0

# Push tag to GitHub
git push origin v2.5.0

# This will build v2.5.0 without auto-increment
# Next auto-increment will be v2.5.1
```

## Workflow Summary

```
┌─────────────────────┐
│  Push to master     │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Auto-create version │
│ (based on commit)   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Build Docker image  │
│ with version tags   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Push to GHCR        │
│ ghcr.io/OWNER/REPO  │
└─────────────────────┘
```

## Summary

✅ **Automatic**: No manual version management  
✅ **Semantic**: Follows semver standard  
✅ **Flexible**: Control with commit messages  
✅ **Traceable**: Every version tied to a commit  
✅ **Docker-ready**: Auto-tagged images  

---

**Need help?** Check the GitHub Actions logs under the "Auto-increment version tag" step to see what version was created!

