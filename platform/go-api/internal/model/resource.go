package model

import (
	"crypto/rand"
	"fmt"
)

// ResourceType enumerates the infrastructure resource kinds the orchestrator
// manages. Modeled after Terraform resource definitions.
type ResourceType string

const (
	ResourceVM      ResourceType = "thiscloud_vm"
	ResourceNetwork ResourceType = "thiscloud_network"
	ResourceStorage ResourceType = "thiscloud_storage_pool"
)

// Resource is the base envelope every managed object implements, mirroring
// Terraform's core ResourceData/InstanceState concept: an identifier plus a
// map of attributes.
type Resource interface {
	Type() ResourceType
	ID() string
	Attributes() map[string]any
	// DeletableID is the identifier the daemon expects on its DELETE route.
	// VMs and networks are addressed by id; storage pools have no id in the
	// daemon and are addressed by name. Using ID() for storage would 404.
	DeletableID() string
}

// VM mirrors the compute module's VM model exposed by thiscloudd.
type VM struct {
	TypeName   string   `json:"type"`
	ResourceID string   `json:"id"`
	Name       string   `json:"name"`
	VCPUs      int      `json:"vcpus"`
	MemoryMB   int      `json:"memory_mb"`
	DiskGB     int      `json:"disk_gb"`
	Image      string   `json:"image"`
	Networks   []string `json:"networks"`
	Node       string   `json:"node,omitempty"`
	UEFI       bool     `json:"uefi,omitempty"`
	TPM        bool     `json:"tpm,omitempty"`
	HA         bool     `json:"ha,omitempty"`
	Status     string   `json:"status"`
}

// Network mirrors the network module's LogicalNetwork model.
type Network struct {
	TypeName   string   `json:"type"`
	ResourceID string   `json:"id"`
	Name       string   `json:"name"`
	CIDR       string   `json:"cidr"`
	Gateway    string   `json:"gateway"`
	VLAN       *int     `json:"vlan,omitempty"`
	DNS        []string `json:"dns,omitempty"`
	Status     string   `json:"status"`
}

// StoragePool mirrors the storage module's model.
type StoragePool struct {
	TypeName    string   `json:"type"`
	ResourceID  string   `json:"id"`
	Name        string   `json:"name"`
	PoolType    string   `json:"pool_type"`
	Devices     []string `json:"devices"`
	Replication int      `json:"replication"`
}

// daemonPayload is the payload the daemon expects for each resource type. The
// Go model keeps UI-facing field names (vcpus, disk_gb), but the daemon's Rust
// structs use their own (cpus, disk_path) and reject empty/null sequences and
// empty enums. Emit exactly what the daemon serializes so apply() succeeds and
// the resource actually materializes in the daemon — otherwise deletes fail.
func (r VM) Type() ResourceType { return ResourceVM }
func (r VM) ID() string         { return r.ResourceID }
func (r VM) DeletableID() string {
	return r.ResourceID
}
func (r VM) Attributes() map[string]any {
	attrs := map[string]any{
		"name":      r.Name,
		"cpus":      r.VCPUs,
		"memory_mb": r.MemoryMB,
		"image":     r.Image,
		"networks":  nonNil(r.Networks),
	}
	// Pass the orchestrator's id so the daemon addresses this VM by the same
	// identifier. Without it the daemon generates its own UUID, the stored
	// state and the physical VM diverge, and DELETE /vms/{id} 404s.
	if r.ResourceID != "" {
		attrs["id"] = r.ResourceID
	}
	if r.Node != "" {
		attrs["node"] = r.Node
	}
	if r.UEFI {
		attrs["uefi"] = true
	}
	if r.TPM {
		attrs["tpm"] = true
	}
	if r.HA {
		attrs["ha"] = true
	}
	return attrs
}

func (r Network) Type() ResourceType { return ResourceNetwork }
func (r Network) ID() string         { return r.ResourceID }
func (r Network) DeletableID() string {
	return r.ResourceID
}
func (r Network) Attributes() map[string]any {
	attrs := map[string]any{
		"name":    r.Name,
		"cidr":    r.CIDR,
		"gateway": r.Gateway,
		"dns":     nonNil(r.DNS),
	}
	if r.VLAN != nil {
		attrs["vlan"] = *r.VLAN
	}
	return attrs
}

func (r StoragePool) Type() ResourceType { return ResourceStorage }
func (r StoragePool) ID() string         { return r.ResourceID }
func (r StoragePool) DeletableID() string {
	// The daemon's storage module keys pools by name — there is no id field.
	return r.Name
}
func (r StoragePool) Attributes() map[string]any {
	replication := r.Replication
	if replication <= 0 {
		replication = 2 // daemon default; it rejects replication == 0
	}
	return map[string]any{
		"name":        r.Name,
		"pool_type":   nonEmpty(r.PoolType, "linstor"),
		"devices":     nonNil(r.Devices),
		"replication": replication,
	}
}

// nonNil returns a non-nil slice so JSON serializes as [] instead of null;
// the daemon rejects null for sequence fields.
func nonNil(s []string) []string {
	if s == nil {
		return []string{}
	}
	return s
}

// nonEmpty returns v, or fallback when v is blank (daemon rejects "" enums).
func nonEmpty(v, fallback string) string {
	if v == "" {
		return fallback
	}
	return v
}

// newID returns a random UUID v4 string, matching the daemon's id scheme for
// resources created without an explicit identifier.
func newID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		panic(err)
	}
	b[6] = (b[6] & 0x0f) | 0x40 // version 4
	b[8] = (b[8] & 0x3f) | 0x80 // variant 10
	return fmt.Sprintf("%x-%x-%x-%x-%x", b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}

// AssignID returns res with a generated id when it has none, so every managed
// resource carries a stable identifier in the orchestrator state. Without one,
// deletes address an empty id and the API rejects them with 405.
func AssignID(res Resource) Resource {
	if res.ID() != "" {
		return res
	}
	switch v := res.(type) {
	case VM:
		v.ResourceID = newID()
		return v
	case Network:
		v.ResourceID = newID()
		return v
	case StoragePool:
		v.ResourceID = newID()
		return v
	default:
		return res
	}
}
