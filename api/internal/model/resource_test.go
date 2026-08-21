package model

import (
	"encoding/json"
	"testing"
)

// The daemon's Rust structs reject empty/blank enums and null sequences
// (networks, dns, devices). Attributes() must emit the daemon's field names
// (cpus, not vcpus) and never serialize null for slices — otherwise apply()
// fails with 422 and the resource never materializes in the daemon.
func TestVMAttributesDaemonPayload(t *testing.T) {
	vm := VM{
		TypeName: "thiscloud_vm", ResourceID: "id-1",
		Name: "web1", VCPUs: 2, MemoryMB: 2048, DiskGB: 20,
		Image: "alma9", Node: "master",
	}
	data, err := json.Marshal(vm.Attributes())
	if err != nil {
		t.Fatal(err)
	}
	raw := string(data)
	if contains(raw, "vcpus") {
		t.Errorf("daemon payload must use cpus, got vcpus: %s", raw)
	}
	if contains(raw, "disk_gb") {
		t.Errorf("daemon payload must not carry UI-only disk_gb: %s", raw)
	}
	if contains(raw, "networks:null") {
		t.Errorf("networks must serialize as [] not null: %s", raw)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatal(err)
	}
	if m["cpus"] != float64(2) {
		t.Errorf("cpus = %v, want 2", m["cpus"])
	}
	if m["memory_mb"] != float64(2048) {
		t.Errorf("memory_mb = %v, want 2048", m["memory_mb"])
	}
	// The daemon addresses VMs by id; the orchestrator's id must be forwarded
	// so DELETE /vms/{id} targets the physical VM, not a 404.
	if m["id"] != "id-1" {
		t.Errorf("id = %v, want id-1 (daemon must use the orchestrator id)", m["id"])
	}
}

func TestVMAttributesOmitsEmptyID(t *testing.T) {
	vm := VM{TypeName: "thiscloud_vm", Name: "web1", VCPUs: 2, MemoryMB: 1024, Image: "alma9"}
	data, err := json.Marshal(vm.Attributes())
	if err != nil {
		t.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatal(err)
	}
	if _, ok := m["id"]; ok {
		t.Errorf("id must be omitted when empty (daemon generates one): %s", data)
	}
}

func TestNetworkAttributesNilDNSNotNull(t *testing.T) {
	net := Network{TypeName: "thiscloud_network", ResourceID: "n-1", Name: "net1", CIDR: "10.0.0.0/24"}
	data, err := json.Marshal(net.Attributes())
	if err != nil {
		t.Fatal(err)
	}
	if contains(string(data), "dns:null") {
		t.Errorf("dns must serialize as [] not null: %s", data)
	}
	if contains(string(data), "vlan:null") {
		t.Errorf("vlan must be omitted when nil: %s", data)
	}
}

func TestStoragePoolAttributesDefaults(t *testing.T) {
	pool := StoragePool{TypeName: "thiscloud_storage_pool", ResourceID: "p-1", Name: "pool1"}
	data, err := json.Marshal(pool.Attributes())
	if err != nil {
		t.Fatal(err)
	}
	raw := string(data)
	if contains(raw, "devices:null") {
		t.Errorf("devices must serialize as [] not null: %s", raw)
	}
	if contains(raw, "pool_type:\"\"") {
		t.Errorf("pool_type must default to linstor: %s", raw)
	}
	if contains(raw, "replication:0") {
		t.Errorf("replication must default to 2: %s", raw)
	}
}

func contains(haystack, needle string) bool {
	for i := 0; i+len(needle) <= len(haystack); i++ {
		if haystack[i:i+len(needle)] == needle {
			return true
		}
	}
	return false
}