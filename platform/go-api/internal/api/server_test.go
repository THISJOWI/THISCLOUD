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