"use client";

import { useEffect, useState } from "react";
import { Image, listImages, registerImage, uploadImage } from "@/lib/api";

type FormState = {
  name: string;
  node: string;
  image: string;
  disk_gb: string;
  vcpus: string;
  memory_mb: string;
  networks: string[];
  uefi: boolean;
  tpm: boolean;
  ha: boolean;
};

const TABS = ["General", "OS / Disk", "CPU", "Network", "Options"] as const;

const EMPTY: FormState = {
  name: "",
  node: "",
  image: "",
  disk_gb: "20",
  vcpus: "2",
  memory_mb: "2048",
  networks: [],
  uefi: false,
  tpm: false,
  ha: false,
};

export function CreateVmModal({
  networks,
  onClose,
  onCreated,
}: {
  networks: string[];
  onClose: () => void;
  onCreated: () => void;
}) {
  const [tab, setTab] = useState<(typeof TABS)[number]>("General");
  const [form, setForm] = useState<FormState>(EMPTY);
  const [images, setImages] = useState<Image[]>([]);
  const [importing, setImporting] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [importForm, setImportForm] = useState({
    name: "",
    source: "",
    format: "qcow2",
    os_family: "alma",
    version: "",
  });
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  useEffect(() => {
    listImages().then(setImages).catch(() => {});
  }, []);

  function set<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function onImport(e: React.FormEvent) {
    e.preventDefault();
    if (!importForm.name.trim() || !importForm.source.trim()) {
      setError("Image name and source URL are required");
      return;
    }
    setImporting(true);
    setError("");
    try {
      const img = await registerImage({
        name: importForm.name.trim(),
        source: importForm.source.trim(),
        format: importForm.format,
        os_family: importForm.os_family,
        version: importForm.version.trim(),
      });
      setImages((imgs) => [...imgs, img]);
      setForm((f) => ({ ...f, image: img.name || "" }));
      setNotice(`Image "${importForm.name}" registered.`);
      setImportForm({
        name: "",
        source: "",
        format: "qcow2",
        os_family: "alma",
        version: "",
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setImporting(false);
    }
  }

  async function onUpload(e: React.FormEvent) {
    e.preventDefault();
    if (!uploadFile) {
      setError("Select a local file to upload");
      return;
    }
    const baseName = uploadFile.name.replace(/\.(iso|qcow2|qcow|img|raw)$/i, "");
    const inferredFormat = uploadFile.name.toLowerCase().endsWith(".iso")
      ? "iso"
      : uploadFile.name.toLowerCase().endsWith(".qcow2") || uploadFile.name.toLowerCase().endsWith(".qcow")
        ? "qcow2"
        : "raw";
    setUploading(true);
    setError("");
    setNotice("");
    try {
      const img = await registerImage({
        name: importForm.name.trim() || baseName,
        source: "",
        format: importForm.format || inferredFormat,
        os_family: importForm.os_family,
        version: importForm.version.trim(),
      });
      await uploadImage(img.id!, uploadFile);
      const fresh = await listImages();
      setImages(fresh);
      setForm((f) => ({ ...f, image: img.name || "" }));
      setNotice(`Image "${img.name}" uploaded (${uploadFile.name}).`);
      setUploadFile(null);
      setImportForm({ name: "", source: "", format: "qcow2", os_family: "alma", version: "" });
    } catch (err) {
      setError(String(err));
    } finally {
      setUploading(false);
    }
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (form.name.trim().length < 2) {
      setError("VM name must be at least 2 characters");
      return;
    }
    const vcpus = parseInt(form.vcpus, 10);
    const memory_mb = parseInt(form.memory_mb, 10);
    const disk_gb = parseInt(form.disk_gb, 10);
    if (!vcpus || vcpus < 1) return setError("CPU cores must be at least 1");
    if (!memory_mb || memory_mb < 512)
      return setError("Memory must be at least 512 MB");
    if (!disk_gb || disk_gb < 1) return setError("Disk size must be at least 1 GB");

    setSubmitting(true);
    setError("");
    try {
      const { createResource } = await import("@/lib/api");
      await createResource("thiscloud_vm", {
        name: form.name.trim(),
        vcpus,
        memory_mb,
        disk_gb,
        image: form.image,
        networks: form.networks,
        node: form.node,
        uefi: form.uefi,
        tpm: form.tpm,
        ha: form.ha,
      });
      onCreated();
      onClose();
    } catch (err) {
      setError(String(err));
      setSubmitting(false);
    }
  }

  const runningImages = images.filter(
    (i) => i.format === "iso" || i.format === "qcow2" || i.format === "raw"
  );

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <form className="modal" onSubmit={onSubmit}>
        <div className="modal-header">
          <span className="modal-title">Create Virtual Machine</span>
          <button type="button" className="modal-close" onClick={onClose} title="Close">
            ×
          </button>
        </div>

        <div className="modal-tabs">
          {TABS.map((t) => (
            <button
              key={t}
              type="button"
              className={`modal-tab ${tab === t ? "active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        <div className="modal-body">
          {error && <p className="error">{error}</p>}
          {notice && (
            <p className="text-secondary" style={{ fontSize: 12, marginBottom: 12 }}>
              {notice}
            </p>
          )}

          {tab === "General" && (
            <div className="form-grid">
              <div>
                <label className="field-label" htmlFor="vm-name">Name</label>
                <input
                  id="vm-name"
                  className="form-input"
                  value={form.name}
                  onChange={(e) => set("name", e.target.value)}
                  placeholder="my-vm"
                />
              </div>
              <div>
                <label className="field-label" htmlFor="vm-node">Node (optional)</label>
                <input
                  id="vm-node"
                  className="form-input"
                  value={form.node}
                  onChange={(e) => set("node", e.target.value)}
                  placeholder="auto (best-fit scheduler)"
                />
              </div>
            </div>
          )}

          {tab === "OS / Disk" && (
            <div>
              <div className="form-grid">
                <div>
                  <label className="field-label" htmlFor="vm-image">Image / ISO</label>
                  <select
                    id="vm-image"
                    className="form-select"
                    value={form.image}
                    onChange={(e) => set("image", e.target.value)}
                  >
                    <option value="">— select an image —</option>
                    {runningImages.map((img) => (
                      <option key={img.id ?? img.name} value={img.name}>
                        {img.name}
                        {img.version ? ` (${img.version})` : ""} — {img.format}
                        {img.os_family ? ` · ${img.os_family}` : ""}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="field-label" htmlFor="vm-disk">Disk size (GB)</label>
                  <input
                    id="vm-disk"
                    type="number"
                    min={1}
                    className="form-input"
                    value={form.disk_gb}
                    onChange={(e) => set("disk_gb", e.target.value)}
                  />
                </div>
              </div>

              <div className="import-box">
                <div className="field-label">Register new image</div>
                <div className="form-grid">
                  <div>
                    <label className="field-label" htmlFor="imp-name">Name</label>
                    <input
                      id="imp-name"
                      className="form-input"
                      value={importForm.name}
                      onChange={(e) =>
                        setImportForm({ ...importForm, name: e.target.value })
                      }
                      placeholder="alma9-minimal"
                    />
                  </div>
                  <div>
                    <label className="field-label" htmlFor="imp-source">Source URL</label>
                    <input
                      id="imp-source"
                      className="form-input"
                      value={importForm.source}
                      onChange={(e) =>
                        setImportForm({ ...importForm, source: e.target.value })
                      }
                      placeholder="https://example.com/img.qcow2"
                    />
                  </div>
                  <div>
                    <label className="field-label" htmlFor="imp-format">Format</label>
                    <select
                      id="imp-format"
                      className="form-select"
                      value={importForm.format}
                      onChange={(e) =>
                        setImportForm({ ...importForm, format: e.target.value })
                      }
                    >
                      <option value="qcow2">qcow2</option>
                      <option value="iso">iso</option>
                      <option value="raw">raw</option>
                    </select>
                  </div>
                  <div>
                    <label className="field-label" htmlFor="imp-os">OS family</label>
                    <select
                      id="imp-os"
                      className="form-select"
                      value={importForm.os_family}
                      onChange={(e) =>
                        setImportForm({ ...importForm, os_family: e.target.value })
                      }
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
                    <label className="field-label" htmlFor="imp-version">Version</label>
                    <input
                      id="imp-version"
                      className="form-input"
                      value={importForm.version}
                      onChange={(e) =>
                        setImportForm({ ...importForm, version: e.target.value })
                      }
                      placeholder="9"
                    />
                  </div>
                </div>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={onImport}
                  disabled={importing}
                  style={{ marginTop: 4 }}
                >
                  {importing ? "Registering..." : "Register image"}
                </button>

                <div className="import-divider">or upload a local file</div>
                <div className="form-grid">
                  <div style={{ gridColumn: "1 / -1" }}>
                    <input
                      id="imp-file"
                      type="file"
                      accept=".iso,.qcow2,.qcow,.img,.raw"
                      className="form-input"
                      onChange={(e) => setUploadFile(e.target.files?.[0] ?? null)}
                    />
                    {uploadFile && (
                      <p className="field-hint" style={{ marginTop: 4 }}>
                        {uploadFile.name} — {(uploadFile.size / 1024 / 1024).toFixed(1)} MB
                      </p>
                    )}
                  </div>
                </div>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={onUpload}
                  disabled={uploading || !uploadFile}
                  style={{ marginTop: 4 }}
                >
                  {uploading ? "Uploading..." : "Upload image"}
                </button>
              </div>
            </div>
          )}

          {tab === "CPU" && (
            <div className="form-grid">
              <div>
                <label className="field-label" htmlFor="vm-vcpus">Cores</label>
                <input
                  id="vm-vcpus"
                  type="number"
                  min={1}
                  className="form-input"
                  value={form.vcpus}
                  onChange={(e) => set("vcpus", e.target.value)}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="vm-mem">Memory (MB)</label>
                <input
                  id="vm-mem"
                  type="number"
                  min={512}
                  step={512}
                  className="form-input"
                  value={form.memory_mb}
                  onChange={(e) => set("memory_mb", e.target.value)}
                />
              </div>
            </div>
          )}

          {tab === "Network" && (
            <div>
              {networks.length === 0 ? (
                <p className="text-muted">
                  No networks available. Create one in the Networks page first.
                </p>
              ) : (
                networks.map((n) => (
                  <label className="form-checkbox" key={n}>
                    <input
                      type="checkbox"
                      checked={form.networks.includes(n)}
                      onChange={(e) =>
                        set(
                          "networks",
                          e.target.checked
                            ? [...form.networks, n]
                            : form.networks.filter((x) => x !== n)
                        )
                      }
                    />
                    {n}
                  </label>
                ))
              )}
            </div>
          )}

          {tab === "Options" && (
            <div>
              <label className="form-checkbox">
                <input
                  type="checkbox"
                  checked={form.uefi}
                  onChange={(e) => set("uefi", e.target.checked)}
                />
                Boot with UEFI firmware (OVMF)
              </label>
              <label className="form-checkbox">
                <input
                  type="checkbox"
                  checked={form.tpm}
                  onChange={(e) => set("tpm", e.target.checked)}
                />
                Attach vTPM device (requires UEFI)
              </label>
              <label className="form-checkbox">
                <input
                  type="checkbox"
                  checked={form.ha}
                  onChange={(e) => set("ha", e.target.checked)}
                />
                High availability (auto-failover on node outage)
              </label>
              <p className="field-hint" style={{ marginTop: 8 }}>
                The boot disk lives at /var/lib/thiscloud/vms/{"{name}"}.qcow2 on
                the chosen node. Data disks and snapshots are managed by the
                storage module.
              </p>
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn btn-primary" disabled={submitting}>
            {submitting ? "Creating..." : "Create VM"}
          </button>
        </div>
      </form>
    </div>
  );
}