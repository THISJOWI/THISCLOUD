// Command api-server is the THISCLOUD orchestrator API. It exposes the
// infrastructure state (Terraform-provider-shaped CRUD) and bridges the web UI
// to the physical resources managed by the thiscloudd Rust daemon.
package main

import (
	"log"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"thiscloud/api/internal/api"
	"thiscloud/api/internal/backend"
	"thiscloud/api/internal/state"
)

func main() {
	statePath := env("THISCLOUD_STATE_FILE", filepath.Join(".", "thiscloud.tfstate"))
	daemonURL := env("THISCLOUD_API_URL", "http://127.0.0.1:8080")
	bind := env("THISCLOUD_API_BIND", "127.0.0.1:8081")

	store, err := state.NewStore(statePath)
	if err != nil {
		log.Fatalf("state store: %v", err)
	}

	client := backend.NewClient(daemonURL)
	server := api.NewServer(store, client)

	log.Printf("thiscloud api listening on %s (state: %s, daemon: %s)", bind, statePath, daemonURL)
	srv := &http.Server{
		Addr:         bind,
		Handler:      server.Handler(),
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  120 * time.Second,
	}
	if err := srv.ListenAndServe(); err != nil {
		log.Fatal(err)
	}
}

func env(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}