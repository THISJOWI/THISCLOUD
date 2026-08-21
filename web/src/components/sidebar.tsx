"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";

type ClusterNode = {
  id: string;
  name: string;
  role?: string;
  address?: string;
  state?: string;
  cpus_total?: number;
  memory_total_mb?: number;
  vms?: number;
};

const STATE_DOT: Record<string, string> = {
  online: "ok",
  offline: "off",
  draining: "busy",
};

export function Sidebar() {
  const pathname = usePathname();
  const [nodes, setNodes] = useState<ClusterNode[]>([]);
  const [vmCount, setVmCount] = useState<number | null>(null);

  useEffect(() => {
    Promise.all([
      fetch("/api/proxy/api/v1/nodes", { cache: "no-store" }).then((r) =>
        r.ok ? r.json() : []
      ),
      fetch("/api/proxy/api/v1/resources/thiscloud_vm", {
        cache: "no-store",
      }).then((r) => (r.ok ? r.json() : [])),
    ])
      .then(([nodeData, vmData]) => {
        setNodes(Array.isArray(nodeData) ? nodeData : []);
        setVmCount(Array.isArray(vmData) ? vmData.length : null);
      })
      .catch(() => {
        setNodes([]);
        setVmCount(null);
      });
  }, []);

  function isActive(href: string): boolean {
    if (href === "/") return pathname === "/" || pathname === "/admin";
    return pathname === href || pathname.startsWith(href + "/");
  }

  const nodeList = nodes.length > 0 ? nodes : [];

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <div className="sidebar-header-title">
          <span className="tree-icon">ᐳ</span>
          <h2>Resource Tree</h2>
        </div>
        <div className="sidebar-header-sub">Management Console</div>
      </div>

      <nav className="sidebar-nav">
        <div className="tree-root">
          <div className="tree-root-label">
            <span className="tree-icon">🗀</span>
            Datacenter
          </div>
          <div className="tree-group">
            {nodeList.map((node) => {
              const state = node.state ?? "online";
              const active = nodeList.length === 1 || node.role === "master";
              return (
                <Link
                  key={node.id}
                  href="/"
                  className={`tree-node ${isActive("/") && active ? "active" : ""}`}
                >
                  <span className="tree-icon">🖥</span>
                  {node.name}
                  {node.role === "master" && (
                    <span className="tree-role">M</span>
                  )}
                  <span
                    className={`tree-dot ${STATE_DOT[state] ?? "off"}`}
                    title={state}
                  />
                </Link>
              );
            })}
            <Link
              href="/admin/vms"
              className={`tree-node ${isActive("/admin/vms") ? "active" : ""}`}
            >
              <span className="tree-icon">▣</span>
              Virtual Machines
              {vmCount !== null && <span className="tree-count">{vmCount}</span>}
            </Link>
            <Link
              href="/admin/networks"
              className={`tree-node ${isActive("/admin/networks") ? "active" : ""}`}
            >
              <span className="tree-icon">⌁</span>
              Networks
            </Link>
            <Link
              href="/admin/storage"
              className={`tree-node ${isActive("/admin/storage") ? "active" : ""}`}
            >
              <span className="tree-icon">⬢</span>
              Storage
            </Link>
            <Link
              href="/admin/images"
              className={`tree-node ${isActive("/admin/images") ? "active" : ""}`}
            >
              <span className="tree-icon">⬡</span>
              Images
            </Link>
          </div>
        </div>
      </nav>

      <div className="sidebar-footer">
        <Link href="/console" className={`sidebar-footer-item ${isActive("/console") ? "active" : ""}`}>
          <span className="tree-icon">&gt;_</span>
          Console / Logs
        </Link>
        <div className="sidebar-footer-meta">THISCLOUD v0.3.0</div>
      </div>
    </aside>
  );
}