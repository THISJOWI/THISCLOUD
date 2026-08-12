<p align="center">
  <h1 align="center">THISCLOUD</h1>
</p>

<p align="center">
  <strong>Hypervisor OS — self-hosted cloud platform for VMs, networks, storage, and apps</strong>
</p>

<p align="center">
  Manage your own private cloud from a single CLI and web dashboard. Initialize a cluster, provision virtual machines, define networks, configure replicated storage, and install apps from the marketplace — all self-hosted.
</p>

<p align="center">
  <a href="https://github.com/THISJOWI/THISCLOUD/releases">
    <img src="https://img.shields.io/github/v/release/THISJOWI/THISCLOUD?style=flat-square" alt="Release" />
  </a>
  <a href="https://github.com/THISJOWI/THISCLOUD/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License" />
  </a>
  <a href="https://github.com/THISJOWI/THISCLOUD/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/THISJOWI/THISCLOUD/ci.yml?style=flat-square&label=ci" alt="CI" />
  </a>
  <a href="https://github.com/THISJOWI/THISCLOUD/discussions">
    <img src="https://img.shields.io/github/discussions/THISJOWI/THISCLOUD?style=flat-square" alt="Discussions" />
  </a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quickstart">Quickstart</a> •
  <a href="#cli-commands">CLI Commands</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#development">Development</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## Features

- **Cluster management** — initialize master/worker nodes and join them into one cluster
- **Virtual machines** — create, start, stop, and delete VMs with CPU/memory control
- **Networking** — define networks with CIDR and gateway
- **Replicated storage** — storage pools backed by Linstor, DRBD, or local disk with configurable replication
- **App marketplace** — install and manage apps from sources
- **Web dashboard** — Next.js UI over the Go orchestrator API
- **Installable ISO** — build a bootable THISCLOUD image (AlmaLinux 9 x86_64)

---

## Quickstart

```sh
# Clone the repository
git clone https://github.com/THISJOWI/THISCLOUD.git
cd THISCLOUD

# 1. Initialize the first node (master)
thiscloud init --ip <IP> --role master

# 2. Add workers
thiscloud join --master <MASTER_IP> --ip <IP>

# 3. Provision a VM
thiscloud vm create --name web-01 --cpus 2 --memory 4096
thiscloud vm start web-01

# Check cluster status
thiscloud status
```

---

## CLI Commands

### Cluster Management

```sh
thiscloud init --ip <IP> --role master|worker   # Initialize THISCLOUD on this node
thiscloud status                                # Show cluster status
thiscloud join --master <MASTER_IP> --ip <IP>   # Join an existing cluster
```

### Virtual Machines

```sh
thiscloud vm list                                # List all VMs
thiscloud vm create --name <NAME> --cpus <N> --memory <MB>  # Create a VM
thiscloud vm start <VM>                          # Start a VM
thiscloud vm stop <VM>                           # Stop a VM
thiscloud vm delete <VM>                         # Delete a VM
```

### Networks

```sh
thiscloud network list                           # List all networks
thiscloud network create --name <NAME> --cidr <CIDR> --gateway <IP>  # Create a network
thiscloud network delete <ID>                    # Delete a network
```

### Storage

```sh
thiscloud storage list                           # List all storage pools
thiscloud storage create --name <NAME> --pool-type linstor|drbd|local --replication <N>  # Create a pool
thiscloud storage delete <NAME>                  # Delete a storage pool
```

### Marketplace

```sh
thiscloud marketplace list                                    # List marketplace apps
thiscloud marketplace install --name <NAME> --source <SRC>    # Install an app
thiscloud marketplace uninstall <ID>                          # Uninstall an app
```

---

## Architecture

```
platform/
├── thiscloud-cli/    # CLI tool (thiscloud)
├── thiscloudd/       # Rust daemon (thiscloudd)
├── go-api/           # Go orchestrator API
├── web-ui/           # Next.js dashboard
└── iso/              # ISO build tooling
```

```
┌──────────────────────────────────────────────────┐
│                    Web UI (Next.js)              │
│                       │                          │
│                       ▼                          │
│                  Go orchestrator API             │
│                       │                          │
│                       ▼                          │
│              thiscloudd (Rust daemon)            │
│         ┌──────────┬──────────┬───────────┐      │
│         │   VMs    │ Networks │  Storage  │      │
│         │          │          │ Linstor / │      │
│         │          │          │ DRBD/local│      │
│         └──────────┴──────────┴───────────┘      │
└──────────────────────────────────────────────────┘
```

### Tech Stack

| Component   | Technology            |
|-------------|-----------------------|
| Daemon      | Rust (thiscloudd)     |
| CLI         | Rust (thiscloud-cli)  |
| API         | Go orchestrator       |
| Dashboard   | Next.js (TypeScript)  |
| Storage     | Linstor, DRBD, local  |
| Installer   | AlmaLinux 9 ISO       |

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | Daemon API URL (CLI) |
| `NEXT_PUBLIC_API_URL` | `http://127.0.0.1:8081` | Go API URL (web UI) |
| `THISCLOUD_STATE_FILE` | `./thiscloud.tfstate` | State file path (Go API) |
| `THISCLOUD_API_BIND` | `127.0.0.1:8081` | Go API bind address |

---

## Development

### Rust (daemon + CLI)

```sh
cd platform
cargo build                              # Build all crates
cargo build --release                    # Release build
cargo test                               # Run all tests
```

### Go API

```sh
cd platform/go-api
go build ./cmd/api-server                # Build API server
```

### Web UI

```sh
cd platform/web-ui
npm install
npm run dev                              # http://localhost:3000
npm run build                            # Production build
npm test                                 # Run tests
npm run lint                             # Lint
```

### ISO Build

See `platform/iso/README.md` for full details. Must be built on AlmaLinux 9 x86_64.

```sh
cd platform/iso
scripts/build-iso.sh                     # Full pipeline: cross-compile → RPM → ISO
```

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](./CONTRIBUTING.md) for setup, guidelines, and how to report bugs.

## Security

Found a vulnerability? Report it privately via [Security Advisories](https://github.com/THISJOWI/THISCLOUD/security/advisories) — see [SECURITY.md](./SECURITY.md).

## License

THISCLOUD is released under the [MIT License](./LICENSE).

---

<p align="center">
  <sub>Built for the self-hosting community.</sub>
</p>
