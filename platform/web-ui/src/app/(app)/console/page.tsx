"use client";

import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { Header } from "@/components/header";
import { StatusBadge } from "@/components/ui";

const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://127.0.0.1:8081/ws/console";

export default function ConsolePage() {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: '"SF Mono", "Fira Code", "Cascadia Code", monospace',
      theme: { background: "#0d1017", foreground: "#e6e9ef", cursor: "#3b82f6" },
    });
    term.open(containerRef.current);
    termRef.current = term;

    term.writeln("THISCLOUD — client console");
    term.writeln("");
    term.writeln("Connecting to cluster...");

    function connect() {
      const ws = new WebSocket(WS_URL);
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
        setError("Connection failed — backend may be offline");
        term.writeln("");
        term.writeln("ERROR: Could not connect to terminal backend.");
        term.writeln(`Attempted: ${WS_URL}`);
        term.writeln("Check that the thiscloud API server is running.");
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
  }, []);

  return (
    <>
      <Header title="Console" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Cluster Console</h1>
            <p className="page-subtitle">Interactive terminal session</p>
          </div>
          <StatusBadge status={connected ? "connected" : "disconnected"} />
        </div>

        {error && <p className="error">{error}</p>}

        <div className="console-wrap">
          <div className="console-body" ref={containerRef} />
        </div>
      </main>
    </>
  );
}
