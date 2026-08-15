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
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+"/images", nil)
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

	var images []map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&images); err != nil {
		return nil, err
	}
	return images, nil
}

// RegisterImage forwards a registration payload to the daemon (POST /images).
func (c *Client) RegisterImage(ctx context.Context, image any) error {
	return c.request(ctx, http.MethodPost, "images", image)
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
		return fmt.Errorf("daemon returned %s: %s", resp.Status, strings.TrimSpace(string(data)))
	}
	return nil
}