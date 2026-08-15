"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, Resource } from "@/lib/api";
import { StatCard, ResourceTable } from "@/components/ui";
import { Header } from "@/components/header";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function AdminPage() {
  const { authorized, error: authError } = useAdminAuth();
  const [vms, setVms] = useState<Resource[]>([]);
  const [networks, setNetworks] = useState<Resource[]>([]);
  const [pools, setPools] = useState<Resource[]>([]);
  const [error, setError] = useState("");

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

  const running = vms.filter((v) => String(v.status).toLowerCase() === "running").length;

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
          <p className="error">{authError || "Access denied"}</p>
          <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
        </main>
      </>
    );
  }

  return (
    <>
      <Header title="Admin" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Infrastructure</h1>
            <p className="page-subtitle">Cluster overview — resources and capacity</p>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        <div className="grid">
          <StatCard label="Virtual Machines" value={vms.length} icon="▣" accent="var(--accent)" />
          <StatCard label="Running" value={running} icon="▶" accent="var(--ok)" />
          <StatCard label="Networks" value={networks.length} icon="⬡" accent="var(--info)" />
          <StatCard label="Storage Pools" value={pools.length} icon="⬢" accent="var(--warn)" />
        </div>

        <div className="grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}>
          <Link href="/admin/vms" className="glass-panel resource-tile">
            <div className="resource-tile-icon" style={{ color: "var(--accent)" }}>▣</div>
            <div>
              <div className="resource-tile-title">Virtual Machines</div>
              <div className="resource-tile-desc">Create, manage and monitor VMs</div>
            </div>
            <span className="resource-tile-arrow">→</span>
          </Link>
          <Link href="/admin/networks" className="glass-panel resource-tile">
            <div className="resource-tile-icon" style={{ color: "var(--info)" }}>⬡</div>
            <div>
              <div className="resource-tile-title">Networks</div>
              <div className="resource-tile-desc">Virtual networks and CIDR ranges</div>
            </div>
            <span className="resource-tile-arrow">→</span>
          </Link>
          <Link href="/admin/storage" className="glass-panel resource-tile">
            <div className="resource-tile-icon" style={{ color: "var(--warn)" }}>⬢</div>
            <div>
              <div className="resource-tile-title">Storage</div>
              <div className="resource-tile-desc">Storage pools and replication</div>
            </div>
            <span className="resource-tile-arrow">→</span>
          </Link>
        </div>

        <ResourceTable
          title="Virtual Machines"
          headers={["id", "name", "vcpus", "memory_mb", "status"]}
          rows={vms}
        />
      </main>
    </>
  );
}