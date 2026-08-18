# Monorepo Sychronized Versioning — Google Release-Please

**Date:** 2026-08-18
**Status:** Approved
**Goal:** Automate package versioning and release notes using Conventional Commits and Google Release-Please, targeting `develop` and `main` branches to support both pre-releases (testing/nightly) and official releases.

## Problem

Currently, releases are manually coordinated or tied to specific release branches (like `release-v0.2.0`). Version numbers are patched dynamically in CI based on the branch name, making it hard to maintain consistent, automatic semver numbering and a unified changelog across multiple monorepo components (`thiscloudd`, `thiscloud-cli`, `web-ui`, etc.).

Furthermore, testing builds (which cannot be fully simulated in CI because they require live virtual machine installations, ISO assembly, and package deployment) should be distinguished from stable official releases.

## Approach: Google Release-Please on `develop` and `main`

We configure Google Release-Please as a synchronized monorepo workspace.
- **`develop` branch:** Dispatches pre-releases (e.g. `0.4.0-alpha.1`) for integration testing, ISO assembly, and package updates.
- **`main` branch:** Dispatches official stable releases (e.g. `0.4.0`).

### Workspace Configuration

We define a configuration to keep all packages/crates synchronized under the same version (linked-versions group):

1. **`release-please-config.json`**
```json
{
  "packages": {
    "platform": {
      "release-type": "rust",
      "package-name": "thiscloud-platform"
    },
    "platform/web-ui": {
      "release-type": "node",
      "package-name": "thiscloud-web"
    }
  },
  "plugins": [
    {
      "type": "linked-versions",
      "groupName": "thiscloud",
      "packages": [
        "platform",
        "platform/web-ui"
      ]
    }
  ]
}
```

2. **`.release-please-manifest.json`**
```json
{
  "platform": "0.3.0",
  "platform/web-ui": "0.3.0"
}
```

### Flow and Automation

- On any push to `develop` or `main`:
  - Run the `release-please-action` in dry-run/PR creation mode.
  - Release-Please parses commits following **Conventional Commits** (`feat:`, `fix:`, `chore:`, etc.) since the last release tag.
  - It creates/updates a **Release Pull Request** targeting the respective branch (e.g., a PR to merge `release-please--branches--develop` into `develop`).
  - The PR automatically updates:
    - `platform/Cargo.toml`
    - `platform/web-ui/package.json`
    - `CHANGELOG.md` (unified changelog)

- When the PR is merged:
  - Release-Please automatically tags the merge commit (e.g., `v0.4.0-alpha.1` on `develop`, or `v0.4.0` on `main`).
  - This tag triggers the building and publishing workflow (`release.yml` and `iso.yml`).
  - This ensures testing packages and ISOs are built with precise pre-release versions for verification on bare metal.

> Changelogs are generated per package (`platform/CHANGELOG.md`,
> `platform/web-ui/CHANGELOG.md`) by the linked-versions group PR, then
> combined into the single `v*` GitHub Release.
