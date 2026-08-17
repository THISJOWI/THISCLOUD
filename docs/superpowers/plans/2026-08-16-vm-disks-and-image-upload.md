# VM Disks Page, Image Source Selector, Create VM Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a source-type selector (URL vs file upload) to the Images page, remove inline image register/upload tooling from the Create VM modal, and add a new read-only VM Disks page listing every VM's boot and data disks.

**Architecture:** The go-api gains a daemon-proxied `GET /api/v1/vm-disks` endpoint that flattens each VM from the daemon's `GET /vms` into boot-disk + data-disk rows, enriching boot size from go-api desired-state `disk_gb` by VM id. The web UI gets a `listVmDisks()` helper, a new `/admin/disks` page, a "Disks" sidebar entry, an upload-source toggle on the Images page, and a trimmed Create VM modal. Design doc: `docs/superpowers/specs/2026-08-16-vm-disks-and-image-upload-design.md`.

**Tech Stack:** Go (go-api), Next.js 14 App Router + TypeScript (web-ui), node:test (web-ui tests), net/http + httptest (go-api tests).

## Global Constraints

- Web UI never calls the daemon (`:8080`) directly. Server components use the Go API at `:8081` via `src/lib/api.ts`; client components go through `/api/proxy/[...path]`. Never expose `API_URL` to the client.
- `npm test` runs `node --test tests/api.test.mjs` — no jest/vitest, no framework-specific test code.
- go-api verification: `cd platform/go-api && go test ./...`.
- web-ui verification: `cd platform/web-ui && npm run lint && npm test`.
- Disks page is **read-only** — no create/delete/attach actions.
- Create VM keeps the simple image `<select>` of already-registered images; only the inline register/upload box is removed.
- Existing CSS classes only (`.btn .btn-primary .btn-secondary .form-grid .field-label .form-input .form-select .form-row .field-hint .table-wrap .table-toolbar .table-title .table-actions .id-cell .empty .badge .content .error`). No new CSS.

---

## File Structure

```
platform/go-api/internal/backend/client.go       # add ListVMDisk() — GET daemon /vms
platform/go-api/internal/api/server.go           # add GET /api/v1/vm-disks handler
platform/go-api/internal/api/server_test.go      # add TestListVmDisksProxiesDaemon
platform/web-ui/src/lib/api.ts                   # add VmDisk type + listVmDisks()
platform/web-ui/tests/api.test.mjs               # add vm-disks mock route + contract test
platform/web-ui/src/app/(app)/admin/disks/page.tsx  # NEW read-only disks page
platform/web-ui/src/components/sidebar.tsx       # add "Disks" nav entry
platform/web-ui/src/app/(app)/admin/images/page.tsx # source type selector (URL | File)
platform/web-ui/src/components/create-vm-modal.tsx  # remove inline image tooling
```

---

### Task 1: go-api VM disks endpoint

**Files:**
- Modify: `platform/go-api/internal/backend/client.go`
- Modify: `platform/go-api/internal/api/server.go`
- Test: `platform/go-api/internal/api/server_test.go`

**Interfaces:**
- Produces: `backend.Client.ListVMDisk(ctx context.Context) ([]map[string]any, error)` — returns the daemon's raw VM array (fields: `id`, `name`, `status`, `disk_path`, `disks[]` where each disk has `id`, `path`, `size_gb`).
- Produces: HTTP route `GET /api/v1/vm-disks` returning JSON array of rows `{vm_id, vm_name, disk_id, path, size_gb, kind: "boot"|"data", vm_status}`.

- [ ] **Step 1: Add `ListVMDisk` to the backend client**

In `platform/go-api/internal/backend/client.go`, after the `ListNodes` method (line ~80), add:

```go
// ListVMDisk returns the daemon's live VM list (GET /vms), used to flatten
// boot and data disks for the web UI's read-only disks view.
func (c *Client) ListVMDisk(ctx context.Context) ([]map[string]any, error) {
	return c.list(ctx, "vms")
}
```

- [ ] **Step 2: Add the handler and route**

In `platform/go-api/internal/api/server.go`:
1. Register the route in `Handler()` (after `mux.HandleFunc("GET /api/v1/nodes", s.listNodes)`):

```go
	mux.HandleFunc("GET /api/v1/vm-disks", s.listVmDisks)
```

2. Add the handler after `listNodes`:

```go
// listVmDisks returns a flattened, live view of every VM's disks. Boot disks
// come from the daemon's disk_path; boot size is enriched from the
// orchestrator's desired-state disk_gb matched by VM id (the daemon does not
// persist a boot size). Data disks come from the daemon's disks[] entries.
func (s *Server) listVmDisks(w http.ResponseWriter, r *http.Request) {
	vms, err := s.backend.ListVMDisk(r.Context())
	if err != nil {
		writeError(w, http.StatusBadGateway, err)
		return
	}

	bootGB := map[string]int{}
	if stored, err := s.store.List(model.ResourceVM); err == nil {
		for _, res := range stored {
			if vm, ok := res.(model.VM); ok {
				bootGB[vm.ID()] = vm.DiskGB
			}
		}
	}

	rows := make([]map[string]any, 0)
	for _, vm := range vms {
		id, _ := vm["id"].(string)
		name, _ := vm["name"].(string)
		status, _ := vm["status"].(string)
		path, _ := vm["disk_path"].(string)

		rows = append(rows, map[string]any{
			"vm_id":     id,
			"vm_name":   name,
			"disk_id":   "",
			"path":      path,
			"size_gb":   bootGB[id],
			"kind":      "boot",
			"vm_status": status,
		})

		if disks, ok := vm["disks"].([]any); ok {
			for _, d := range disks {
				disk, _ := d.(map[string]any)
				rows = append(rows, map[string]any{
					"vm_id":     id,
					"vm_name":   name,
					"disk_id":   disk["id"],
					"path":      disk["path"],
					"size_gb":   disk["size_gb"],
					"kind":      "data",
					"vm_status": status,
				})
			}
		}
	}
	writeJSON(w, http.StatusOK, rows)
}
```

- [ ] **Step 3: Write the failing test**

In `platform/go-api/internal/api/server_test.go`, add `"thiscloud/api/internal/model"` to the imports, then append:

```go
func TestListVmDisksProxiesDaemon(t *testing.T) {
	daemon := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/vms" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`[
			{"id":"vm-1","name":"web","disk_path":"/var/lib/thiscloud/vms/web.qcow2","status":"running",
			 "disks":[{"id":"d-1","path":"/data/d1.qcow2","size_gb":50}]},
			{"id":"vm-2","name":"db","disk_path":"/var/lib/thiscloud/vms/db.qcow2","status":"stopped","disks":[]}
		]`))
	}))
	defer daemon.Close()

	path := filepath.Join(t.TempDir(), "test.tfstate")
	store, err := state.NewStore(path)
	if err != nil {
		t.Fatalf("state: %v", err)
	}
	// Seed desired state so boot-size enrichment has something to read.
	if err := store.Put(model.VM{
		TypeName:   "thiscloud_vm",
		ResourceID: "vm-1",
		Name:       "web",
		DiskGB:     20,
	}); err != nil {
		t.Fatalf("seed store: %v", err)
	}
	s := NewServer(store, backend.NewClient(daemon.URL))

	rec := doJSON(t, s, http.MethodGet, "/api/v1/vm-disks", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("vm-disks: want 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var rows []map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &rows); err != nil {
		t.Fatalf("decode rows: %v", err)
	}
	if len(rows) != 3 {
		t.Fatalf("want 3 rows, got %d", len(rows))
	}
	// Row 0: vm-1 boot disk, size enriched from state (20).
	if rows[0]["vm_id"] != "vm-1" || rows[0]["kind"] != "boot" || rows[0]["size_gb"] != float64(20) {
		t.Fatalf("boot row wrong: %v", rows[0])
	}
	// Row 1: vm-1 data disk, size from daemon (50).
	if rows[1]["kind"] != "data" || rows[1]["size_gb"] != float64(50) {
		t.Fatalf("data row wrong: %v", rows[1])
	}
	// Row 2: vm-2 boot disk, no state entry -> size 0.
	if rows[2]["vm_id"] != "vm-2" || rows[2]["kind"] != "boot" || rows[2]["size_gb"] != float64(0) {
		t.Fatalf("vm-2 boot row wrong: %v", rows[2])
	}
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cd platform/go-api && go test ./internal/api/ -run TestListVmDisksProxiesDaemon -v`
Expected: FAIL — `undefined: s.listVmDisks` (or compile error about `ListVMDisk`).

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd platform/go-api && go test ./...`
Expected: PASS — all go-api tests pass.

- [ ] **Step 6: Commit**

```bash
git add platform/go-api/internal/backend/client.go platform/go-api/internal/api/server.go platform/go-api/internal/api/server_test.go
git commit -m "feat(go-api): proxy VM disks from daemon via /api/v1/vm-disks"
```

---

### Task 2: web-ui API layer for VM disks

**Files:**
- Modify: `platform/web-ui/src/lib/api.ts`
- Test: `platform/web-ui/tests/api.test.mjs`

**Interfaces:**
- Consumes: `GET /api/v1/vm-disks` (from Task 1).
- Produces: `type VmDisk` and `listVmDisks(): Promise<VmDisk[]>` consumed by Task 3.

- [ ] **Step 1: Add `VmDisk` type and `listVmDisks()`**

In `platform/web-ui/src/lib/api.ts`, after `ClusterNode`/`listNodes` (or after the image helpers), add:

```ts
export type VmDisk = {
  vm_id: string;
  vm_name: string;
  disk_id?: string;
  path: string;
  size_gb?: number;
  kind: "boot" | "data";
  vm_status?: string;
};

export async function listVmDisks(): Promise<VmDisk[]> {
  const res = await apiFetch("/api/v1/vm-disks");
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] GET /api/v1/vm-disks failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  const data = await res.json();
  return Array.isArray(data) ? data : [];
}
```

- [ ] **Step 2: Add a mock route + contract test**

In `platform/web-ui/tests/api.test.mjs`, in `startMock()` before the final `res.statusCode = 404`, add a route for the disks endpoint:

```js
    if (url.pathname === "/api/v1/vm-disks" && req.method === "GET") {
      res.end(
        JSON.stringify([
          {
            vm_id: "vm-1",
            vm_name: "web",
            disk_id: "",
            path: "/var/lib/thiscloud/vms/web.qcow2",
            size_gb: 20,
            kind: "boot",
            vm_status: "running",
          },
          {
            vm_id: "vm-1",
            vm_name: "web",
            disk_id: "d-1",
            path: "/data/d1.qcow2",
            size_gb: 50,
            kind: "data",
            vm_status: "running",
          },
        ])
      );
      return;
    }
```

Then append this test at the end of the file:

```js
test("list VM disks returns boot and data rows", async () => {
  const { server, base } = await startMock();
  try {
    const res = await fetch(`${base}/api/v1/vm-disks`);
    assert.equal(res.ok, true);
    const body = await res.json();
    assert.equal(body.length, 2);
    assert.equal(body[0].kind, "boot");
    assert.equal(body[0].size_gb, 20);
    assert.equal(body[1].kind, "data");
    assert.equal(body[1].size_gb, 50);
  } finally {
    server.close();
  }
});
```

- [ ] **Step 3: Run the tests**

Run: `cd platform/web-ui && npm test`
Expected: PASS — all tests including the new VM disks test.

- [ ] **Step 4: Run lint**

Run: `cd platform/web-ui && npm run lint`
Expected: PASS — no errors.

- [ ] **Step 5: Commit**

```bash
git add platform/web-ui/src/lib/api.ts platform/web-ui/tests/api.test.mjs
git commit -m "feat(web-ui): add listVmDisks API helper and contract test"
```

---

### Task 3: VM Disks page + sidebar entry

**Files:**
- Create: `platform/web-ui/src/app/(app)/admin/disks/page.tsx`
- Modify: `platform/web-ui/src/components/sidebar.tsx`

**Interfaces:**
- Consumes: `listVmDisks()`, `VmDisk` from Task 2; `ContextHeader`, `StatusBadge` from `@/components/ui`.

- [ ] **Step 1: Create the disks page**

Create `platform/web-ui/src/app/(app)/admin/disks/page.tsx`:

```tsx
"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listVmDisks, VmDisk } from "@/lib/api";
import { ContextHeader, StatusBadge } from "@/components/ui";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function DisksPage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/disks");
  const [disks, setDisks] = useState<VmDisk[]>([]);
  const [error, setError] = useState("");

  async function refresh() {
    setDisks(await listVmDisks().catch(() => []));
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  function formatSize(gb?: number) {
    if (!gb || gb <= 0) return "—";
    return `${gb} GB`;
  }

  if (authorized === null) {
    return (
      <div className="content">
        <div className="loading-page">
          <div className="spinner" />
          Checking authorization...
        </div>
      </div>
    );
  }

  if (authorized === false) {
    return (
      <div className="content">
        <ContextHeader title="Disks" />
        <p className="error">{authError || "Access denied"}</p>
        <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
      </div>
    );
  }

  return (
    <div className="content">
      <ContextHeader
        title="Disks"
        meta="Boot and data disks attached to virtual machines"
      />

      {error && <p className="error">{error}</p>}

      <div className="table-wrap">
        <div className="table-toolbar">
          <span className="table-title">All VM Disks</span>
          <div className="table-actions">
            <span className="text-muted" style={{ fontSize: 12 }}>
              {disks.length} disk{disks.length !== 1 ? "s" : ""}
            </span>
          </div>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>VM</th>
                <th>Disk</th>
                <th>Size</th>
                <th>Kind</th>
                <th>VM Status</th>
              </tr>
            </thead>
            <tbody>
              {disks.length === 0 && (
                <tr>
                  <td colSpan={5} className="empty">
                    No VM disks
                  </td>
                </tr>
              )}
              {disks.map((d, i) => (
                <tr key={`${d.vm_id}-${d.kind}-${d.disk_id ?? "boot"}-${i}`}>
                  <td>
                    <span className="id-cell" style={{ maxWidth: 160 }}>
                      {d.vm_name}
                    </span>
                  </td>
                  <td>
                    <span className="id-cell" style={{ maxWidth: 340 }}>
                      {d.path}
                    </span>
                  </td>
                  <td>{formatSize(d.size_gb)}</td>
                  <td>{d.kind === "boot" ? "boot" : "data"}</td>
                  <td>
                    <StatusBadge status={d.vm_status ?? ""} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Add the sidebar entry**

In `platform/web-ui/src/components/sidebar.tsx`, after the Storage link (line ~114, before the Images link), add:

```tsx
            <Link
              href="/admin/disks"
              className={`tree-node ${isActive("/admin/disks") ? "active" : ""}`}
            >
              <span className="tree-icon">⌗</span>
              Disks
            </Link>
```

- [ ] **Step 3: Run lint**

Run: `cd platform/web-ui && npm run lint`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add platform/web-ui/src/app/"(app)"/admin/disks/page.tsx platform/web-ui/src/components/sidebar.tsx
git commit -m "feat(web-ui): add read-only VM disks page and sidebar entry"
```

---

### Task 4: Images page — source type selector

**Files:**
- Modify: `platform/web-ui/src/app/(app)/admin/images/page.tsx`

**Interfaces:**
- Consumes: `registerImage`, `uploadImage`, `listImages` from `@/lib/api` (already exported).

- [ ] **Step 1: Add source-mode state and file state**

In `platform/web-ui/src/app/(app)/admin/images/page.tsx`:

1. Change the import to include `uploadImage`:

```tsx
import { Image, listImages, registerImage, uploadImage } from "@/lib/api";
```

2. After `const [importing, setImporting] = useState(false);` add:

```tsx
  const [sourceMode, setSourceMode] = useState<"url" | "file">("url");
  const [file, setFile] = useState<File | null>(null);
```

- [ ] **Step 2: Rewrite `onImport` to branch on source mode**

Replace the existing `onImport` function (lines 33-63) with:

```tsx
  async function onImport(e: React.FormEvent) {
    e.preventDefault();
    if (!importForm.name.trim()) {
      setError("Image name is required");
      return;
    }
    if (sourceMode === "url" && !importForm.source.trim()) {
      setError("Source URL is required");
      return;
    }
    if (sourceMode === "file" && !file) {
      setError("Select a local file to upload");
      return;
    }
    setImporting(true);
    setError("");
    try {
      if (sourceMode === "file") {
        const baseName = file!.name.replace(/\.(iso|qcow2|qcow|img|raw)$/i, "");
        const inferredFormat = file!.name.toLowerCase().endsWith(".iso")
          ? "iso"
          : file!.name.toLowerCase().endsWith(".qcow2") ||
              file!.name.toLowerCase().endsWith(".qcow")
            ? "qcow2"
            : "raw";
        const img = await registerImage({
          name: importForm.name.trim() || baseName,
          source: "",
          format: inferredFormat,
          os_family: importForm.os_family,
          version: importForm.version.trim(),
        });
        await uploadImage(img.id!, file!);
      } else {
        await registerImage({
          name: importForm.name.trim(),
          source: importForm.source.trim(),
          format: importForm.format,
          os_family: importForm.os_family,
          version: importForm.version.trim(),
        });
      }
      setShowImport(false);
      setImportForm({
        name: "",
        source: "",
        format: "qcow2",
        os_family: "alma",
        version: "",
      });
      setFile(null);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setImporting(false);
    }
  }
```

- [ ] **Step 3: Add the toggle and conditional source field**

In the register form JSX, replace the `Source URL` field `<div>` (lines 125-134) with a source-mode toggle followed by a conditional field:

```tsx
              <div>
                <label className="field-label">Source</label>
                <div className="form-row" style={{ marginBottom: 0 }}>
                  <button
                    type="button"
                    className={`btn ${sourceMode === "url" ? "btn-primary" : "btn-secondary"}`}
                    onClick={() => setSourceMode("url")}
                  >
                    URL
                  </button>
                  <button
                    type="button"
                    className={`btn ${sourceMode === "file" ? "btn-primary" : "btn-secondary"}`}
                    onClick={() => setSourceMode("file")}
                  >
                    File upload
                  </button>
                </div>
              </div>
              {sourceMode === "url" ? (
                <div>
                  <label className="field-label" htmlFor="img-source">Source URL</label>
                  <input
                    id="img-source"
                    className="form-input"
                    value={importForm.source}
                    onChange={(e) => setImportForm({ ...importForm, source: e.target.value })}
                    placeholder="https://example.com/img.qcow2"
                  />
                </div>
              ) : (
                <div>
                  <label className="field-label" htmlFor="img-file">Local file</label>
                  <input
                    id="img-file"
                    type="file"
                    accept=".iso,.qcow2,.qcow,.img,.raw"
                    className="form-input"
                    onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                  />
                  {file && (
                    <p className="field-hint" style={{ marginTop: 4 }}>
                      {file.name} — {(file.size / 1024 / 1024).toFixed(1)} MB
                    </p>
                  )}
                </div>
              )}
```

- [ ] **Step 4: Run lint**

Run: `cd platform/web-ui && npm run lint`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add platform/web-ui/src/app/"(app)"/admin/images/page.tsx
git commit -m "feat(web-ui): add URL/file source selector to images page"
```

---

### Task 5: Create VM modal — remove inline image tooling

**Files:**
- Modify: `platform/web-ui/src/components/create-vm-modal.tsx`

**Interfaces:**
- Consumes: `listImages`, `Image` from `@/lib/api` (kept); `registerImage`, `uploadImage` no longer used.
- Produces: unchanged `CreateVmModal` props `{ networks, onClose, onCreated }` and same `createResource("thiscloud_vm", ...)` payload including `image`.

- [ ] **Step 1: Trim imports and remove dead state**

In `platform/web-ui/src/components/create-vm-modal.tsx`:

1. Change line 4 from:

```tsx
import { Image, listImages, registerImage, uploadImage } from "@/lib/api";
```

to:

```tsx
import { Image, listImages } from "@/lib/api";
```

2. Remove these state declarations (lines 46-58):

```tsx
  const [importing, setImporting] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [importForm, setImportForm] = useState({
    name: "",
    source: "",
    format: "qcow2",
    os_family: "alma",
    version: "",
  });
  const [uploadFile, setUploadFile] = useState<File | null>(null);
```

3. Remove the `notice` state line: `const [notice, setNotice] = useState("");`

- [ ] **Step 2: Remove `onImport` and `onUpload`**

Delete the `onImport` function (lines 68-99) and the `onUpload` function (lines 101-136) in their entirety.

- [ ] **Step 3: Remove the `notice` banner**

In the modal body, remove the `notice` paragraph block:

```tsx
          {notice && (
            <p className="text-secondary" style={{ fontSize: 12, marginBottom: 12 }}>
              {notice}
            </p>
          )}
```

- [ ] **Step 4: Remove the `import-box` block from the OS/Disk tab**

Replace the entire OS/Disk tab contents (the `{tab === "OS / Disk" && (...)}` block, lines 236-381) with the trimmed version (image select + disk size only, no import box):

```tsx
          {tab === "OS / Disk" && (
            <div className="form-grid">
              <div>
                <label className="field-label" htmlFor="vm-image">Image / ISO</label>
                <select
                  id="vm-image"
                  className="form-select"
                  value={form.image}
                  onChange={(e) => set("image", e.target.value)}
                >
                  <option value="">— select an image —</option>
                  {runningImages.map((img) => (
                    <option key={img.id ?? img.name} value={img.name}>
                      {img.name}
                      {img.version ? ` (${img.version})` : ""} — {img.format}
                      {img.os_family ? ` · ${img.os_family}` : ""}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="field-label" htmlFor="vm-disk">Disk size (GB)</label>
                <input
                  id="vm-disk"
                  type="number"
                  min={1}
                  className="form-input"
                  value={form.disk_gb}
                  onChange={(e) => set("disk_gb", e.target.value)}
                />
              </div>
            </div>
          )}
```

`runningImages` (lines 176-178) stays — it still feeds the simple selector.

- [ ] **Step 5: Verify no dangling references**

Run: `cd platform/web-ui && npx tsc --noEmit`
Expected: PASS — no references to `importForm`, `uploadFile`, `notice`, `onImport`, `onUpload`, `importing`, `uploading` remain.

- [ ] **Step 6: Run lint and tests**

Run: `cd platform/web-ui && npm run lint && npm test`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add platform/web-ui/src/components/create-vm-modal.tsx
git commit -m "refactor(web-ui): drop inline image register/upload from create VM modal"
```

---

## Verification

- [ ] `cd platform/go-api && go test ./...` — PASS
- [ ] `cd platform/web-ui && npm run lint` — PASS
- [ ] `cd platform/web-ui && npm test` — PASS
- [ ] `cd platform/web-ui && npx tsc --noEmit` — PASS