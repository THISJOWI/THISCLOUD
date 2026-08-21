"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, listNodes, ClusterNode, Resource } from "@/lib/api";
import { ContextHeader, StatCard, ResourceTable } from "@/components/ui";
import { useAdminAuth } from "@/lib/use-admin-auth";

function TelemetryCard({
  value,
  label,
  max = 100,
  unit = "",
  color = "var(--primary)",
}: {
  value: number;
  label: string;
  max?: number;
  unit?: string;
  color?: string;
}) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div className="stat-card">
      <span className="stat-label">{label}</span>
      <div className="stat-value">
        {Math.round(value)}
        <span className="text-muted" style={{ fontSize: "var(--body-sm)", fontWeight: 400 }}>
          {unit}
        </span>
      </div>
      <div className="stat-trend">
        {value} / {max} {unit.trim()}
      </div>
      <div className="progress">
        <div
          className="progress-fill"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
    </div>
  );
}

export default function AdminPage() {
  const { authorized, error: authError } = useAdminAuth();
  const [vms, setVms] = useState<Resource[]>([]);
  const [networks, setNetworks] = useState<Resource[]>([]);
  const [pools, setPools] = useState<Resource[]>([]);
  const [nodes, setNodes] = useState<ClusterNode[]>([]);
  const [error, setError] = useState("");

  async function refresh() {
    const [v, n, p, nd] = await Promise.all([
      listResources("thiscloud_vm").catch(() => []),
      listResources("thiscloud_network").catch(() => []),
      listResources("thiscloud_storage_pool").catch(() => []),
      listNodes().catch(() => []),
    ]);
    setVms(v);
    setNetworks(n);
    setPools(p);
    setNodes(nd);
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  const running = vms.filter(
    (v) => String(v.status).toLowerCase() === "running"
  ).length;
  const onlineNodes = nodes.filter(
    (nd) => String(nd.state).toLowerCase() === "online"
  ).length;
  const totalCpus = vms.reduce(
    (acc, v) => acc + (Number(v.vcpus) || 0),
    0
  );
  const totalMemGb = vms.reduce(
    (acc, v) => acc + (Number(v.memory_mb) || 0) / 1024,
    0
  );
  const totalDiskGb = vms.reduce(
    (acc, v) => acc + (Number(v.disk_gb) || 0),
    0
  );

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
        <ContextHeader title="Admin Panel" />
        <p className="error">{authError || "Access denied"}</p>
        <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
      </div>
    );
  }

  return (
    <div className="content">
      <ContextHeader
        title="Cluster Summary"
        meta={
          <>
            {nodes.length} Nodes ({onlineNodes} online) <span className="sep">•</span> {vms.length} VMs{" "}
            <span className="sep">•</span> {networks.length} Networks{" "}
            <span className="sep">•</span> {pools.length} Storage Pools
          </>
        }
        actions={
          <>
            <button className="btn" onClick={refresh}>
              ↻ Refresh
            </button>
            <Link href="/admin/vms" className="btn btn-primary">
              + Create VM
            </Link>
          </>
        }
      />

      {error && <p className="error">{error}</p>}

      <div className="grid">
        <StatCard
          label="Cluster Nodes"
          value={nodes.length}
          sub={`${onlineNodes} online`}
          icon="🖥"
        />
        <StatCard
          label="Virtual Machines"
          value={vms.length}
          sub={`${running} running`}
          icon="▣"
        />
        <StatCard
          label="Networks"
          value={networks.length}
          sub={`${networks.filter((n) => String(n.status).toLowerCase() === "active").length} active`}
          icon="⌁"
        />
        <StatCard
          label="Storage Pools"
          value={pools.length}
          sub={`${pools.reduce((acc, p) => acc + (Number(p.replication) || 0), 0)} replication`}
          icon="⬢"
        />
      </div>

      {nodes.length > 0 && (
        <div className="grid">
          {nodes.map((node) => (
            <div key={node.id ?? node.name} className="stat-card">
              <span className="stat-label">
                {node.name}
                {node.role === "master" && (
                  <span className="badge badge-master">MASTER</span>
                )}
              </span>
              <div className="stat-value">
                {Number(node.cpus_total) || 0}
                <span className="text-muted" style={{ fontSize: "var(--body-sm)", fontWeight: 400 }}>
                  {" "}CPUs
                </span>
              </div>
              <div className="stat-trend">
                {node.state ?? "online"} •{" "}
                {Number(node.memory_used_mb || 0) / 1024 >= 1
                  ? `${Math.round(Number(node.memory_used_mb || 0) / 1024)} / ${Math.round(Number(node.memory_total_mb || 0) / 1024)} GB`
                  : `${Number(node.memory_used_mb || 0)} / ${Number(node.memory_total_mb || 0)} MB`}{" "}
                • {Number(node.vms) || 0} VMs
              </div>
              <div className="progress">
                <div
                  className="progress-fill"
                  style={{
                    width: `${Math.min(100, Math.round(((Number(node.cpus_used) || 0) / Math.max(1, Number(node.cpus_total) || 1)) * 100))}%`,
                    background:
                      String(node.state).toLowerCase() === "online"
                        ? "var(--ok)"
                        : "var(--warn)",
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="grid">
        <TelemetryCard label="CPU cores allocated" value={totalCpus} max={16} unit="" />
        <TelemetryCard label="Memory allocated" value={totalMemGb} max={32} unit=" GB" color="var(--secondary)" />
        <TelemetryCard label="Disk allocated" value={totalDiskGb} max={100} unit=" GB" color="var(--tertiary)" />
        <TelemetryCard label="VM utilization" value={vms.length} max={Math.max(8, vms.length)} unit=" VMs" color="var(--ok)" />
      </div>

      <div className="grid">
        <Link href="/admin/vms" className="glass-panel resource-tile">
          <div className="resource-tile-icon">▣</div>
          <div>
            <div className="resource-tile-title">Virtual Machines</div>
            <div className="resource-tile-desc">Create, manage and monitor VMs</div>
          </div>
          <span className="resource-tile-arrow">→</span>
        </Link>
        <Link href="/admin/images" className="glass-panel resource-tile">
          <div className="resource-tile-icon">⬡</div>
          <div>
            <div className="resource-tile-title">Images</div>
            <div className="resource-tile-desc">Image registry — disks and ISOs</div>
          </div>
          <span className="resource-tile-arrow">→</span>
        </Link>
        <Link href="/admin/networks" className="glass-panel resource-tile">
          <div className="resource-tile-icon">⌁</div>
          <div>
            <div className="resource-tile-title">Networks</div>
            <div className="resource-tile-desc">Virtual networks and CIDR ranges</div>
          </div>
          <span className="resource-tile-arrow">→</span>
        </Link>
        <Link href="/admin/storage" className="glass-panel resource-tile">
          <div className="resource-tile-icon">⬢</div>
          <div>
            <div className="resource-tile-title">Storage</div>
            <div className="resource-tile-desc">Storage pools and replication</div>
          </div>
          <span className="resource-tile-arrow">→</span>
        </Link>
      </div>

      <ResourceTable
        title="Virtual Machines"
        headers={["id", "name", "vcpus", "memory_mb", "image", "status"]}
        rows={vms}
      />
    </div>
  );
}