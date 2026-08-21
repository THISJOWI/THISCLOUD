"use client";

import { useEffect, useState } from "react";

export function Header() {
  const [online, setOnline] = useState<boolean | null>(null);
  const [user, setUser] = useState<{ id: string; role: string } | null>(null);

  useEffect(() => {
    async function load() {
      try {
        const [healthRes, meRes] = await Promise.allSettled([
          fetch("/api/proxy/healthz", { cache: "no-store" }),
          fetch("/api/auth/me", { cache: "no-store" }),
        ]);
        if (healthRes.status === "fulfilled") setOnline(healthRes.value.ok);
        if (meRes.status === "fulfilled" && meRes.value.ok) {
          const data = await meRes.value.json();
          setUser(data);
        }
      } catch {
        setOnline(false);
      }
    }
    load();
    const iv = setInterval(() => {
      fetch("/api/proxy/healthz", { cache: "no-store" })
        .then((r) => setOnline(r.ok))
        .catch(() => setOnline(false));
    }, 30000);
    return () => clearInterval(iv);
  }, []);

  return (
    <header className="topbar">
      <div className="topbar-left">
        <div className="topbar-brand">
          <div className="brand-mark">T</div>
          <span className="brand-name">THISCLOUD</span>
          <span className="brand-divider" />
        </div>
        <button className="cluster-pill" title="Cluster status">
          <span className="pill-dot" />
          <span className="pill-text">
            {online === null ? "Cluster: Checking" : online ? "Cluster: Online" : "Cluster: Offline"}
          </span>
        </button>
      </div>
      <div className="topbar-right">
        <button className="topbar-icon-btn" title="Notifications">
          <span role="img" aria-label="bell">◔</span>
        </button>
        <button className="topbar-icon-btn" title="Settings">
          <span role="img" aria-label="settings">⚙</span>
        </button>
        <button className="topbar-icon-btn" title="Help">
          <span role="img" aria-label="help">?</span>
        </button>
        <div className="topbar-user-sep">
          <div className="topbar-user">
            <span className="avatar">{user?.id?.[0]?.toUpperCase() ?? "U"}</span>
            <span className="user-meta">
              <span className="user-name">{user?.id ?? "user"}</span>
              {user?.role && <span className="topbar-role">{user.role}</span>}
            </span>
          </div>
          <button
            className="topbar-logout"
            title="Sign out"
            onClick={() => (window.location.href = "/api/auth/logout")}
          >
            Logout
          </button>
        </div>
      </div>
    </header>
  );
}