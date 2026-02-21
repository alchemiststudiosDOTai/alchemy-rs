# GitHub Security Features Setup

This document explains how to enable GitHub security features for the Alchemy repository.

## Quick Setup Commands

Run these commands with `gh` CLI to enable security features:

```bash
# Enable secret scanning
gh api repos/tunahorse/alchemy-rs -X PATCH -F security_and_analysis='{"secret_scanning":{"status":"enabled"}}'

# Enable secret scanning push protection
gh api repos/tunahorse/alchemy-rs -X PATCH -F security_and_analysis='{"secret_scanning_push_protection":{"status":"enabled"}}'

# Enable code scanning (CodeQL) - creates default setup
gh api repos/tunahorse/alchemy-rs/code-scanning/default-setup -X PATCH -F state='configured' -F languages='["rust"]' -F query_suite='default' -F schedule='weekly'

# Enable dependency graph (required for Dependabot alerts)
gh api repos/tunahorse/alchemy-rs/dependency-graph/snapshots -X POST

# Enable automated security fixes
gh api repos/tunahorse/alchemy-rs/automated-security-fixes -X PUT
```

## Manual Setup (GitHub Web UI)

If you prefer using the GitHub web interface:

### 1. Secret Scanning

1. Go to **Settings** → **Security** → **Secret scanning**
2. Click **Enable** for:
   - **Secret scanning**
   - **Push protection** (prevents commits with secrets)

### 2. Code Scanning (CodeQL)

1. Go to **Settings** → **Security** → **Code scanning**
2. Click **Set up** → **Default**
3. Select **Rust** as the language
4. Choose **Default** query suite
5. Set schedule to **Weekly**
6. Click **Enable CodeQL**

### 3. Branch Protection Rules

1. Go to **Settings** → **Branches**
2. Click **Add rule** next to `main`
3. Enable:
   - ☑️ **Require a pull request before merging**
     - ☑️ **Require approvals** (set to 1)
     - ☑️ **Dismiss stale PR approvals when new commits are pushed**
   - ☑️ **Require status checks to pass**
     - Search for and add:
       - `cargo check`
       - `cargo clippy`
       - `cargo test`
   - ☑️ **Require branches to be up to date before merging**
   - ☑️ **Require conversation resolution before merging**
   - ☐ **Allow force pushes** (keep disabled)
   - ☐ **Allow deletions** (keep disabled)

### 4. Dependabot Alerts

1. Go to **Settings** → **Security** → **Dependabot**
2. Click **Enable** for:
   - **Dependabot alerts**
   - **Dependabot security updates**

Already configured:
- ✅ **Dependabot version updates** (via `.github/dependabot.yml`)

## Verification

After setup, verify features are enabled:

```bash
# Check security features status
gh api repos/tunahorse/alchemy-rs --jq '.security_and_analysis'

# Check branch protection
gh api repos/tunahorse/alchemy-rs/branches/main/protection

# Check code scanning
gh api repos/tunahorse/alchemy-rs/code-scanning/default-setup
```

## Security Policy

The `SECURITY.md` file in this directory defines:
- How to report vulnerabilities
- Supported versions
- Security features in use

## Recommendations

1. **Enable all features immediately** - They're free for public repos
2. **Review alerts weekly** - Check Security tab for new findings
3. **Update dependencies monthly** - Dependabot will open PRs
4. **Require PR reviews** - Never push directly to `main`

## Troubleshooting

### Secret scanning shows "disabled" after enabling
Wait 5-10 minutes and refresh. GitHub's API may show stale data.

### CodeQL not running on PRs
Check that GitHub Actions is enabled:
Settings → Actions → General → Allow all actions and reusable workflows

### Branch protection blocking admin pushes
This is expected. Use PRs even for admin changes.

## Additional Resources

- [GitHub Security Features](https://docs.github.com/en/code-security)
- [CodeQL for Rust](https://docs.github.com/en/code-security/code-scanning/creating-an-advanced-setup-for-code-scanning/codeql-code-scanning-for-compiled-languages)
- [Dependabot Configuration](https://docs.github.com/en/code-security/dependabot/dependabot-version-updates/configuration-options-for-the-dependabot.yml-file)
