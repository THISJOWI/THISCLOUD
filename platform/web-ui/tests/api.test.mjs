import test from "node:test";
import assert from "node:assert";
import { createServer } from "node:http";

// A tiny in-memory mock of the Go orchestrator API so the web UI's API
// contract is exercised without requiring a live daemon.
function startMock() {
  const resources = [];
  const server = createServer((req, res) => {
    const url = new URL(req.url, "http://localhost");
    res.setHeader("Content-Type", "application/json");
    if (url.pathname === "/healthz" && req.method === "GET") {
      res.end(JSON.stringify({ status: "ok" }));
      return;
    }
    const match = url.pathname.match(
      /^\/api\/v1\/resources\/(thiscloud_[^/]+)(?:\/([^/]+))?$/
    );
    if (match) {
      const type = match[1];
      const id = match[2];
      if (req.method === "GET") {
        const list = resources.filter((r) => r.type === type);
        res.end(JSON.stringify(id ? list.filter((r) => r.id === id) : list));
        return;
      }
      if (req.method === "POST") {
        let body = "";
        req.on("data", (c) => (body += c));
        req.on("end", () => {
          const r = JSON.parse(body);
          resources.push(r);
          res.statusCode = 201;
          res.end(JSON.stringify(r));
        });
        return;
      }
      if (req.method === "DELETE") {
        const idx = resources.findIndex((r) => r.id === id);
        if (idx < 0) {
          res.statusCode = 404;
          res.end(JSON.stringify({ error: "not found" }));
          return;
        }
        resources.splice(idx, 1);
        res.end(JSON.stringify({ status: "deleted" }));
        return;
      }
    }
    if (url.pathname === "/api/v1/vm-disks" && req.method === "GET") {
      res.end(
        JSON.stringify([
          {
            vm_id: "vm-1",
            vm_name: "web",
            disk_id: "",
            path: "/var/lib/thiscloud/vms/web.qcow2",
            size_gb: 20,
            kind: "boot",
            vm_status: "running",
          },
          {
            vm_id: "vm-1",
            vm_name: "web",
            disk_id: "d-1",
            path: "/data/d1.qcow2",
            size_gb: 50,
            kind: "data",
            vm_status: "running",
          },
        ])
      );
      return;
    }
    res.statusCode = 404;
    res.end(JSON.stringify({ error: "not found" }));
  });
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      resolve({ server, base: `http://127.0.0.1:${port}` });
    });
  });
}

test("health returns ok", async () => {
  const { server, base } = await startMock();
  try {
    const res = await fetch(`${base}/healthz`);
    assert.equal(res.ok, true);
    const body = await res.json();
    assert.equal(body.status, "ok");
  } finally {
    server.close();
  }
});

test("create then list a VM resource", async () => {
  const { server, base } = await startMock();
  try {
    const created = await fetch(`${base}/api/v1/resources/thiscloud_vm`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        type: "thiscloud_vm",
        id: "vm-1",
        name: "web",
        vcpus: 2,
      }),
    });
    assert.equal(created.status, 201);

    const list = await fetch(`${base}/api/v1/resources/thiscloud_vm`);
    const body = await list.json();
    assert.equal(body.length, 1);
    assert.equal(body[0].name, "web");
  } finally {
    server.close();
  }
});

test("delete removes a resource", async () => {
  const { server, base } = await startMock();
  try {
    await fetch(`${base}/api/v1/resources/thiscloud_network`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type: "thiscloud_network", id: "net-1", name: "n" }),
    });
    const del = await fetch(`${base}/api/v1/resources/thiscloud_network/net-1`, {
      method: "DELETE",
    });
    assert.equal(del.ok, true);

    const list = await fetch(`${base}/api/v1/resources/thiscloud_network`);
    assert.equal((await list.json()).length, 0);
  } finally {
    server.close();
  }
});

test("list VM disks returns boot and data rows", async () => {
  const { server, base } = await startMock();
  try {
    const res = await fetch(`${base}/api/v1/vm-disks`);
    assert.equal(res.ok, true);
    const body = await res.json();
    assert.equal(body.length, 2);
    assert.equal(body[0].kind, "boot");
    assert.equal(body[0].size_gb, 20);
    assert.equal(body[1].kind, "data");
    assert.equal(body[1].size_gb, 50);
  } finally {
    server.close();
  }
});