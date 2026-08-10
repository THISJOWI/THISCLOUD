import { listResources, health } from "@/lib/api";
import { StatCard, ResourceTable } from "@/components/ui";
import { Header } from "@/components/header";
import { cookies } from "next/headers";

export const dynamic = "force-dynamic";

export default async function PortalPage() {
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;

  if (!session) {
    return (
      <>
        <Header title="Portal" />
        <main className="content">
          <div className="page-header">
            <div>
              <h1 className="page-title">Cloud Portal</h1>
              <p className="page-subtitle">Sign in to access your resources</p>
            </div>
          </div>
          <p className="text-secondary">
            Please <a href="/login">sign in</a> to continue.
          </p>
        </main>
      </>
    );
  }

  const [online, vms, networks, pools] = await Promise.all([
    health(),
    listResources("thiscloud_vm").catch(() => []),
    listResources("thiscloud_network").catch(() => []),
    listResources("thiscloud_storage_pool").catch(() => []),
  ]);

  return (
    <>
      <Header title="Portal" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Dashboard</h1>
            <p className="page-subtitle">Cluster overview and resource summary</p>
          </div>
        </div>

        <div className="grid">
          <StatCard
            label="Virtual Machines"
            value={vms.length}
            icon="▣"
            accent="var(--accent)"
          />
          <StatCard
            label="Networks"
            value={networks.length}
            icon="⬡"
            accent="var(--ok)"
          />
          <StatCard
            label="Storage Pools"
            value={pools.length}
            icon="⬢"
            accent="var(--warn)"
          />
          <StatCard
            label="API Status"
            value={online ? "Online" : "Offline"}
            icon="◉"
            accent={online ? "var(--ok)" : "var(--error)"}
          />
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
