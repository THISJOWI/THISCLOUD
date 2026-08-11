# THISCLOUD

THISCLOUD Hypervisor OS — a self-hosted cloud platform for managing VMs, networks, storage, and apps.

## Project Structure

```
platform/
  thiscloud-cli/    CLI tool (thiscloud)
  thiscloudd/       Rust daemon (thiscloudd)
  go-api/           Go orchestrator API
  web-ui/           Next.js dashboard
  iso/              ISO build tooling
```

## CLI Commands (`thiscloud`)

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

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `THISCLOUD_API_URL` | `http://127.0.0.1:8080` | Daemon API URL (CLI) |
| `NEXT_PUBLIC_API_URL` | `http://127.0.0.1:8081` | Go API URL (web UI) |
| `THISCLOUD_STATE_FILE` | `./thiscloud.tfstate` | State file path (Go API) |
| `THISCLOUD_API_BIND` | `127.0.0.1:8081` | Go API bind address |

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
