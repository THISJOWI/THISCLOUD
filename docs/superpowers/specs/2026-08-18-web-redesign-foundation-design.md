# Design — Web redesign foundation: daemon proxy, VM id reconciliation, console WS

Date: 2026-08-18

## Context

The web UI (`platform/web-ui/`) is being rebuilt as a Proxmox + OpenStack-style dashboard. Before any feature work, three blockers make the deployed platform non-functional and must be fixed first (verified live against `192.168.1.18`):

1. **Stale orchestrator build.** The deployed go-api returns `404 page not found` for `/api/v1/nodes`, `/api/v1/vm-disks`, `/api/v1/images`, and `/api/v1/vms/{id}/start|stop`, even though the repo already ships those routes. Cause: the running binary predates the routes.
2. **Corrupt VM state.** The VM "test" in state has `id: ""` and `status: ""` — created by a pre-`AssignID` go-api. The console `<select>` renders options with `value=""` (unselectable), and lifecycle calls against the empty id fail.
3. **Console websocket hardcoded to `ws://127.0.0.1:8080` in dev** (`console/page.tsx`), which breaks the console on any remote box.

Additionally, the whole redesign depends on the web UI reaching the **full daemon API surface** (node lifecycle, VM snapshot/clone/resize/migrate/hotplug, image delete/template, routers, DHCP, floating IPs, quotas, audit, backups, S3, marketplace, metrics). Writing a typed go-api handler per endpoint is not viable — a generic passthrough is needed.

## Decisions

- Add a **generic daemon proxy** in go-api: catch-all routes `GET/POST/PUT/PATCH/DELETE /api/v1/{rest...}` that forward method, path, query and body to the daemon and stream status/body back. Existing explicit routes (resources CRUD, images, nodes, vm-disks, start/stop) keep winning because Go's `ServeMux` (1.22+) prefers the most specific pattern. The web UI keeps calling go-api via `/api/proxy/...` unchanged.
- Reconcile VM ids: when orchestrator state diverges from the daemon (legacy entries backfilled with a placeholder uuid on load), `listAll` **adopts the daemon's real id** matched by VM name and persists it (`Store.Replace`), so start/stop/delete/console address the same VM the daemon knows.
- Fix the console websocket: drop the `IS_DEV` hardcode. Derive the WS host from `window.location.host`, with `NEXT_PUBLIC_WS_URL` as an explicit override (e.g. `ws://192.168.1.18:8080`). nginx on the box must WS-proxy `/api/v1/vms/{id}/console/ws` to the daemon `:8080`.

## Design

### 1. go-api — generic daemon proxy

`internal/backend/client.go`:

- `Proxy(ctx, method, rest, query url.Values, contentType string, body io.Reader) (*http.Response, error)` — builds `c.baseURL + "/" + rest`, sets Content-Type (JSON default), forwards query, and returns the raw daemon response without buffering the body.

`internal/api/server.go`:

- Register five catch-all patterns **last** in `Handler()` (specificity makes them lose to every explicit route):
  `GET/POST/PUT/PATCH/DELETE /api/v1/{rest...}` → `s.proxyDaemon`.
- `proxyDaemon` reads the request body, calls `backend.Proxy`, copies the daemon's status code + Content-Type, and streams the body through. Daemon errors (4xx/5xx) pass through verbatim so the web UI sees the daemon's message.

### 2. go-api — VM id reconciliation

`internal/state/store.go`:

- `Replace(oldID string, r model.Resource) error` — swap the entry carrying `oldID` for `r`, persist. Returns `ErrNotFound` if absent. (Plain `Put` would append a duplicate, since the healed id differs from the old one.)

`internal/api/server.go` `listAll` (VM enrichment block):

- Look up each stored VM in the daemon status map by id first, then by name.
- If a daemon match exists with a **different** non-empty id, adopt it: set `vm.ResourceID = did`, persist via `store.Replace(oldID, vm)`, and surface the healed id in the response. Status/node enrichment stays in-memory as today.

This heals clusters upgraded from pre-id go-api builds in place, without manual tfstate surgery.

### 3. web-ui — console websocket host

`src/app/(app)/console/page.tsx`:

- Remove `IS_DEV` and the `ws://127.0.0.1:8080` default.
- `WS_BASE = process.env.NEXT_PUBLIC_WS_URL ?? ""`; component already falls back to `\`${WS_PROTO}://${window.location.host}\``.

### 4. Deployment (ops, not code)

- Rebuild and restart go-api on the box (stale build is the live blocker).
- Rebuild web-ui with `NEXT_PUBLIC_WS_URL=ws://<box-ip>:8080` (or add an nginx WS proxy location for `/api/v1/vms/{id}/console/ws` → daemon `:8080`).
- Restart daemon if needed so the live VM list matches the reconciled state.

## Files touched

| File | Change |
|---|---|
| `platform/go-api/internal/backend/client.go` | Add `Proxy` passthrough method |
| `platform/go-api/internal/api/server.go` | Catch-all proxy routes + `proxyDaemon`; id adoption in `listAll` |
| `platform/go-api/internal/api/server_test.go` | Proxy get/post/error-passthrough tests, specific-route-wins test, daemon-id adoption test |
| `platform/go-api/internal/state/store.go` | Add `Replace` |
| `platform/go-api/internal/state/store_test.go` | `Replace` swap + not-found tests |
| `platform/web-ui/src/app/(app)/console/page.tsx` | WS host derivation, drop hardcode |

## Verification

- `go test ./...` in `platform/go-api/` (proxy forwards method/path/query/body; explicit routes beat catch-all; daemon id adopted and persisted).
- `npm test` and `npm run lint` in `platform/web-ui/`.
- Live: after redeploy, `/api/proxy/api/v1/nodes`, `/api/proxy/api/v1/images`, `/api/proxy/api/v1/vms/{id}/start` return real data; console `<select>` shows a real id and the terminal opens.
