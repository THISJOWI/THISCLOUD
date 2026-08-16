# Design — Image source selector, Create VM cleanup, VM Disks page

Date: 2026-08-16

## Context

The web UI (`platform/web-ui/`) has three gaps:

1. The **Images** page (`admin/images`) only registers images by URL. Local file upload exists in `api.ts` (`registerImage` + `uploadImage`) and is already exercised inside the Create VM modal, but is not reachable from the Images page.
2. The **Create VM** modal (`create-vm-modal.tsx`) bundles inline image tooling (register-by-URL + upload-by-file) into its OS/Disk tab, duplicating what belongs on the Images page.
3. VM **disks** are not visible anywhere in the UI. The daemon's `VmConfig` carries a boot disk (`disk_path`) plus data disks (`disks[]` with `id`, `path`, `size_gb`), but the go-api `VM` model does not expose them and the UI has no disks view.

## Decisions

- Create VM keeps a **simple selector** of already-registered images. The inline register/upload box is removed.
- The VM disks view is a **new read-only admin page** (`/admin/disks`), data sourced **live via a go-api proxy** (like `listImages`/`listNodes`), not from go-api's desired-state store.
- Boot disk size is **enriched from go-api state** (`disk_gb` matched by VM id), because the daemon persists only `disk_path` for the boot disk and no size. Data disk sizes come straight from the daemon's `disks[].size_gb`.

## Design

### 1. Images page — source type selector (`admin/images/page.tsx`)

- Add a segmented toggle **Source: URL | File upload** at the top of the register form.
- **URL mode** (default): existing `Source URL` input. Submit calls `registerImage({ name, source, format, os_family, version })` unchanged.
- **File mode**: a file input accepting `.iso,.qcow2,.qcow,.img,.raw`. Submit calls `registerImage({ name, source: "", format, os_family, version })` then `uploadImage(img.id, file)` — the exact two-step flow already implemented in `create-vm-modal.tsx`. When the name field is blank, auto-fill it from the filename (base name minus extension); infer `format` from the extension (`.iso` → iso, `.qcow2`/`.qcow` → qcow2, else raw).
- `Name`, `Format`, `OS family`, `Version` fields are shared and shown in both modes.
- The upload path (`/api/proxy/api/v1/images/{id}/upload`, octet-stream passthrough) is already proxied by the go-api; no backend change needed for uploads.

### 2. Create VM — drop inline image tooling (`create-vm-modal.tsx`)

- Delete the entire `import-box` block from the OS/Disk tab (register-by-URL form + upload-by-file control + "or upload a local file" divider).
- Keep the image `<select>` (populated from `listImages()`, filtered to bootable formats iso/qcow2/raw) and the disk size field.
- Remove dead state/logic: `importForm`, `uploadFile`, `importing`, `uploading`, `notice`, `onImport`, `onUpload`, and the unused `registerImage`/`uploadImage` imports.

### 3. VM Disks page (`/admin/disks`)

#### go-api (`platform/go-api/`)

- `internal/backend/client.go`: add `ListVMDisk(ctx) ([]map[string]any, error)` — GET daemon `/vms` (reuses the existing `list` helper).
- `internal/api/server.go`: register `GET /api/v1/vm-disks` → handler `listVmDisks`.
  - Fetch VMs from the daemon via `ListVMDisk`.
  - For each VM: emit one boot-disk row `{ vm_id, vm_name, disk_id: "", path: vm.disk_path, size_gb: <state disk_gb>, kind: "boot", vm_status }`, where `<state disk_gb>` comes from the go-api store matched by VM `id` (store `List("thiscloud_vm")`), falling back to 0 when absent.
  - For each `disks[]` entry: emit a data-disk row `{ vm_id, vm_name, disk_id, path, size_gb, kind: "data", vm_status }`.
  - Empty result serializes as `[]` (same guard as other list handlers).
- Add go-api handler tests in `internal/api/server_test.go` using the mock daemon (existing test scaffolding).

#### web-ui (`platform/web-ui/`)

- `src/lib/api.ts`: add
  - `type VmDisk = { vm_id: string; vm_name: string; disk_id?: string; path: string; size_gb?: number; kind: "boot" | "data"; vm_status?: string }`
  - `listVmDisks(): Promise<VmDisk[]>` → GET `/api/v1/vm-disks`.
- New page `src/app/(app)/admin/disks/page.tsx`: client component, `useAdminAuth("/?redirect=/admin/disks")`, same shell as `storage/page.tsx`. Table columns: **VM**, **Disk** (path, rendered in an `id-cell`), **Size** (boot enriched / data from daemon, `formatBytes` helper), **Kind** (boot/data), **VM status** (`StatusBadge`). Empty state: "No VM disks".
- `src/components/sidebar.tsx`: add a "Disks" tree-node link under Storage (`/admin/disks`, icon `⌗`).

## Files touched

| File | Change |
|---|---|
| `platform/web-ui/src/app/(app)/admin/images/page.tsx` | Source type selector, file upload path |
| `platform/web-ui/src/components/create-vm-modal.tsx` | Remove inline image tooling, clean dead state |
| `platform/web-ui/src/lib/api.ts` | `VmDisk` type + `listVmDisks()` |
| `platform/web-ui/src/app/(app)/admin/disks/page.tsx` | New read-only disks page |
| `platform/web-ui/src/components/sidebar.tsx` | "Disks" nav entry |
| `platform/go-api/internal/backend/client.go` | `ListVMDisk` |
| `platform/go-api/internal/api/server.go` | `GET /api/v1/vm-disks` handler |
| `platform/go-api/internal/api/server_test.go` | Handler tests |

## Error handling

- go-api disks handler: daemon unreachable → `502 Bad Gateway` with the backend error (same as `listImages`/`listNodes`).
- web-ui disks page: fetch failure → `error` banner + empty table (same pattern as other admin pages).
- Images file upload: empty file selection → inline error "Select a local file to upload"; upload failure surfaces the API error text.

## Testing

- go-api: `go test ./...`; new `TestListVmDisks`-style case against mock daemon (mirrors existing `TestListImages`-style tests).
- web-ui: `npm run lint`; `npm test` (existing `tests/api.test.mjs` — no framework-specific test code added unless the existing test file's shape naturally covers it).
- Manual: register image via URL and via file on Images page; confirm Create VM shows only the simple image selector; confirm Disks page lists boot + data disks with sizes.
