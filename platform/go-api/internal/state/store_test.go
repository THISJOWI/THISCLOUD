package state

import (
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