"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Image, listImages, registerImage } from "@/lib/api";
import { Header } from "@/components/header";
import { useAdminAuth } from "@/lib/use-admin-auth";

export default function ImagesPage() {
  const { authorized, error: authError } = useAdminAuth("/?redirect=/admin/images");
  const [images, setImages] = useState<Image[]>([]);
  const [error, setError] = useState("");
  const [showImport, setShowImport] = useState(false);
  const [importForm, setImportForm] = useState({
    name: "",
    source: "",
    format: "qcow2",
    os_family: "alma",
    version: "",
  });
  const [importing, setImporting] = useState(false);

  async function refresh() {
    setImages(await listImages().catch(() => []));
  }

  useEffect(() => {
    if (authorized) {
      refresh().catch((e) => setError(String(e)));
    }
  }, [authorized]);

  async function onImport(e: React.FormEvent) {
    e.preventDefault();
    if (!importForm.name.trim() || !importForm.source.trim()) {
      setError("Image name and source URL are required");
      return;
    }
    setImporting(true);
    setError("");
    try {
      await registerImage({
        name: importForm.name.trim(),
        source: importForm.source.trim(),
        format: importForm.format,
        os_family: importForm.os_family,
        version: importForm.version.trim(),
      });
      setShowImport(false);
      setImportForm({
        name: "",
        source: "",
        format: "qcow2",
        os_family: "alma",
        version: "",
      });
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setImporting(false);
    }
  }

  function formatBytes(bytes?: number) {
    if (!bytes) return "—";
    const gb = bytes / (1024 ** 3);
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = bytes / (1024 ** 2);
    return `${mb.toFixed(0)} MB`;
  }

  if (authorized === null) {
    return (
      <>
        <Header title="Images" />
        <main className="content">
          <div className="loading-page">
            <div className="spinner" />
            Checking authorization...
          </div>
        </main>
      </>
    );
  }

  if (authorized === false) {
    return (
      <>
        <Header title="Images" />
        <main className="content">
          <div className="page-header">
            <h1 className="page-title">Images</h1>
          </div>
          <p className="error">{authError || "Access denied"}</p>
          <Link href="/" className="btn btn-secondary">Return to Dashboard</Link>
        </main>
      </>
    );
  }

  return (
    <>
      <Header title="Images" />
      <main className="content">
        <div className="page-header">
          <div>
            <h1 className="page-title">Images</h1>
            <p className="page-subtitle">
              Image registry — bootable disk images and ISOs used by VMs
            </p>
          </div>
          <div className="page-actions">
            <button className="btn btn-primary" onClick={() => setShowImport(true)}>
              + Register Image
            </button>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        {showImport && (
          <form
            className="glass-panel"
            style={{ padding: 16, marginBottom: 16 }}
            onSubmit={onImport}
          >
            <div className="form-grid">
              <div>
                <label className="field-label" htmlFor="img-name">Name</label>
                <input
                  id="img-name"
                  className="form-input"
                  value={importForm.name}
                  onChange={(e) => setImportForm({ ...importForm, name: e.target.value })}
                  placeholder="alma9-minimal"
                />
              </div>
              <div>
                <label className="field-label" htmlFor="img-source">Source URL</label>
                <input
                  id="img-source"
                  className="form-input"
                  value={importForm.source}
                  onChange={(e) => setImportForm({ ...importForm, source: e.target.value })}
                  placeholder="https://example.com/img.qcow2"
                />
              </div>
              <div>
                <label className="field-label" htmlFor="img-format">Format</label>
                <select
                  id="img-format"
                  className="form-select"
                  value={importForm.format}
                  onChange={(e) => setImportForm({ ...importForm, format: e.target.value })}
                >
                  <option value="qcow2">qcow2</option>
                  <option value="iso">iso</option>
                  <option value="raw">raw</option>
                  <option value="cloud-init">cloud-init</option>
                </select>
              </div>
              <div>
                <label className="field-label" htmlFor="img-os">OS family</label>
                <select
                  id="img-os"
                  className="form-select"
                  value={importForm.os_family}
                  onChange={(e) => setImportForm({ ...importForm, os_family: e.target.value })}
                >
                  <option value="generic">generic</option>
                  <option value="alma">alma</option>
                  <option value="ubuntu">ubuntu</option>
                  <option value="debian">debian</option>
                  <option value="fedora">fedora</option>
                  <option value="rocky">rocky</option>
                </select>
              </div>
              <div>
                <label className="field-label" htmlFor="img-version">Version</label>
                <input
                  id="img-version"
                  className="form-input"
                  value={importForm.version}
                  onChange={(e) => setImportForm({ ...importForm, version: e.target.value })}
                  placeholder="9"
                />
              </div>
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button type="submit" className="btn btn-primary" disabled={importing}>
                {importing ? "Registering..." : "Register"}
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setShowImport(false)}
              >
                Cancel
              </button>
            </div>
          </form>
        )}

        <div className="table-wrap">
          <div className="table-toolbar">
            <span className="table-title">Image Registry</span>
            <div className="table-actions">
              <span className="text-muted" style={{ fontSize: 12 }}>
                {images.length} image{images.length !== 1 ? "s" : ""}
              </span>
            </div>
          </div>
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Format</th>
                <th>OS Family</th>
                <th>Version</th>
                <th>Size</th>
                <th>Template</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {images.length === 0 && (
                <tr>
                  <td colSpan={7} className="empty">
                    No images registered
                  </td>
                </tr>
              )}
              {images.map((img) => (
                <tr key={img.id ?? img.name}>
                  <td>
                    <span className="id-cell" style={{ maxWidth: 220 }}>
                      {img.name}
                    </span>
                  </td>
                  <td>{img.format ?? "—"}</td>
                  <td>{img.os_family ?? "—"}</td>
                  <td>{img.version || "—"}</td>
                  <td>{formatBytes(img.size_bytes)}</td>
                  <td>{img.template ? "yes" : "no"}</td>
                  <td>{img.status ?? "available"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </main>
    </>
  );
}