# THISCLOUD Web UI

Next.js dashboard for THISCLOUD. Two views:

- **Portal** (`/`) — client/cloud view: read-only stats and VM list.
- **Admin** (`/admin`) — private panel: create and delete VMs, networks and storage pools.
- **Console** (`/console`) — xterm.js based cluster console.

Talks to the Go orchestrator API (`platform/go-api`, default `http://127.0.0.1:8081`). Set `NEXT_PUBLIC_API_URL` to override.

## Run

```sh
npm install
npm run dev        # http://localhost:3000
```

## Test

```sh
npm test           # node --test tests/api.test.mjs
```

## Build

```sh
npm run build
```
