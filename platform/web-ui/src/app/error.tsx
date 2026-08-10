"use client";

import { useEffect } from "react";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("[App Error]", error);
  }, [error]);

  return (
    <main className="content" style={{ maxWidth: 500, margin: "64px auto" }}>
      <div className="table-wrap">
        <div style={{ padding: 24 }}>
          <h2 style={{ color: "var(--error)", marginBottom: 8, fontSize: 16 }}>Something went wrong</h2>
          <p className="mono" style={{ color: "var(--error)", marginBottom: 12, wordBreak: "break-word" }}>
            {error.message}
          </p>
          {error.digest && (
            <p className="text-muted" style={{ fontSize: 11, marginBottom: 16 }}>Error ID: {error.digest}</p>
          )}
          <button onClick={reset} className="btn btn-primary">Try again</button>
        </div>
      </div>
    </main>
  );
}
