"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import {
  listResources,
  createResource,
  deleteResource,
  Resource,
} from "@/lib/api";
import { StatCard, ResourceTable } from "@/components/ui";
import { Header } from "@/components/header";

export default function AdminPage() {
  const router = useRouter();
  const [vms, setVms] = useState<Resource[]>([]);
  const [networks, setNetworks] = useState<Resource[]>([]);
  const [pools, setPools] = useState<Resource[]>([]);
  const [error, setError] = useState("");
  const [name, setName] = useState("");
  const [authorized, setAuthorized] = useState<boolean | null>(null);
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);

  useEffect(() => {
    async function checkAuth() {
      try {
        const res = await fetch("/api/auth/me", { cache: "no-store" });
        if (!res.ok) {
          router.push("/?redirect=/admin");
          return;
        }
        const user = await res.json();
        if (!user.isAdmin) {
          setError("Access denied: admin privileges required");
          setAuthorized(false);
          return;
        }
        setAuthorized(true);
      } catch {
        router.push("/?redirect=/admin");
      }
    }
    checkAuth();
  }, [router]);

  async function refresh() {
    const [v, n, p] = await Promise.all([
      listResources("thiscloud_vm").catch(() => []),
      listResources("thiscloud_network").catch(() => []),
      listResources("thiscloud_storage_pool").catch(() => []),
    ]);
    setVms(v);
    setNetworks(n);
    setPools(p);
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
      await createResource("thiscloud_vm", {
        id: `vm-${crypto.randomUUID()}`,
        name,
        vcpus: 2,
        memory_mb: 2048,
        disk_gb: 20,
        status: "running",
      });
      setName("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  }

  async function onDelete(kind: string, id: string) {
    if (!confirm(`Delete ${kind}:${id}? This cannot be undone.`)) return;
    setDeleting(`${kind}:${id}`);
    setError("");
    try {
      await deleteResource(kind, id);
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
        <Header title="Admin" />
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
        <Header title="Admin" />
        <main className="content">
          <div className="page-header">
            <h1 className="page-title">Admin Panel</h1>
          </div>
          <p className="error">{error || "Access denied"}</p>
          <Link href="/" className="btn btn-secondary">Return to Portal</Link>
        </main>
      </>
    );
  }

  return (
    <>
      <Header title="Admin — Virtual Machines" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Virtual Machines</h1>
            <p className="page-subtitle">Create and manage virtual machines</p>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        <div className="grid">
          <StatCard label="VMs" value={vms.length} icon="▣" accent="var(--accent)" />
          <StatCard label="Networks" value={networks.length} icon="⬡" accent="var(--ok)" />
          <StatCard label="Storage" value={pools.length} icon="⬢" accent="var(--warn)" />
        </div>

        <div className="table-wrap" style={{ marginBottom: 16 }}>
          <div className="table-toolbar">
            <span className="table-title">Create Virtual Machine</span>
          </div>
          <div style={{ padding: "12px 16px" }}>
            <form onSubmit={onCreate} className="form-row">
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
        </div>

        <ResourceTable
          title="All Virtual Machines"
          headers={["id", "name", "vcpus", "memory_mb", "status"]}
          rows={vms}
          onDelete={onDelete}
        />

        <div id="networks">
          <ResourceTable
            title="Networks"
            headers={["id", "name", "cidr", "gateway"]}
            rows={networks}
            onDelete={onDelete}
          />
        </div>

        <div id="storage">
          <ResourceTable
            title="Storage Pools"
            headers={["id", "name", "pool_type", "replication"]}
            rows={pools}
            onDelete={onDelete}
          />
        </div>

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
