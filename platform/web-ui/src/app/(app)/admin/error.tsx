"use client";

import { useEffect } from "react";
import Link from "next/link";
import { Header } from "@/components/header";

export default function AdminError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("[Admin Error]", error);
  }, [error]);

  return (
    <>
      <Header title="Admin — Error" />
      <main className="content" style={{ maxWidth: 500, margin: "64px auto" }}>
        <div className="table-wrap">
          <div style={{ padding: 24 }}>
            <h2 style={{ color: "var(--error)", marginBottom: 8, fontSize: 16 }}>Admin Panel Error</h2>
            <p className="text-muted" style={{ marginBottom: 12, fontSize: 13 }}>
              An error occurred while loading the admin panel. This may indicate a
              permissions issue or backend connectivity problem.
            </p>
            <p className="mono" style={{ color: "var(--error)", marginBottom: 16, wordBreak: "break-word", fontSize: 12 }}>
              {error.message}
            </p>
            <div style={{ display: "flex", gap: 8 }}>
              <button onClick={reset} className="btn btn-primary">Retry</button>
              <Link href="/" className="btn btn-secondary">Back to Portal</Link>
            </div>
          </div>
        </div>
      </main>
    </>
  );
}
