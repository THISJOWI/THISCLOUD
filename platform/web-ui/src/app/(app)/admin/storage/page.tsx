"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, createResource, deleteResource, Resource } from "@/lib/api";
import { ResourceTable } from "@/components/ui";
import { Header } from "@/components/header";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function StoragePage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/storage");
  const [pools, setPools] = useState<Resource[]>([]);
  const [error, setError] = useState("");
  const [name, setName] = useState("");
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  async function refresh() {
    setPools(await listResources("thiscloud_storage_pool").catch(() => []));
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  async function onCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || creating) return;
    setCreating(true);
    setError("");
    try {
      await createResource("thiscloud_storage_pool", {
        name: name.trim(),
        pool_type: "linstor",
        replication: 2,
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
    if (!confirm(`Delete storage pool ${id}? This cannot be undone.`)) return;
    setDeleting(id);
    setError("");
    try {
      await deleteResource("thiscloud_storage_pool", id);
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
        <Header title="Storage" />
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
        <Header title="Storage" />
        <main className="content">
          <div className="page-header">
            <h1 className="page-title">Storage</h1>
          </div>
          <p className="error">{authError || "Access denied"}</p>
          <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
        </main>
      </>
    );
  }

  return (
    <>
      <Header title="Storage" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Storage</h1>
            <p className="page-subtitle">Storage pools and replication</p>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        <div className="glass-panel" style={{ padding: 16, marginBottom: 16 }}>
          <form onSubmit={onCreate} className="form-row" style={{ marginBottom: 0 }}>
            <input
              className="form-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Storage pool name"
              disabled={creating}
            />
            <button
              type="submit"
              className="btn btn-primary"
              disabled={creating || !name}
            >
              {creating ? "Creating..." : "Create Pool"}
            </button>
          </form>
        </div>

        <ResourceTable
          title="All Storage Pools"
          headers={["id", "name", "pool_type", "replication"]}
          rows={pools}
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