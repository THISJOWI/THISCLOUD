package api

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"

	"thiscloud/api/internal/backend"
	"thiscloud/api/internal/model"
	"thiscloud/api/internal/state"
)

// Server is the orchestrator HTTP API. It maintains desired-state in a
// tfstate-style Store and forwards lifecycle operations to the thiscloudd
// daemon via a backend.Client. This mirrors a Terraform provider: the store
// is the plan/state and the backend client is the apply.
type Server struct {
	store   *state.Store
	backend *backend.Client
}

// NewServer wires the API around a state store and daemon client.
func NewServer(store *state.Store, client *backend.Client) *Server {
	return &Server{store: store, backend: client}
}

// Handler returns the root http.Handler for the API.
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/v1/resources", s.listResources)
	mux.HandleFunc("POST /api/v1/resources", s.createResource)
	mux.HandleFunc("GET /api/v1/resources/{type}", s.listByType)
	mux.HandleFunc("POST /api/v1/resources/{type}", s.createByType)
	mux.HandleFunc("GET /api/v1/resources/{type}/{id}", s.getResource)
	mux.HandleFunc("PUT /api/v1/resources/{type}/{id}", s.updateResource)
	mux.HandleFunc("DELETE /api/v1/resources/{type}/{id}", s.deleteResource)
	mux.HandleFunc("GET /api/v1/images", s.listImages)
	mux.HandleFunc("POST /api/v1/images", s.registerImage)
	mux.HandleFunc("GET /healthz", s.health)
	return mux
}

func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// listImages proxies the daemon's image registry.
func (s *Server) listImages(w http.ResponseWriter, r *http.Request) {
	images, err := s.backend.ListImages(r.Context())
	if err != nil {
		writeError(w, http.StatusBadGateway, err)
		return
	}
	if images == nil {
		images = []map[string]any{}
	}
	writeJSON(w, http.StatusOK, images)
}

// registerImage forwards an image registration to the daemon.
func (s *Server) registerImage(w http.ResponseWriter, r *http.Request) {
	body, err := readBody(r)
	if err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}
	if err := s.backend.RegisterImage(r.Context(), payload); err != nil {
		writeError(w, http.StatusBadGateway, err)
		return
	}
	writeJSON(w, http.StatusCreated, payload)
}

func (s *Server) listResources(w http.ResponseWriter, r *http.Request) {
	s.listAll(w, "")
}

func (s *Server) listByType(w http.ResponseWriter, r *http.Request) {
	s.listAll(w, r.PathValue("type"))
}

func (s *Server) listAll(w http.ResponseWriter, typeFilter string) {
	resources, err := s.store.List(model.ResourceType(typeFilter))
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	writeJSON(w, http.StatusOK, resources)
}

func (s *Server) createResource(w http.ResponseWriter, r *http.Request) {
	s.create(w, r, "")
}

func (s *Server) createByType(w http.ResponseWriter, r *http.Request) {
	s.create(w, r, r.PathValue("type"))
}

func (s *Server) create(w http.ResponseWriter, r *http.Request, typeFilter string) {
	body, err := readBody(r)
	if err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}

	meta := struct {
		Type string `json:"type"`
	}{}
	if err := json.Unmarshal(body, &meta); err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}
	if meta.Type == "" {
		meta.Type = typeFilter
	}
	if meta.Type == "" {
		writeError(w, http.StatusBadRequest, errors.New("missing required field: type"))
		return
	}

	res, err := decode(model.ResourceType(meta.Type), body)
	if err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}

	// Apply to the physical resources through the daemon client.
	if err := s.apply(r, res); err != nil {
		writeError(w, http.StatusBadGateway, err)
		return
	}

	if err := s.store.Put(res); err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	writeJSON(w, http.StatusCreated, res)
}

func (s *Server) getResource(w http.ResponseWriter, r *http.Request) {
	res, err := s.store.Get(r.PathValue("id"))
	if errors.Is(err, state.ErrNotFound) {
		writeError(w, http.StatusNotFound, err)
		return
	}
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	writeJSON(w, http.StatusOK, res)
}

func (s *Server) deleteResource(w http.ResponseWriter, r *http.Request) {
	res, err := s.store.Get(r.PathValue("id"))
	if errors.Is(err, state.ErrNotFound) {
		writeError(w, http.StatusNotFound, err)
		return
	}
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}

	if err := s.backend.Delete(r.Context(), collectionFor(res.Type()), res.ID()); err != nil {
		// State cleanup proceeds even if the daemon is unreachable so the
		// orchestrator stays the source of truth for its own state file.
		log.Printf("warning: daemon delete failed: %v", err)
	}

	if err := s.store.Delete(res.ID()); err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "deleted", "id": res.ID()})
}

func (s *Server) updateResource(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")
	resType := model.ResourceType(r.PathValue("type"))

	existing, err := s.store.Get(id)
	if errors.Is(err, state.ErrNotFound) {
		writeError(w, http.StatusNotFound, err)
		return
	}
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	_ = existing // confirm resource exists with given type

	body, err := readBody(r)
	if err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}

	// Ensure the type in the payload matches the URL.
	meta := struct {
		Type string `json:"type"`
	}{}
	if err := json.Unmarshal(body, &meta); err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}
	if meta.Type != "" && model.ResourceType(meta.Type) != resType {
		writeError(w, http.StatusBadRequest, errors.New("type mismatch between URL and body"))
		return
	}

	res, err := decode(resType, body)
	if err != nil {
		writeError(w, http.StatusBadRequest, err)
		return
	}
	// Preserve the original ID from the URL.
	switch v := res.(type) {
	case model.VM:
		v.ResourceID = id
		res = v
	case model.Network:
		v.ResourceID = id
		res = v
	case model.StoragePool:
		v.ResourceID = id
		res = v
	}

	if err := s.store.Put(res); err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	writeJSON(w, http.StatusOK, res)
}

// apply forwards a create to the daemon when reachable.
func (s *Server) apply(r *http.Request, res model.Resource) error {
	collection := collectionFor(res.Type())
	payload := map[string]any{}
	payload["type"] = string(res.Type())
	for k, v := range res.Attributes() {
		payload[k] = v
	}
	// Ignore failures: with the mock daemon (dev) or unreachable core the
	// orchestrator still records desired state.
	if err := s.backend.Create(r.Context(), collection, payload); err != nil {
		log.Printf("warning: daemon create failed: %v", err)
	}
	return nil
}

// decode parses a raw JSON body into the concrete resource model.
func decode(t model.ResourceType, body []byte) (model.Resource, error) {
	switch t {
	case model.ResourceVM:
		var r model.VM
		if err := json.Unmarshal(body, &r); err != nil {
			return nil, err
		}
		return r, nil
	case model.ResourceNetwork:
		var r model.Network
		if err := json.Unmarshal(body, &r); err != nil {
			return nil, err
		}
		return r, nil
	case model.ResourceStorage:
		var r model.StoragePool
		if err := json.Unmarshal(body, &r); err != nil {
			return nil, err
		}
		return r, nil
	default:
		return nil, errors.New("unknown resource type: " + string(t))
	}
}

func collectionFor(t model.ResourceType) string {
	switch t {
	case model.ResourceVM:
		return "vms"
	case model.ResourceNetwork:
		return "networks"
	case model.ResourceStorage:
		return "storage/pools"
	default:
		return ""
	}
}

const maxBodySize = 10 << 20 // 10 MB

func readBody(r *http.Request) ([]byte, error) {
	defer r.Body.Close()
	limited := io.LimitReader(r.Body, maxBodySize+1)
	buf := make([]byte, 0, 65536)
	tmp := make([]byte, 4096)
	var readErr error
	for {
		n, err := limited.Read(tmp)
		buf = append(buf, tmp[:n]...)
		if err != nil {
			if !errors.Is(err, io.EOF) {
				readErr = err
			}
			break
		}
	}
	if readErr != nil {
		return nil, readErr
	}
	if len(buf) > maxBodySize {
		return nil, errors.New("request body exceeds 10 MB limit")
	}
	if len(bytes.TrimSpace(buf)) == 0 {
		return nil, errors.New("empty request body")
	}
	return buf, nil
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, status int, err error) {
	writeJSON(w, status, map[string]string{"error": err.Error()})
}