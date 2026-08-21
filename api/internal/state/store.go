package state

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sync"

	"thiscloud/api/internal/model"
)

// ErrNotFound is returned when a resource id does not exist in state.
var ErrNotFound = errors.New("not found")

// StateFile is the on-disk layout of the orchestrator's infrastructure state,
// deliberately shaped like a Terraform state file so the API can be surfaced
// through a Terraform provider later.
type StateFile struct {
	Version   int              `json:"version"`
	Resources []model.Resource `json:"resources"`
}

// Store is a thread-safe, JSON-persisted state store.
type Store struct {
	mu   sync.RWMutex
	path string
	file StateFile
}

// NewStore creates a store backed by the given file path. If the file does not
// exist yet it is created empty.
func NewStore(path string) (*Store, error) {
	s := &Store{path: path}
	if err := s.load(); err != nil {
		return nil, err
	}
	if err := s.persist(); err != nil {
		return nil, err
	}
	return s, nil
}

func (s *Store) load() error {
	data, err := os.ReadFile(s.path)
	if errors.Is(err, os.ErrNotExist) {
		s.file = StateFile{Version: 4, Resources: []model.Resource{}}
		return nil
	}
	if err != nil {
		return err
	}
	// Unmarshal into a raw map first so we can route each entry to the
	// concrete model type by its declared type.
	var raw struct {
		Version   int               `json:"version"`
		Resources []json.RawMessage `json:"resources"`
	}
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}
	file := StateFile{Version: raw.Version, Resources: []model.Resource{}}
	for _, r := range raw.Resources {
		res, err := decodeResource(r)
		if err != nil {
			return err
		}
		// Self-heal resources created before IDs were assigned: without an id
		// the web UI cannot render a delete button. Assigning on load makes
		// legacy entries addressable so they can be deleted from the UI.
		res = model.AssignID(res)
		file.Resources = append(file.Resources, res)
	}
	s.file = file
	return nil
}

func (s *Store) persist() error {
	if err := os.MkdirAll(filepath.Dir(s.path), 0o755); err != nil {
		return err
	}
	data, err := json.MarshalIndent(s.file, "", "  ")
	if err != nil {
		return err
	}
	tmp := s.path + ".tmp"
	if err := os.WriteFile(tmp, data, 0o644); err != nil {
		return err
	}
	if err := os.Rename(tmp, s.path); err != nil {
		os.Remove(tmp)
		return err
	}
	return nil
}

// decodeResource routes a raw JSON resource entry to its concrete model type.
func decodeResource(raw json.RawMessage) (model.Resource, error) {
	var meta struct {
		Type string `json:"type"`
	}
	if err := json.Unmarshal(raw, &meta); err != nil {
		return nil, err
	}
	switch model.ResourceType(meta.Type) {
	case model.ResourceVM:
		var r model.VM
		if err := json.Unmarshal(raw, &r); err != nil {
			return nil, err
		}
		return r, nil
	case model.ResourceNetwork:
		var r model.Network
		if err := json.Unmarshal(raw, &r); err != nil {
			return nil, err
		}
		return r, nil
	case model.ResourceStorage:
		var r model.StoragePool
		if err := json.Unmarshal(raw, &r); err != nil {
			return nil, err
		}
		return r, nil
	default:
		return nil, errors.New("unknown resource type: " + meta.Type)
	}
}

// Put stores a resource, replacing any existing one with the same id.
func (s *Store) Put(r model.Resource) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	for i, existing := range s.file.Resources {
		if existing.ID() == r.ID() {
			s.file.Resources[i] = r
			return s.persist()
		}
	}
	s.file.Resources = append(s.file.Resources, r)
	return s.persist()
}

// Get returns the resource with the given id, or ErrNotFound.
func (s *Store) Get(id string) (model.Resource, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	for _, r := range s.file.Resources {
		if r.ID() == id {
			return r, nil
		}
	}
	return nil, ErrNotFound
}

// List returns all resources, optionally filtered by type.
func (s *Store) List(t model.ResourceType) ([]model.Resource, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	// Initialize as an empty slice (not nil) so JSON serializes as `[]`
	// rather than `null` when there are no matching resources.
	out := make([]model.Resource, 0)
	for _, r := range s.file.Resources {
		if t == "" || r.Type() == t {
			out = append(out, r)
		}
	}
	return out, nil
}

// Replace swaps the entry carrying oldID for r, persisting the change. It is
// used to reconcile a resource whose persisted id diverged from the daemon's
// (legacy entries backfilled with a placeholder uuid on load): the orchestrator
// adopts the daemon's real id so lifecycle calls address the same VM.
func (s *Store) Replace(oldID string, r model.Resource) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	for i, existing := range s.file.Resources {
		if existing.ID() == oldID {
			s.file.Resources[i] = r
			return s.persist()
		}
	}
	return ErrNotFound
}

// Delete removes the resource with the given id.
func (s *Store) Delete(id string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	for i, r := range s.file.Resources {
		if r.ID() == id {
			s.file.Resources = append(s.file.Resources[:i], s.file.Resources[i+1:]...)
			return s.persist()
		}
	}
	return ErrNotFound
}

// Count returns the total number of resources in state.
func (s *Store) Count() (int, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.file.Resources), nil
}
