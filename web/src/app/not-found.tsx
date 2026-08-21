"use client";

import Link from "next/link";

export default function NotFound() {
  return (
    <main className="content" style={{ textAlign: "center", paddingTop: 80 }}>
      <h2 style={{ fontSize: 36, fontWeight: 700, marginBottom: 8, color: "var(--fg-muted)" }}>404</h2>
      <p style={{ color: "var(--fg-secondary)", marginBottom: 24, fontSize: 14 }}>
        The page you are looking for does not exist or has been moved.
      </p>
      <Link href="/" className="btn btn-secondary">Return to Portal</Link>
    </main>
  );
}
