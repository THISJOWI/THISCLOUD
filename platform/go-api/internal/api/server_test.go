package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"

	"thiscloud/api/internal/backend"
	"thiscloud/api/internal/model"
	"thiscloud/api/internal/state"
)

func newTestServer(t *testing.T) *Server {
	t.Helper()
	path := filepath.Join(t.TempDir(), "test.tfstate")
	store, err := state.NewStore(path)
	if err != nil {
		t.Fatalf("state: %v", err)
	}
	// Point the client at a closed port so apply() logs a warning instead of
	// failing — the orchestrator stays the source of truth for state.
	client := backend.NewClient("http://127.0.0.1:1")
	return NewServer(store, client)
}

func doJSON(t *testing.T, s *Server, method, path, body string) *httptest.ResponseRecorder {
	t.Helper()
	req := httptest.NewRequest(method, path, bytes.NewBufferString(body))
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	rec := httptest.NewRecorder()
	s.Handler().ServeHTTP(rec, req)
	return rec
}

func TestCreateAndListResources(t *testing.T) {
	s := newTestServer(t)

	create := `{
		"type": "thiscloud_vm",
		"id": "vm-1",
		"name": "web",
		"vcpus": 2,
		"memory_mb": 2048,
		"disk_gb": 20,
		"image": "almalinux-9",
		"status": "running"
	}`
	rec := doJSON(t, s, http.MethodPost, "/api/v1/resources", create)
	if rec.Code != http.StatusCreated {
		t.Fatalf("create: want 201, got %d: %s", rec.Code, rec.Body.String())
	}

	rec = doJSON(t, s, http.MethodGet, "/api/v1/resources", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("list: want 200, got %d", rec.Code)
	}
	var list []json.RawMessage
	if err := json.Unmarshal(rec.Body.Bytes(), &list); err != nil {
		t.Fatalf("decode list: %v", err)
	}
	if len(list) != 1 {
		t.Fatalf("want 1 resource, got %d", len(list))
	}
}

func TestCreateByTypeAndGet(t *testing.T) {
	s := newTestServer(t)

	body := `{
		"id": "net-1",
		"name": "overlay",
		"cidr": "10.0.0.0/24",
		"gateway": "10.0.0.1"
	}`
	rec := doJSON(t, s, http.MethodPost, "/api/v1/resources/thiscloud_network", body)
	if rec.Code != http.StatusCreated {
		t.Fatalf("create: want 201, got %d: %s", rec.Code, rec.Body.String())
	}

	rec = doJSON(t, s, http.MethodGet, "/api/v1/resources/thiscloud_network/net-1", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("get: want 200, got %d", rec.Code)
	}
	if !strings.Contains(rec.Body.String(), `"overlay"`) {
		t.Fatalf("want network name in body: %s", rec.Body.String())
	}
}

func TestCreateWithoutIDAssignsOne(t *testing.T) {
	s := newTestServer(t)

	body := `{
		"type": "thiscloud_vm",
		"name": "no-id-vm",
		"vcpus": 1,
		"memory_mb": 1024
	}`
	rec := doJSON(t, s, http.MethodPost, "/api/v1/resources", body)
	if rec.Code != http.StatusCreated {
		t.Fatalf("create: want 201, got %d: %s", rec.Code, rec.Body.String())
	}

	var created map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &created); err != nil {
		t.Fatalf("decode created: %v", err)
	}
	id, _ := created["id"].(string)
	if id == "" {
		t.Fatalf("create without id: want generated id, got empty")
	}

	rec = doJSON(t, s, http.MethodDelete, "/api/v1/resources/thiscloud_vm/"+id, "")
	if rec.Code != http.StatusOK {
		t.Fatalf("delete: want 200, got %d: %s", rec.Code, rec.Body.String())
	}
}

func TestGetMissingReturns404(t *testing.T) {
	s := newTestServer(t)
	rec := doJSON(t, s, http.MethodGet, "/api/v1/resources/thiscloud_vm/nope", "")
	if rec.Code != http.StatusNotFound {
		t.Fatalf("want 404, got %d", rec.Code)
	}
}

func TestDeleteResource(t *testing.T) {
	s := newTestServer(t)

	create := `{
		"type": "thiscloud_storage_pool",
		"id": "pool-1",
		"name": "data",
		"pool_type": "linstor",
		"replication": 2
	}`
	if rec := doJSON(t, s, http.MethodPost, "/api/v1/resources", create); rec.Code != http.StatusCreated {
		t.Fatalf("create: %d", rec.Code)
	}

	rec := doJSON(t, s, http.MethodDelete, "/api/v1/resources/thiscloud_storage_pool/pool-1", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("delete: want 200, got %d: %s", rec.Code, rec.Body.String())
	}

	rec = doJSON(t, s, http.MethodGet, "/api/v1/resources/thiscloud_storage_pool/pool-1", "")
	if rec.Code != http.StatusNotFound {
		t.Fatalf("want 404 after delete, got %d", rec.Code)
	}
}

func TestDeleteWithoutIDReturns400(t *testing.T) {
	s := newTestServer(t)
	// A delete that doesn't address a concrete resource id used to surface as
	// Go's bare 405 (only GET/POST registered for /resources/{type}); it should
	// now return a clear 400 so clients can diagnose the missing id.
	rec := doJSON(t, s, http.MethodDelete, "/api/v1/resources/thiscloud_vm", "")
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "id") {
		t.Fatalf("want error mentioning missing id, got: %s", rec.Body.String())
	}
}

func TestHealth(t *testing.T) {
	s := newTestServer(t)
	rec := doJSON(t, s, http.MethodGet, "/healthz", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("health: want 200, got %d", rec.Code)
	}
}

func TestCreateUnknownTypeRejected(t *testing.T) {
	s := newTestServer(t)
	rec := doJSON(t, s, http.MethodPost, "/api/v1/resources", `{"type":"nope","id":"x"}`)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", rec.Code)
	}
}
func TestListNodesProxiesDaemon(t *testing.T) {
	daemon := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/nodes" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`[
			{"id":"master-1","name":"Host-01","role":"master","state":"online","cpus_total":16,"cpus_used":4,"memory_total_mb":32768,"memory_used_mb":8192,"vms":3},
			{"id":"worker-1","name":"Worker-01","role":"worker","state":"online","cpus_total":8,"cpus_used":2,"memory_total_mb":16384,"memory_used_mb":4096,"vms":1}
		]`))
	}))
	defer daemon.Close()

	path := filepath.Join(t.TempDir(), "test.tfstate")
	store, err := state.NewStore(path)
	if err != nil {
		t.Fatalf("state: %v", err)
	}
	s := NewServer(store, backend.NewClient(daemon.URL))

	rec := doJSON(t, s, http.MethodGet, "/api/v1/nodes", "")
	if rec.Code != http.StatusOK {
		t.Fatalf("nodes: want 200, got %d: %s", rec.Code, rec.Body.String())
	}
	var nodes []map[string]any
	if err := json.Unmarshal(rec.Body.Bytes(), &nodes); err != nil {
		t.Fatalf("decode nodes: %v", err)
	}
	if len(nodes) != 2 {
		t.Fatalf("want 2 nodes, got %d", len(nodes))
	}
	if nodes[0]["role"] != "master" {
		t.Fatalf("want master first, got %v", nodes[0]["role"])
	}
}

// A daemon that explicitly rejects a create (HTTP 5xx) must not leave a
// phantom resource in orchestrator state — the request fails loudly and the
// resource stays absent from the store.
func TestCreateRejectedByDaemonDoesNotPersist(t *testing.T) {
	daemon := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/api/v1/vms" {
			t.Fatalf("unexpected request: %s %s", r.Method, r.URL.Path)
		}
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte(`{"error":"node worker-1 is not online (state=Offline)"}`))
	}))
	defer daemon.Close()

	path := filepath.Join(t.TempDir(), "test.tfstate")
	store, err := state.NewStore(path)
	if err != nil {
		t.Fatalf("state: %v", err)
	}
	s := NewServer(store, backend.NewClient(daemon.URL))

	body := `{
		"type": "thiscloud_vm",
		"id": "vm-offline-node",
		"name": "phantom",
		"vcpus": 1,
		"memory_mb": 1024,
		"node": "worker-1"
	}`
	rec := doJSON(t, s, http.MethodPost, "/api/v1/resources", body)
	if rec.Code != http.StatusBadGateway {
		t.Fatalf("create: want 502, got %d: %s", rec.Code, rec.Body.String())
	}
	if !strings.Contains(rec.Body.String(), "not online") {
		t.Fatalf("want daemon error surfaced, got: %s", rec.Body.String())
	}

	// The rejected resource must not be stored.
	rec = doJSON(t, s, http.MethodGet, "/api/v1/resources/thiscloud_vm/vm-offline-node", "")
	if rec.Code != http.StatusNotFound {
		t.Fatalf("want 404 (no phantom), got %d", rec.Code)
	}
}

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
