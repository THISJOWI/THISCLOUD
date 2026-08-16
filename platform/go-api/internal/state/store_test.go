package state

import (
	"os"
	"path/filepath"
	"testing"

	"thiscloud/api/internal/model"
)

func tempPath(t *testing.T) string {
	t.Helper()
	return filepath.Join(t.TempDir(), "state", "test.tfstate")
}

func TestStorePutGetListDelete(t *testing.T) {
	store, err := NewStore(tempPath(t))
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}

	vm := model.VM{
		TypeName: string(model.ResourceVM),
		ResourceID: "vm-1",
		Name:     "web",
		VCPUs:    2,
		MemoryMB: 2048,
		DiskGB:   20,
		Image:    "almalinux-9",
	}
	if err := store.Put(vm); err != nil {
		t.Fatalf("Put: %v", err)
	}

	got, err := store.Get("vm-1")
	if err != nil {
		t.Fatalf("Get: %v", err)
	}
	if got.ID() != "vm-1" || got.Type() != model.ResourceVM {
		t.Fatalf("unexpected: %#v", got)
	}

	all, err := store.List("")
	if err != nil {
		t.Fatalf("List: %v", err)
	}
	if len(all) != 1 {
		t.Fatalf("want 1 resource, got %d", len(all))
	}

	if err := store.Delete("vm-1"); err != nil {
		t.Fatalf("Delete: %v", err)
	}
	if _, err := store.Get("vm-1"); err != ErrNotFound {
		t.Fatalf("want ErrNotFound, got %v", err)
	}
}

func TestStorePersistsAcrossReopen(t *testing.T) {
	path := tempPath(t)

	store, err := NewStore(path)
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}
	pool := model.StoragePool{
		TypeName: string(model.ResourceStorage),
		ResourceID: "pool-1",
		Name:        "data",
		PoolType:    "linstor",
		Replication: 2,
	}
	if err := store.Put(pool); err != nil {
		t.Fatalf("Put: %v", err)
	}

	reopened, err := NewStore(path)
	if err != nil {
		t.Fatalf("reopen: %v", err)
	}
	count, err := reopened.Count()
	if err != nil {
		t.Fatalf("Count: %v", err)
	}
	if count != 1 {
		t.Fatalf("want 1 resource after reopen, got %d", count)
	}

	got, err := reopened.List(model.ResourceStorage)
	if err != nil {
		t.Fatalf("List: %v", err)
	}
	if got[0].ID() != "pool-1" {
		t.Fatalf("want pool-1, got %q", got[0].ID())
	}
}

func TestStoreFilterByType(t *testing.T) {
	store, err := NewStore(tempPath(t))
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}
	store.Put(model.VM{TypeName: string(model.ResourceVM), ResourceID: "vm-1"})
	store.Put(model.Network{TypeName: string(model.ResourceNetwork), ResourceID: "net-1"})

	onlyVM, err := store.List(model.ResourceVM)
	if err != nil {
		t.Fatalf("List: %v", err)
	}
	if len(onlyVM) != 1 || onlyVM[0].ID() != "vm-1" {
		t.Fatalf("want only vm-1, got %#v", onlyVM)
	}
}

func TestGetMissingReturnsNotFound(t *testing.T) {
	store, err := NewStore(tempPath(t))
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}
	if _, err := store.Get("nope"); err != ErrNotFound {
		t.Fatalf("want ErrNotFound, got %v", err)
	}
}

// TestLoadAssignsIDToLegacyResources guards against the regression where VMs
// persisted before id assignment had an empty ResourceID — those render with a
// "—" instead of a delete button in the web UI. On load, empty ids must be
// backfilled so legacy entries become deletable.
func TestLoadAssignsIDToLegacyResources(t *testing.T) {
	path := tempPath(t)

	legacy := `{"version":4,"resources":[` +
		`{"type":"thiscloud_vm","id":"","name":"old-vm","vcpus":1,"memory_mb":512,"disk_gb":10,"image":"x","networks":[],"status":""}]}`
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	if err := os.WriteFile(path, []byte(legacy), 0o644); err != nil {
		t.Fatalf("write: %v", err)
	}

	store, err := NewStore(path)
	if err != nil {
		t.Fatalf("NewStore: %v", err)
	}
	vm, err := store.Get("")
	if err == nil {
		t.Fatalf("want ErrNotFound for empty id, got %#v", vm)
	}
	if err != ErrNotFound {
		t.Fatalf("want ErrNotFound, got %v", err)
	}

	list, err := store.List(model.ResourceVM)
	if err != nil {
		t.Fatalf("List: %v", err)
	}
	if len(list) != 1 {
		t.Fatalf("want 1 VM, got %d", len(list))
	}
	if list[0].ID() == "" {
		t.Fatalf("legacy VM still has empty id after load")
	}
	// The backfilled id must be persisted so a restart keeps it stable.
	reopened, err := NewStore(path)
	if err != nil {
		t.Fatalf("reopen: %v", err)
	}
	rlist, err := reopened.List(model.ResourceVM)
	if err != nil {
		t.Fatalf("List reopen: %v", err)
	}
	if len(rlist) != 1 || rlist[0].ID() != list[0].ID() {
		t.Fatalf("backfilled id not stable across reopen: %#v vs %#v", rlist, list)
	}
}