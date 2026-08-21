"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, createResource, deleteResource, Resource } from "@/lib/api";
import { ResourceTable, ContextHeader } from "@/components/ui";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function NetworksPage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/networks");
  const [networks, setNetworks] = useState<Resource[]>([]);
  const [error, setError] = useState("");
  const [name, setName] = useState("");
  const [cidr, setCidr] = useState("");
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  async function refresh() {
    setNetworks(await listResources("thiscloud_network").catch(() => []));
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  async function onCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || !cidr.trim() || creating) return;
    setCreating(true);
    setError("");
    try {
      await createResource("thiscloud_network", {
        name: name.trim(),
        cidr: cidr.trim(),
      });
      setName("");
      setCidr("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  }

  async function onDelete(id: string) {
    if (!confirm(`Delete network ${id}? This cannot be undone.`)) return;
    setDeleting(id);
    setError("");
    try {
      await deleteResource("thiscloud_network", id);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setDeleting(null);
    }
  }

  if (authorized === null) {
    return (
      <div className="content">
        <div className="loading-page">
          <div className="spinner" />
          Checking authorization...
        </div>
      </div>
    );
  }

  if (authorized === false) {
    return (
      <div className="content">
        <ContextHeader title="Networks" />
        <p className="error">{authError || "Access denied"}</p>
        <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
      </div>
    );
  }

  return (
    <div className="content">
      <ContextHeader
        title="Networks"
        meta="Virtual networks and address ranges"
      />

        {error && <p className="error">{error}</p>}

        <div className="glass-panel" style={{ padding: 16, marginBottom: 16 }}>
          <form onSubmit={onCreate} className="form-row" style={{ marginBottom: 0 }}>
            <input
              className="form-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Network name"
              disabled={creating}
            />
            <input
              className="form-input"
              value={cidr}
              onChange={(e) => setCidr(e.target.value)}
              placeholder="CIDR (e.g. 10.0.0.0/24)"
              disabled={creating}
            />
            <button
              type="submit"
              className="btn btn-primary"
              disabled={creating || !name || !cidr}
            >
              {creating ? "Creating..." : "Create Network"}
            </button>
          </form>
        </div>

        <ResourceTable
          title="All Networks"
          headers={["id", "name", "cidr", "gateway"]}
          rows={networks}
          onDelete={(_, id) => onDelete(id)}
        />

        {deleting && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
            <div className="spinner" style={{ marginRight: 0 }} />
            <span className="text-muted">Deleting {deleting}...</span>
          </div>
        )}
      </div>
    );
}