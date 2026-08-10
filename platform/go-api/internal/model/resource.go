package model

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

func (r VM) Type() ResourceType { return ResourceVM }
func (r VM) ID() string         { return r.ResourceID }
func (r VM) Attributes() map[string]any {
	return map[string]any{
		"name":      r.Name,
		"vcpus":     r.VCPUs,
		"memory_mb": r.MemoryMB,
		"disk_gb":   r.DiskGB,
		"image":     r.Image,
		"networks":  r.Networks,
		"status":    r.Status,
	}
}

func (r Network) Type() ResourceType { return ResourceNetwork }
func (r Network) ID() string         { return r.ResourceID }
func (r Network) Attributes() map[string]any {
	return map[string]any{
		"name":    r.Name,
		"cidr":    r.CIDR,
		"gateway": r.Gateway,
		"vlan":    r.VLAN,
		"dns":     r.DNS,
	}
}

func (r StoragePool) Type() ResourceType { return ResourceStorage }
func (r StoragePool) ID() string         { return r.ResourceID }
func (r StoragePool) Attributes() map[string]any {
	return map[string]any{
		"name":        r.Name,
		"pool_type":   r.PoolType,
		"devices":     r.Devices,
		"replication": r.Replication,
	}
}
