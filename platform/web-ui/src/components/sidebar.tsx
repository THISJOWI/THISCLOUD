"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";

export function Sidebar() {
  const pathname = usePathname();
  const [vmCount, setVmCount] = useState<number | null>(null);

  useEffect(() => {
    fetch("/api/proxy/resources/thiscloud_vm", { cache: "no-store" })
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => setVmCount(Array.isArray(data) ? data.length : null))
      .catch(() => setVmCount(null));
  }, []);

  function isActive(href: string): boolean {
    if (href === "/") return pathname === "/" || pathname === "/admin";
    return pathname === href || pathname.startsWith(href + "/");
  }

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
            <Link
              href="/"
              className={`tree-node ${isActive("/") ? "active" : ""}`}
            >
              <span className="tree-icon">🖥</span>
              Host-01
              <span className="tree-dot" title="Online" />
            </Link>
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
        <div className="sidebar-footer-meta">THISCLOUD v0.2.4</div>
      </div>
    </aside>
  );
}