---
description: Prepare and publish a new release following the project's release process
allowed-tools: Bash, Read, Edit, Write, Grep, Glob, AskUserQuestion
---

# Release Process

Follow these steps to prepare and publish a new release for LaReview.

## Step 1: Determine the new version

1. Read the current version from `Cargo.toml`:
   ```bash
   grep '^version = ' Cargo.toml
   ```

2. Ask the user what the new version should be (suggest incrementing the patch version).

## Step 2: Run quality checks

Run all quality checks to ensure the codebase is ready for release.

### Backend checks:
```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

### Frontend checks:
```bash
cd frontend && pnpm lint && pnpm test && pnpm build
```

If any checks fail, report the failures and ask the user if they want to fix them before continuing.

## Step 3: Gather changes for changelog

1. Get the list of commits since the last release tag:
   ```bash
   git log $(git describe --tags --abbrev=0)..HEAD --oneline
   ```

2. Categorize changes into:
   - **Added**: New features and capabilities
   - **Changed**: Modifications to existing functionality
   - **Fixed**: Bug fixes
   - **Removed**: Removed features (if any)

3. Present the proposed changelog entry to the user for review/modification.

## Step 4: Update version files

Update the version in the following files:

1. **Cargo.toml** (line with `version = "X.X.X"`):
   - Use the Edit tool to update the version string

2. **tauri.conf.json** (line with `"version": "X.X.X"`):
   - Use the Edit tool to update the version string

3. **Sync Cargo.lock** by running:
   ```bash
   cargo build
   ```

## Step 5: Update CHANGELOG.md

1. Read the current `CHANGELOG.md`
2. Add a new entry at the top (after the header), following this format:

```markdown
## [X.X.X] - YYYY-MM-DD

### Added

- **Scope**: Description of new feature.

### Changed

- **Scope**: Description of change.

### Fixed

- **Scope**: Description of fix.
```

Use today's date in YYYY-MM-DD format. Only include sections that have changes.

## Step 6: Confirm with user

Present to the user:
1. The new version number
2. Files that will be modified
3. The changelog entry
4. Summary of quality check results

Ask for final confirmation before creating the release commit.

## Step 7: Create release commit and tag

Once confirmed:

1. Stage all changes:
   ```bash
   git add -A
   ```

2. Create the release commit:
   ```bash
   git commit -m "chore(release): v{VERSION}"
   ```

3. Create an annotated tag with the changelog summary:
   ```bash
   git tag -a v{VERSION} -m "v{VERSION}

   {CHANGELOG_SUMMARY}"
   ```

4. Show the commit and tag:
   ```bash
   git log -1 --format="%h %s"
   git tag -l -n10 v{VERSION}
   ```

## Step 8: Push (optional)

Ask the user if they want to push the release:
```bash
git push && git push --tags
```

Note: GitHub CI will automatically handle builds, releases, and Homebrew tap updates once pushed.

## Artifacts Generated

The release workflow will generate the following artifacts:

### macOS
- `LaReview_aarch64.app.tar.gz` - Apple Silicon app bundle
- `LaReview_x64.app.tar.gz` - Intel app bundle
- `.dmg` installers for both architectures

### Linux
- `LaReview_${VERSION}_amd64.AppImage` - AppImage format (portable)
- `lareview-linux.tar.gz` - Tarball with binary executable
- `.deb` package (Debian/Ubuntu)

---

## Important Notes

- Never push without explicit user confirmation
- If any step fails, stop and report the issue
- The version format is `0.0.X` (we're pre-1.0)
- Always run quality checks before bumping version
- Keep changelog entries concise but descriptive
