"use client";

import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { Header } from "@/components/header";
import { listResources, Resource } from "@/lib/api";
import { StatusBadge } from "@/components/ui";

const WS_PROTO = typeof window !== "undefined" && window.location.protocol === "https:" ? "wss" : "ws";
const IS_DEV = process.env.NODE_ENV !== "production";
const WS_BASE = process.env.NEXT_PUBLIC_WS_URL ?? (IS_DEV
  ? "ws://127.0.0.1:8080"
  : "");

export default function ConsolePage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [vms, setVms] = useState<Resource[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    listResources("thiscloud_vm")
      .then((v) => setVms(v))
      .catch(() => setVms([]));
  }, []);

  useEffect(() => {
    if (!containerRef.current || !selected) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: '"JetBrains Mono", "SF Mono", "Fira Code", monospace',
      theme: { background: "#070b13", foreground: "#dce2f7", cursor: "#b4c5ff" },
    });
    term.open(containerRef.current);
    termRef.current = term;

    const base = WS_BASE || `${WS_PROTO}://${window.location.host}`;
    const wsUrl = `${base}/api/v1/vms/${selected}/console/ws`;
    term.writeln("THISCLOUD — VM console");
    term.writeln("");
    term.writeln(`Connecting to ${selected}...`);

    function connect() {
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
        setError("");
        term.writeln("Connected.");
        term.writeln("");
      };

      ws.onmessage = (event) => {
        const data = typeof event.data === "string" ? event.data : "";
        term.write(data);
      };

      ws.onerror = () => {
        setError("Connection failed — daemon may be offline or VM not found");
        term.writeln("");
        term.writeln("ERROR: Could not connect to console endpoint.");
        term.writeln(`Attempted: ${wsUrl}`);
      };

      ws.onclose = (event) => {
        setConnected(false);
        if (event.code !== 1000) {
          term.writeln("");
          term.writeln(`Disconnected (code ${event.code}). Reconnecting in 3s...`);
          setTimeout(connect, 3000);
        }
      };

      term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(data);
        }
      });
    }

    connect();

    return () => {
      wsRef.current?.close(1000, "Component unmounting");
      term.dispose();
    };
  }, [selected]);

  return (
    <>
      <Header title="Console" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Cluster Console</h1>
            <p className="page-subtitle">Interactive terminal session for a VM</p>
          </div>
          {selected && (
            <StatusBadge status={connected ? "connected" : "disconnected"} />
          )}
        </div>

        <div className="glass-panel" style={{ padding: 16, marginBottom: 16 }}>
          <label className="stat-label" style={{ display: "block", marginBottom: 8 }}>
            Virtual Machine
          </label>
          <select
            className="form-input"
            style={{ width: "100%", maxWidth: 420 }}
            value={selected}
            onChange={(e) => setSelected(e.target.value)}
          >
            <option value="">Select a VM...</option>
            {vms.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name || v.id} ({v.id})
              </option>
            ))}
          </select>
        </div>

        {error && <p className="error">{error}</p>}

        {selected ? (
          <div className="console-wrap">
            <div className="console-body" ref={containerRef} />
          </div>
        ) : (
          <div className="glass-panel" style={{ padding: 32, textAlign: "center" }}>
            <div style={{ fontSize: 40, marginBottom: 12, opacity: 0.5 }}>🖥</div>
            <p className="text-secondary">Select a virtual machine above to open its console.</p>
          </div>
        )}
      </main>
    </>
  );
}