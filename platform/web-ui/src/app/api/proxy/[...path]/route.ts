import { NextRequest, NextResponse } from "next/server";
import { cookies } from "next/headers";
import { isMutation, verifySessionToken } from "@/lib/session";

const API_URL = process.env.API_URL ?? "http://127.0.0.1:8081";
const PUBLIC_PROXY_PATHS = new Set(["/healthz"]);

/**
 * Server-side API proxy.
 * Client components call /api/proxy/... which this handler forwards to the backend.
 * This keeps API_URL server-only and allows auth headers to be injected.
 */
async function proxyRequest(
  request: NextRequest,
  path: string[]
): Promise<NextResponse> {
  const targetPath = "/" + path.join("/");
  const targetUrl = new URL(targetPath, API_URL);

  // Forward query parameters
  request.nextUrl.searchParams.forEach((value, key) => {
    targetUrl.searchParams.set(key, value);
  });

  // Read session cookie and forward as Authorization header
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;
  const claims = await verifySessionToken(session);

  if (!claims && !PUBLIC_PROXY_PATHS.has(targetPath)) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }

  if (claims && isMutation(request.method)) {
    const headerToken = request.headers.get("x-csrf-token");
    const cookieToken = cookieStore.get("csrf-token")?.value;
    if (
      !headerToken ||
      !cookieToken ||
      headerToken !== cookieToken ||
      headerToken !== claims.csrfToken
    ) {
      return NextResponse.json({ error: "Invalid CSRF token" }, { status: 403 });
    }
  }

  const headers = new Headers();
  if (session && claims) {
    headers.set("Authorization", `Bearer ${session}`);
  }

  // Forward content-type
  const contentType = request.headers.get("content-type");
  if (contentType) {
    headers.set("Content-Type", contentType);
  }

  // Forward CSRF token
  const csrfToken = request.headers.get("x-csrf-token");
  if (csrfToken) {
    headers.set("X-CSRF-Token", csrfToken);
  }

  // Forward cookies to backend
  const cookieHeader = request.headers.get("cookie");
  if (cookieHeader) {
    headers.set("Cookie", cookieHeader);
  }

  const init: RequestInit = {
    method: request.method,
    headers,
  };

  // Forward body for non-GET/HEAD requests
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.text();
  }

  try {
    const res = await fetch(targetUrl.toString(), init);
    const body = await res.text();

    return new NextResponse(body, {
      status: res.status,
      statusText: res.statusText,
      headers: {
        "Content-Type": res.headers.get("Content-Type") ?? "application/json",
      },
    });
  } catch (error) {
    return NextResponse.json(
      { error: "Backend unreachable", detail: String(error) },
      { status: 502 }
    );
  }
}

export async function GET(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const { path } = await params;
  return proxyRequest(request, path);
}

export async function POST(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const { path } = await params;
  return proxyRequest(request, path);
}

export async function PUT(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const { path } = await params;
  return proxyRequest(request, path);
}

export async function PATCH(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const { path } = await params;
  return proxyRequest(request, path);
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const { path } = await params;
  return proxyRequest(request, path);
}
