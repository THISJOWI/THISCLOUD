export type Resource = {
  type?: string;
  id: string;
  name?: string;
  [key: string]: unknown;
};

// Server-side only: never exposed to client bundle
const API_URL = process.env.API_URL ?? "http://127.0.0.1:8081";

/**
 * Get auth headers from the current session.
 * Server-side: reads session cookie and passes as Bearer token.
 * Client-side: calls a server action to get signed headers.
 */
async function getAuthHeaders(): Promise<Record<string, string>> {
  if (typeof window === "undefined") {
    // Server-side: read session cookie via next/headers
    try {
      const { cookies } = await import("next/headers");
      const cookieStore = await cookies();
      const session = cookieStore.get("session")?.value;
      if (session) {
        return { Authorization: `Bearer ${session}` };
      }
    } catch {
      // SSR or middleware context — cookies() may not be available
    }
    return {};
  }

  // Client-side: call a server action to get auth headers
  // This keeps the session token on the server
  try {
    const res = await fetch("/api/auth/headers", { cache: "no-store" });
    if (res.ok) {
      return await res.json();
    }
  } catch {
    // Fall through to unauthenticated
  }
  return {};
}

/**
 * Generate a CSRF token for mutations.
 * Client-side: reads from cookie set by server.
 * Server-side: generates from session.
 */
function getCsrfToken(): string {
  if (typeof window === "undefined") {
    return ""; // Server-side generates per-request
  }
  // Read from cookie set by server
  const match = document.cookie.match(/csrf-token=([^;]+)/);
  return match ? decodeURIComponent(match[1]) : "";
}

async function apiFetch(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const authHeaders = await getAuthHeaders();
  const csrfToken = getCsrfToken();

  const headers: Record<string, string> = {
    ...authHeaders,
    ...(options.headers as Record<string, string>),
  };

  // Add CSRF token for mutations
  if (options.method && ["POST", "PUT", "PATCH", "DELETE"].includes(options.method)) {
    if (csrfToken) {
      headers["X-CSRF-Token"] = csrfToken;
    }
  }

  // Server-side: call backend directly via API_URL
  // Client-side: route through /api/proxy to keep API_URL server-only
  const url = typeof window === "undefined"
    ? `${API_URL}${path}`
    : `/api/proxy${path}`;

  return fetch(url, {
    ...options,
    headers,
    cache: "no-store",
  });
}

export async function listResources(type?: string): Promise<Resource[]> {
  const path = type
    ? `/api/v1/resources/${type}`
    : "/api/v1/resources";
  const res = await apiFetch(path);
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] GET ${path} failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  const data = await res.json();
  // Guard against a backend that serializes an empty result set as null.
  return Array.isArray(data) ? data : [];
}

export async function createResource(
  type: string,
  attrs: Record<string, unknown>
): Promise<Resource> {
  const res = await apiFetch(`/api/v1/resources/${type}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ type, ...attrs }),
  });
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] POST /api/v1/resources/${type} failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  return res.json();
}

export async function deleteResource(
  type: string,
  id: string
): Promise<void> {
  if (!id) {
    throw new Error(
      `Cannot delete ${type}: resource has no id (corrupt or legacy state)`
    );
  }
  const res = await apiFetch(`/api/v1/resources/${type}/${id}`, {
    method: "DELETE",
  });
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] DELETE /api/v1/resources/${type}/${id} failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
}

export async function health(): Promise<boolean> {
  try {
    const res = await apiFetch("/healthz");
    return res.ok;
  } catch {
    return false;
  }
}

export type Image = {
  id?: string;
  name: string;
  source: string;
  sha256?: string;
  size_bytes?: number;
  format?: string;
  os_family?: string;
  version?: string;
  template?: boolean;
  status?: string;
};

export type ClusterNode = {
  id?: string;
  name: string;
  role?: string;
  address?: string;
  hostname?: string;
  state?: string;
  cpus_total?: number;
  cpus_used?: number;
  memory_total_mb?: number;
  memory_used_mb?: number;
  vms?: number;
  last_seen_secs?: number;
  ttl_secs?: number;
  labels?: string[];
};

export async function listNodes(): Promise<ClusterNode[]> {
  const res = await apiFetch("/api/v1/nodes");
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] GET /api/v1/nodes failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  const data = await res.json();
  return Array.isArray(data) ? data : [];
}

export async function listImages(): Promise<Image[]> {
  const res = await apiFetch("/api/v1/images");
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] GET /api/v1/images failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  const data = await res.json();
  return Array.isArray(data) ? data : [];
}

export async function registerImage(
  attrs: Record<string, unknown>
): Promise<Image> {
  const res = await apiFetch("/api/v1/images", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(attrs),
  });
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] POST /api/v1/images failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  return res.json();
}

/**
 * Upload a local artifact file (ISO/qcow2) for an already-registered image.
 * Bytes pass through the proxy untouched.
 */
export async function uploadImage(
  id: string,
  file: File
): Promise<Image> {
  const res = await apiFetch(`/api/v1/images/${id}/upload`, {
    method: "PUT",
    headers: { "Content-Type": "application/octet-stream" },
    body: file,
  });
  if (!res.ok) {
    const raw = await res.text().catch(() => "unknown error");
    console.error(`[api] PUT /api/v1/images/${id}/upload failed (${res.status}):`, raw);
    throw new Error(`API error (status ${res.status})`);
  }
  return res.json();
}
