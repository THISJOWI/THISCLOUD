import Link from "next/link";
import { listResources, health, readiness } from "@/lib/api";
import { ContextHeader, StatCard, ResourceTable } from "@/components/ui";
import { cookies } from "next/headers";

export const dynamic = "force-dynamic";

export default async function PortalPage() {
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;

  if (!session) {
    return (
      <div className="content">
        <ContextHeader title="Cloud Portal" meta="Sign in to access your resources" />
        <p className="text-secondary">
          Please <a href="/login">sign in</a> to continue.
        </p>
      </div>
    );
  }

  const [online, ready, vms, networks, pools] = await Promise.all([
    health(),
    readiness(),
    listResources("thiscloud_vm").catch(() => []),
    listResources("thiscloud_network").catch(() => []),
    listResources("thiscloud_storage_pool").catch(() => []),
  ]);

  const running = vms.filter(
    (v) => String(v.status).toLowerCase() === "running"
  ).length;

  const readyLabel = ready
    ? ready.status === "ready"
      ? "Ready"
      : "Degraded"
    : "Unknown";

  return (
    <div className="content">
      <ContextHeader
        title="Host-01 Summary"
        meta={
          <>
            Cluster: {online ? "Online" : "Offline"} <span className="sep">•</span>{" "}
            Readiness: {readyLabel} <span className="sep">•</span> {vms.length} VMs{" "}
            <span className="sep">•</span> {networks.length} Networks{" "}
            <span className="sep">•</span> {pools.length} Storage Pools
          </>
        }
        actions={
          <Link href="/admin/vms" className="btn btn-primary">
            + Create VM
          </Link>
        }
      />

      <div className="grid">
        <StatCard
          label="Virtual Machines"
          value={vms.length}
          sub={`${running} running`}
          icon="▣"
        />
        <StatCard
          label="Networks"
          value={networks.length}
          icon="⌁"
        />
        <StatCard
          label="Storage Pools"
          value={pools.length}
          icon="⬢"
        />
        <StatCard
          label="API Status"
          value={ready ? (ready.status === "ready" ? "Ready" : "Degraded") : online ? "Online" : "Offline"}
          sub={ready?.checks ? Object.entries(ready.checks).map(([k, v]) => `${k}=${v}`).join(", ") : undefined}
          icon="◉"
          progress={ready ? (ready.status === "ready" ? 100 : 50) : online ? 100 : 0}
          progressColor={
            ready
              ? ready.status === "ready"
                ? "var(--ok)"
                : "var(--warn)"
              : online
                ? "var(--ok)"
                : "var(--error)"
          }
        />
      </div>

      <ResourceTable
        title="Virtual Machines"
        headers={["id", "name", "vcpus", "memory_mb", "image", "status"]}
        rows={vms}
      />
    </div>
  );
}