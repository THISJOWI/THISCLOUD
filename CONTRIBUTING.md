# Contributing to THISCLOUD

Thanks for your interest in THISCLOUD! Contributions of all kinds are welcome — code, docs, bug reports, and ideas.

## Getting Started

1. Fork the repository.
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Make your changes.
4. Run the test suites (below).
5. Push and open a pull request against `main`.

## Development Setup

### Rust (daemon + CLI)

```sh
cd platform
cargo test --workspace
cargo clippy --all-targets -- -D warnings
```

### Go API

```sh
cd platform/go-api
go test ./...
```

### Web UI

```sh
cd platform/web-ui
npm install
npm test
npm run lint
```

## ISO Build

See `platform/iso/README.md`. The ISO must be built on AlmaLinux 9 x86_64.

## Pull Request Guidelines

- Keep changes focused. One logical change per PR.
- Add tests for new behavior.
- Keep commits conventional (e.g. `feat:`, `fix:`, `docs:`).
- Ensure CI passes — Rust clippy runs with `-D warnings`.

## Reporting Bugs

Open an issue with the bug template and include:

- Steps to reproduce
- Expected vs actual behavior
- Environment (OS, Rust/Go/Node versions, deployment mode)

## Code of Conduct

Be respectful and constructive. THISCLOUD is built for the self-hosting and homelab community — keep it that way.
