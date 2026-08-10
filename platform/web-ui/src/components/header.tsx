"use client";

import { useEffect, useState } from "react";

export function Header({ title }: { title?: string }) {
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
        <div className="topbar-breadcrumb">
          <span>THISCLOUD</span>
          {title && (
            <>
              <span className="sep">›</span>
              <span className="current">{title}</span>
            </>
          )}
        </div>
      </div>
      <div className="topbar-right">
        <div className="topbar-status">
          <span
            className={`status-dot ${
              online === null ? "" : online ? "online" : "offline"
            }`}
          />
          {online === null ? "Checking..." : online ? "Connected" : "Offline"}
        </div>
        <div className="topbar-user">
          <span className="avatar">
            {user?.id?.[0]?.toUpperCase() ?? "U"}
          </span>
          {user?.id ?? "user"}
          {user?.role && (
            <span className="topbar-role">{user.role}</span>
          )}
          <button
            className="topbar-logout"
            title="Sign out"
            onClick={() => (window.location.href = "/api/auth/logout")}
          >
            ⏻
          </button>
        </div>
      </div>
    </header>
  );
}
