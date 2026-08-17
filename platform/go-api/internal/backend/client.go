package backend

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Client is an HTTP client for the thiscloudd daemon (the Rust core).
// It is the "physical resources" side of the orchestrator bridge: the Go API
// persists desired state and forwards create/delete operations to the daemon.
type Client struct {
	baseURL string
	http    *http.Client
}

// RejectedError signals that the daemon is reachable but refused the request
// (HTTP 4xx/5xx). This is distinct from a connectivity error: the orchestrator
// must not persist desired state the daemon explicitly rejected, or phantom
// resources accumulate (e.g. a VM targeting an offline node).
type RejectedError struct {
	Status string
	Body   string
}

func (e *RejectedError) Error() string {
	if e.Body != "" {
		return fmt.Sprintf("daemon returned %s: %s", e.Status, e.Body)
	}
	return fmt.Sprintf("daemon returned %s", e.Status)
}

// NewClient returns a Client pointed at the given thiscloudd base URL.
// The daemon serves its versioned contract under /api/v1 (T0.1).
func NewClient(baseURL string) *Client {
	if baseURL == "" {
		baseURL = "http://127.0.0.1:8080"
	}
	return &Client{
		baseURL: strings.TrimSuffix(baseURL, "/") + "/api/v1",
		http:    &http.Client{Timeout: 10 * time.Second},
	}
}

// Ping probes the daemon's public health endpoint. Returns the daemon's
// reported status and whether it is reachable.
func (c *Client) Ping(ctx context.Context) (string, bool) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/healthz", nil)
	if err != nil {
		return "", false
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return "", false
	}
	defer resp.Body.Close()
	status := ""
	if resp.StatusCode == http.StatusOK {
		var body map[string]any
		if err := json.NewDecoder(io.LimitReader(resp.Body, 4096)).Decode(&body); err == nil {
			if s, ok := body["status"].(string); ok {
				status = s
			}
		}
	}
	return status, resp.StatusCode == http.StatusOK
}

// Create sends a POST to the daemon's resource collection endpoint.
func (c *Client) Create(ctx context.Context, collection string, body any) error {
	return c.request(ctx, http.MethodPost, collection, body)
}

// Delete sends a DELETE to the daemon's resource endpoint.
func (c *Client) Delete(ctx context.Context, collection, id string) error {
	return c.request(ctx, http.MethodDelete, fmt.Sprintf("%s/%s", collection, id), nil)
}

// ListImages returns the daemon's image registry (GET /images).
func (c *Client) ListImages(ctx context.Context) ([]map[string]any, error) {
	return c.list(ctx, "images")
}

// RegisterImage forwards a registration payload to the daemon (POST /images).
func (c *Client) RegisterImage(ctx context.Context, image any) error {
	return c.request(ctx, http.MethodPost, "images", image)
}

// UploadImage streams raw artifact bytes to the daemon (PUT /images/{id}/upload).
// Used for local file uploads (ISO/qcow2) that never have a fetchable URL.
func (c *Client) UploadImage(ctx context.Context, id string, data io.Reader) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodPut, c.baseURL+"/images/"+id+"/upload", data)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/octet-stream")

	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		return &RejectedError{
			Status: resp.Status,
			Body:   strings.TrimSpace(string(body)),
		}
	}
	return nil
}

// ListNodes returns the cluster nodes registered with the daemon (GET /nodes).
// Nodes are read-through: they self-register via heartbeat and are never
// part of the orchestrator's desired state.
func (c *Client) ListNodes(ctx context.Context) ([]map[string]any, error) {
	return c.list(ctx, "nodes")
}

// ListVMDisk returns the daemon's live VM list (GET /vms), used to flatten
// boot and data disks for the web UI's read-only disks view.
func (c *Client) ListVMDisk(ctx context.Context) ([]map[string]any, error) {
	return c.list(ctx, "vms")
}

// StartVM sends a POST to the daemon to start a VM.
func (c *Client) StartVM(ctx context.Context, id string) error {
	return c.request(ctx, http.MethodPost, fmt.Sprintf("vms/%s/start", id), nil)
}

// StopVM sends a POST to the daemon to stop a VM.
func (c *Client) StopVM(ctx context.Context, id string) error {
	return c.request(ctx, http.MethodPost, fmt.Sprintf("vms/%s/stop", id), nil)
}

// list performs a GET on a daemon collection and decodes the JSON array.
func (c *Client) list(ctx context.Context, collection string) ([]map[string]any, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/"+collection, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		data, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		return nil, fmt.Errorf("daemon returned %s: %s", resp.Status, strings.TrimSpace(string(data)))
	}

	var out []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	return out, nil
}

func (c *Client) request(ctx context.Context, method, path string, body any) error {
	var reader io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return err
		}
		reader = strings.NewReader(string(data))
	}
	req, err := http.NewRequestWithContext(ctx, method, c.baseURL+"/"+path, reader)
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		data, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		return &RejectedError{
			Status: resp.Status,
			Body:   strings.TrimSpace(string(data)),
		}
	}
	return nil
}