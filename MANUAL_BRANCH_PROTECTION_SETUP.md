# Manual Branch Protection Setup Guide

## 🔒 GitHub Branch Protection Setup

Since the automated GitHub Actions approach requires admin permissions that aren't available, here's how to manually set up branch protection for the main branch:

## Step 1: Navigate to Repository Settings

1. Go to the repository: https://github.com/ruvnet/code-mesh
2. Click on the **Settings** tab (you need admin access)
3. In the left sidebar, click on **Branches**

## Step 2: Add Branch Protection Rule

1. Click **Add protection rule**
2. In the **Branch name pattern** field, enter: `main`

## Step 3: Configure Protection Settings

### Required Settings:
- ✅ **Restrict pushes that create files larger than 100 MB**
- ✅ **Require a pull request before merging**
  - ✅ **Require approvals** (set to 1)
  - ✅ **Dismiss stale pull request approvals when new commits are pushed**
  - ❌ **Require review from code owners** (optional, enable if you have CODEOWNERS file)
  - ❌ **Restrict reviews to users with write access** (optional)

- ✅ **Require status checks to pass before merging**
  - ✅ **Require branches to be up to date before merging**
  - **Add the following status checks when they become available:**
    - `Test Suite`
    - `Linting`
    - `Build`
    - `WASM Build`
    - `Security Audit`
    - `Code Coverage`

### Security Settings:
- ✅ **Require conversation resolution before merging**
- ✅ **Require signed commits** (optional but recommended)
- ❌ **Require linear history** (optional)
- ✅ **Do not allow bypassing the above settings**
- ✅ **Restrict pushes that create files larger than 100 MB**

### Force Push Settings:
- ❌ **Allow force pushes** (DISABLE this for security)
- ❌ **Allow deletions** (DISABLE this for security)

## Step 4: Repository Settings

Also configure these repository-wide settings in **Settings > General**:

### Pull Requests:
- ✅ **Allow merge commits**
- ✅ **Allow squash merging** 
- ✅ **Allow rebase merging**
- ✅ **Always suggest updating pull request branches**
- ✅ **Automatically delete head branches**

### Pushes:
- ❌ **Limit pushes to collaborators**

## Step 5: Additional Security (Optional)

### Security & Analysis:
- ✅ **Dependency graph**
- ✅ **Dependabot alerts** 
- ✅ **Dependabot security updates**
- ✅ **Dependabot version updates**
- ✅ **Code scanning alerts**
- ✅ **Secret scanning alerts**

## Verification

Once configured, the branch protection will:

1. **Prevent direct pushes to main** - All changes must go through PRs
2. **Require PR reviews** - At least 1 approval required
3. **Require status checks** - CI/CD must pass before merging
4. **Prevent force pushes** - History cannot be rewritten
5. **Prevent branch deletion** - Main branch cannot be deleted
6. **Auto-delete merged branches** - Keeps repository clean

## Expected Status Checks

When the CI/CD pipeline runs, these status checks should appear:

- ✅ **Test Suite** - Rust tests across multiple platforms
- ✅ **Linting** - Clippy and rustfmt checks
- ✅ **Build** - Compilation verification
- ✅ **WASM Build** - WebAssembly compilation
- ✅ **Security Audit** - Dependency vulnerability scanning
- ✅ **Code Coverage** - Test coverage reporting

## Alternative: GitHub CLI (if you have admin token)

If you have a GitHub personal access token with admin permissions:

```bash
# Create a personal access token with admin:repo_hook and repo scopes
export GITHUB_TOKEN="your_admin_token"

# Enable branch protection
gh api repos/:owner/:repo/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["Test Suite","Linting","Build","WASM Build","Security Audit"]}' \
  --field enforce_admins=false \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
  --field restrictions=null \
  --field allow_force_pushes=false \
  --field allow_deletions=false
```

## Current Status

The repository now has:
- ✅ **Complete CI/CD pipeline** with comprehensive testing
- ✅ **Docker support** with multi-stage builds
- ✅ **Security scanning** with audit and vulnerability checks
- ✅ **Performance benchmarking** with trend analysis
- ✅ **Multi-platform builds** (Linux, Windows, macOS, WASM)
- ✅ **Automated releases** with artifact publishing

**Pending**: Manual branch protection setup via GitHub web interface.

---

Once branch protection is enabled, the main branch will be fully secured against:
- Direct pushes without review
- Force pushes that rewrite history  
- Branch deletion
- Merging without passing CI/CD checks

This ensures code quality and prevents accidental breaking changes to the main branch.