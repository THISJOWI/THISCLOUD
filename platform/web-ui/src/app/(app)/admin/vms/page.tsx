"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, createResource, deleteResource, Resource } from "@/lib/api";
import { ResourceTable } from "@/components/ui";
import { Header } from "@/components/header";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function VmsPage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/vms");
  const [vms, setVms] = useState<Resource[]>([]);
  const [error, setError] = useState("");
  const [name, setName] = useState("");
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  async function refresh() {
    setVms(await listResources("thiscloud_vm").catch(() => []));
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  async function onCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || creating) return;
    if (name.trim().length < 2) {
      setError("VM name must be at least 2 characters");
      return;
    }
    setCreating(true);
    setError("");
    try {
      // No client-generated id — the daemon assigns the canonical id.
      await createResource("thiscloud_vm", {
        name: name.trim(),
        vcpus: 2,
        memory_mb: 2048,
        disk_gb: 20,
      });
      setName("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  }

  async function onDelete(id: string) {
    if (!confirm(`Delete VM ${id}? This cannot be undone.`)) return;
    setDeleting(id);
    setError("");
    try {
      await deleteResource("thiscloud_vm", id);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setDeleting(null);
    }
  }

  if (authorized === null) {
    return (
      <>
        <Header title="Virtual Machines" />
        <main className="content">
          <div className="loading-page">
            <div className="spinner" />
            Checking authorization...
          </div>
        </main>
      </>
    );
  }

  if (authorized === false) {
    return (
      <>
        <Header title="Virtual Machines" />
        <main className="content">
          <div className="page-header">
            <h1 className="page-title">Virtual Machines</h1>
          </div>
          <p className="error">{authError || "Access denied"}</p>
          <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
        </main>
      </>
    );
  }

  return (
    <>
      <Header title="Virtual Machines" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Virtual Machines</h1>
            <p className="page-subtitle">Create and manage virtual machines</p>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        <div className="glass-panel" style={{ padding: 16, marginBottom: 16 }}>
          <form onSubmit={onCreate} className="form-row" style={{ marginBottom: 0 }}>
            <input
              className="form-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="VM name"
              disabled={creating}
            />
            <button
              type="submit"
              className="btn btn-primary"
              disabled={creating || !name}
            >
              {creating ? "Creating..." : "Create VM"}
            </button>
          </form>
        </div>

        <ResourceTable
          title="All Virtual Machines"
          headers={["id", "name", "vcpus", "memory_mb", "status"]}
          rows={vms}
          onDelete={(_, id) => onDelete(id)}
        />

        {deleting && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
            <div className="spinner" style={{ marginRight: 0 }} />
            <span className="text-muted">Deleting {deleting}...</span>
          </div>
        )}
      </main>
    </>
  );
}