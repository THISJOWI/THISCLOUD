# Release-Please Monorepo Versioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automate synchronized semver versioning for the THISCLOUD monorepo using Google Release-Please on `develop` (pre-releases for testing) and `main` (stable releases), replacing the manual `release-v*` branch flow.

**Architecture:** `develop` merges produce `vX.Y.Z-alpha.N` pre-release tags; `main` merges produce `vX.Y.Z` stable tags. Release-Please opens a release PR that bumps `platform/Cargo.toml`, `platform/web-ui/package.json` and changelogs in sync (linked-versions plugin). Merging the PR creates the tag + GitHub Release. A tag-triggered `release.yml` builds RPMs/binary/webui assets and uploads them to that release. `iso.yml` already triggers on `v*` tags.

**Tech Stack:** GitHub Actions, googleapis/release-please-action v4, Conventional Commits.

**Design spec:** `docs/superpowers/specs/2026-08-18-release-please-design.md`

## Global Constraints

- Repo: `THISJOWI/THISCLOUD`, default branch `main`, dev branch `develop`.
- All components must share ONE version (linked-versions group `thiscloud`).
- Git tags MUST be `v<semver>` (NOT `component-v<semver>`) — `iso.yml` and `thiscloud update` depend on the `v*` / plain-tag shape. Requires `include-component-in-tag: false`.
- Version source of truth: `platform/Cargo.toml` `[workspace.package] version` (currently `0.3.0`). Member crates use `version.workspace = true`.
- Current versions to seed the manifest: `platform` = `0.3.0`, `platform/web-ui` = `0.3.0` (bump `web-ui` from `0.1.0`).
- `develop` config must set `prerelease: true` + `prerelease-type: "alpha"` so testing artifacts are pre-releases.
- `thiscloud update` (CLI) reads `releases/latest`, which excludes pre-releases — alpha builds are consumed via direct download/ISO, never auto-updated.
- Conventional Commits required on `develop`/`main` (`feat:`, `fix:`, `chore:`, `docs:`, `!`/`BREAKING CHANGE` for breaking).

---

### Task 1: Release-Please configs + manifest + web-ui version sync

**Files:**
- Create: `release-please-config.json` (repo root, stable config for `main`)
- Create: `release-please-config.develop.json` (repo root, prerelease config for `develop`)
- Create: `.release-please-manifest.json` (repo root)
- Modify: `platform/web-ui/package.json` (version `0.1.0` → `0.3.0`)
- Modify: `platform/web-ui/package-lock.json` (root `version` `0.1.0` → `0.3.0`)

**Interfaces:**
- Produces: `release-please-config.json` and `release-please-config.develop.json` consumed by the workflow in Task 2. `.release-please-manifest.json` seeds both packages at `0.3.0`.

- [ ] **Step 1: Create stable config `release-please-config.json`**

```json
{
  "include-component-in-tag": false,
  "bump-minor-pre-major": true,
  "packages": {
    "platform": {
      "release-type": "rust",
      "component": "platform",
      "package-name": "thiscloud-platform"
    },
    "platform/web-ui": {
      "release-type": "node",
      "component": "web-ui",
      "package-name": "thiscloud-web"
    }
  },
  "plugins": [
    {
      "type": "cargo-workspace",
      "merge": false
    },
    {
      "type": "linked-versions",
      "groupName": "thiscloud",
      "components": ["platform", "web-ui"]
    }
  ]
}
```

- [ ] **Step 2: Create prerelease config `release-please-config.develop.json`**

Same as Task 1 Step 1, plus top-level `prerelease` keys:

```json
{
  "include-component-in-tag": false,
  "bump-minor-pre-major": true,
  "prerelease": true,
  "prerelease-type": "alpha",
  "packages": {
    "platform": {
      "release-type": "rust",
      "component": "platform",
      "package-name": "thiscloud-platform"
    },
    "platform/web-ui": {
      "release-type": "node",
      "component": "web-ui",
      "package-name": "thiscloud-web"
    }
  },
  "plugins": [
    {
      "type": "cargo-workspace",
      "merge": false
    },
    {
      "type": "linked-versions",
      "groupName": "thiscloud",
      "components": ["platform", "web-ui"]
    }
  ]
}
```

- [ ] **Step 3: Create `.release-please-manifest.json`**

```json
{
  "platform": "0.3.0",
  "platform/web-ui": "0.3.0"
}
```

- [ ] **Step 4: Bump web-ui versions to match the shared baseline**

In `platform/web-ui/package.json` change `"version": "0.1.0"` → `"version": "0.3.0"`.
In `platform/web-ui/package-lock.json` change the root `"version": "0.1.0"` → `"version": "0.3.0"`.

- [ ] **Step 5: Validate JSON configs**

Run:
```bash
python3 -m json.tool release-please-config.json > /dev/null && echo OK
python3 -m json.tool release-please-config.develop.json > /dev/null && echo OK
python3 -m json.tool .release-please-manifest.json > /dev/null && echo OK
```
Expected: three `OK` lines, no parse errors.

- [ ] **Step 6: Commit**

```bash
git add release-please-config.json release-please-config.develop.json .release-please-manifest.json platform/web-ui/package.json platform/web-ui/package-lock.json
git commit -m "feat(release): add release-please configs and sync web-ui baseline version"
```

---

### Task 2: Release-Please GitHub Action workflow

**Files:**
- Create: `.github/workflows/release-please.yml`

**Interfaces:**
- Consumes: configs from Task 1.
- Produces: release PRs on `develop`/`main`; on merge, `v*` tags + GitHub Releases that trigger `release.yml` (Task 3) and `iso.yml`.

- [ ] **Step 1: Create the workflow**

```yaml
name: release-please

on:
  push:
    branches:
      - develop
      - main

permissions:
  contents: write
  pull-requests: write

concurrency:
  group: release-please-${{ github.ref }}
  cancel-in-progress: true

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          release-type: simple
          config-file: ${{ github.ref_name == 'main' && 'release-please-config.json' || 'release-please-config.develop.json' }}
          manifest-file: .release-please-manifest.json
          target-branch: ${{ github.ref_name }}
```

- [ ] **Step 2: Validate YAML**

Run:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release-please.yml')); print('YAML OK')"
```
Expected: `YAML OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-please.yml
git commit -m "ci(release): add release-please workflow for develop and main"
```

---

### Task 3: Rewrite `release.yml` to tag-triggered asset builder + add `ci.yml`

**Files:**
- Modify: `.github/workflows/release.yml` (remove test jobs + branch version derivation + release creation; trigger on `v*` tags; upload to the existing release-please release)
- Create: `.github/workflows/ci.yml` (moved test jobs run on push to `develop`/`main` and on PRs)

**Interfaces:**
- Consumes: `v*` tags produced by Task 2.
- Produces: release assets (`*.rpm`, `thiscloud-api-linux-amd64`, `thiscloud-webui.tar.gz`, `thiscloud-systemd.tar.gz`, `manifest.json`) attached to the release-please GitHub Release. `iso.yml` (unchanged) builds the ISO from the same tag.

- [ ] **Step 1: Create `.github/workflows/ci.yml` (tests only)**

```yaml
name: ci

on:
  push:
    branches: [develop, main]
  pull_request:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Install protoc + etcd
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler etcd-server
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: platform
      - run: cargo test --workspace
        working-directory: platform
      - run: cargo clippy --all-targets -- -D warnings
        working-directory: platform

  go:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: "1.22"
      - run: go test ./...
        working-directory: platform/go-api

  web-ui:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm ci
        working-directory: platform/web-ui
      - run: npm test
        working-directory: platform/web-ui
      - run: npm run lint
        working-directory: platform/web-ui

  openapi:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Lint OpenAPI spec
        run: npx --yes @stoplight/spectral-cli lint docs/api/openapi.yaml -r .spectral.yaml -F error
```

- [ ] **Step 2: Rewrite `.github/workflows/release.yml`**

Replace the whole file with:

```yaml
name: release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  build-and-publish:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install system deps
        run: |
          sudo apt-get update
          sudo apt-get install -y protobuf-compiler musl-tools

      - name: Rust toolchain (musl target)
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl

      - name: Set up Go
        uses: actions/setup-go@v5
        with:
          go-version: "1.22"

      - name: Set up Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "20"
          cache: npm
          cache-dependency-path: platform/web-ui/package-lock.json

      - name: Derive version from tag
        id: version
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "Version: $VERSION"

      - name: Cross-compile Rust (musl)
        working-directory: platform
        env:
          CC_x86_64_unknown_linux_musl: musl-gcc
          CXX_x86_64_unknown_linux_musl: musl-gcc
        run: |
          rustup target add x86_64-unknown-linux-musl
          cargo build --release --target x86_64-unknown-linux-musl

      - name: Build RPMs
        working-directory: platform
        run: |
          cargo install cargo-generate-rpm --locked
          mkdir -p target/release
          cp -f target/x86_64-unknown-linux-musl/release/thiscloudd target/release/thiscloudd
          cp -f target/x86_64-unknown-linux-musl/release/thiscloud target/release/thiscloud
          cargo generate-rpm --target x86_64-unknown-linux-musl -p thiscloudd
          cargo generate-rpm --target x86_64-unknown-linux-musl -p thiscloud-cli
          ls -lh target/x86_64-unknown-linux-musl/generate-rpm/

      - name: Build Go API binary
        working-directory: platform/go-api
        run: |
          mkdir -p ../../dist
          CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o ../../dist/thiscloud-api-linux-amd64 ./cmd/api-server

      - name: Build web-ui (Next.js standalone)
        working-directory: platform/web-ui
        run: |
          npm ci
          npm run build

      - name: Stage release assets
        working-directory: platform
        run: |
          mkdir -p dist
          cp -f target/x86_64-unknown-linux-musl/generate-rpm/*.rpm dist/

          # Web UI standalone tarball (server.js at tarball root, matching
          # /usr/share/thiscloud/web-ui layout on the target).
          mkdir -p dist/webui
          # NB: copy with `/.` not `/*` — the glob would skip dotfiles
          # (.next/ with BUILD_ID + server build), producing a tarball that
          # fails to boot with "Could not find a production build".
          cp -a web-ui/.next/standalone/. dist/webui/
          mkdir -p dist/webui/.next
          cp -a web-ui/.next/static dist/webui/.next/static
          [ -d web-ui/public ] && cp -a web-ui/public dist/webui/public
          cp -f web-ui/package.json dist/webui/
          tar -czf dist/thiscloud-webui.tar.gz -C dist/webui .
          rm -rf dist/webui

          # systemd unit tarball
          tar -czf dist/thiscloud-systemd.tar.gz -C iso/systemd .

          echo "==> Staged assets:"
          ls -lh dist/

      - name: Generate manifest.json
        working-directory: platform/dist
        env:
          RELEASE_VERSION: ${{ steps.version.outputs.version }}
        run: |
          node - <<'EOF'
          const fs = require("fs");
          const crypto = require("crypto");
          const files = fs.readdirSync(".").filter(f => f !== "manifest.json");
          const assets = files.map(name => {
            const data = fs.readFileSync(name);
            const sha256 = crypto.createHash("sha256").update(data).digest("hex");
            return { name, sha256 };
          });
          const manifest = {
            version: process.env.RELEASE_VERSION,
            release_date: new Date().toISOString().slice(0, 10),
            assets,
          };
          fs.writeFileSync("manifest.json", JSON.stringify(manifest, null, 2));
          console.log(JSON.stringify(manifest, null, 2));
          EOF

      - name: Upload assets to release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: v${{ steps.version.outputs.version }}
          target_commitish: ${{ github.sha }}
          files: platform/dist/*
          generate_release_notes: true
```

Notes on this rewrite:
- Trigger changed from `push: branches: ["release-*"]` to `push: tags: ["v*"]`.
- Version now comes from the tag (`v0.4.0-alpha.1` → `0.4.0-alpha.1`), NOT from a branch name.
- The `sed` workspace-version patch is removed — release-please already committed the correct version at the tagged commit.
- The test jobs (rust/go/web-ui/openapi) moved to `ci.yml`.
- The release is already created by release-please; `softprops/action-gh-release` with the same `tag_name` adds assets to the existing release.

- [ ] **Step 3: Validate both workflow YAML files**

Run:
```bash
python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in ['.github/workflows/release.yml','.github/workflows/ci.yml']]; print('YAML OK')"
```
Expected: `YAML OK`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml .github/workflows/ci.yml
git commit -m "ci(release): build assets on v* tags; add ci workflow for develop/main"
```

---

### Task 4: Operational verification + docs

**Files:**
- Modify: `docs/superpowers/specs/2026-08-18-release-please-design.md` (mark approved, note changelog is per-package at `platform/CHANGELOG.md` and `platform/web-ui/CHANGELOG.md`)
- No code changes.

- [ ] **Step 1: Local dry-run of release-please (if network/token available)**

Run:
```bash
cd "$(git rev-parse --show-toplevel)"
npx --yes release-please --config-file=release-please-config.develop.json \
  --manifest-file=.release-please-manifest.json \
  --repo-url=THISJOWI/THISCLOUD --target-branch=develop release-pr --dry-run
```
Expected: prints the next proposed version and changelog (no PR created). If GitHub token/network unavailable, note this is skipped — the release-please-action on the first `develop` push is the real verification.

- [ ] **Step 2: Update the design spec status**

In `docs/superpowers/specs/2026-08-18-release-please-design.md`, change `**Status:** Under Review` → `**Status:** Approved` and append under "Flow and Automation":

```markdown
> Changelogs are generated per package (`platform/CHANGELOG.md`,
> `platform/web-ui/CHANGELOG.md`) by the linked-versions group PR, then
> combined into the single `v*` GitHub Release.
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-08-18-release-please-design.md
git commit -m "docs(release): mark release-please design approved"
```

- [ ] **Step 4: Final review gate**

Run `git status` (clean), `git log --oneline -5` shows the four commits. Verify with `git diff origin/develop --stat` that only expected files changed. Confirm the workflow triggers:
- Push to `develop` → `ci.yml` runs tests; `release-please.yml` opens/updates a release PR (or pre-release tag if a release PR was merged).
- Merge release PR on `develop` → tag `vX.Y.Z-alpha.N` → `release.yml` attaches assets, `iso.yml` builds the testing ISO.
- Same flow on `main` with `vX.Y.Z` stable.

---

