"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { listResources, deleteResource, Resource } from "@/lib/api";
import { ContextHeader, ResourceTable } from "@/components/ui";
import { CreateVmModal } from "@/components/create-vm-modal";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function VmsPage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/vms");
  const [vms, setVms] = useState<Resource[]>([]);
  const [networks, setNetworks] = useState<string[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [error, setError] = useState("");
  const [deleting, setDeleting] = useState<string | null>(null);

  async function refresh() {
    const [v, n] = await Promise.all([
      listResources("thiscloud_vm").catch(() => []),
      listResources("thiscloud_network").catch(() => []),
    ]);
    setVms(v);
    setNetworks(
      n
        .map((x) => String(x.name || ""))
        .filter((x) => x.length > 0)
        .sort()
    );
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

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
        <ContextHeader title="Virtual Machines" />
        <p className="error">{authError || "Access denied"}</p>
        <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
      </div>
    );
  }

  return (
    <div className="content">
      <ContextHeader
        title="Virtual Machines"
        meta={`${vms.length} VMs • ${vms.filter((v) => String(v.status).toLowerCase() === "running").length} running`}
        actions={
          <button className="btn btn-primary" onClick={() => setShowCreate(true)}>
            + Create VM
          </button>
        }
      />

        {error && <p className="error">{error}</p>}

        <ResourceTable
          title="All Virtual Machines"
          headers={["id", "name", "vcpus", "memory_mb", "image", "status"]}
          rows={vms}
          onDelete={(_, id) => onDelete(id)}
        />

        {deleting && (
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
            <div className="spinner" style={{ marginRight: 0 }} />
            <span className="text-muted">Deleting {deleting}...</span>
          </div>
        )}

        {showCreate && (
          <CreateVmModal
            networks={networks}
            onClose={() => setShowCreate(false)}
            onCreated={() => refresh()}
          />
        )}
      </div>
    );
}