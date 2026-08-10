import { NextResponse } from "next/server";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

/**
 * Authenticate against the server's PAM/system users.
 * Runs `su` to verify credentials — works on any Linux with shadow passwords.
 * Requires the web-ui process to run as root (systemd service).
 */
async function authenticateSystemUser(
  username: string,
  password: string
): Promise<{ ok: boolean; isAdmin: boolean }> {
  // Validate username: only allow alphanumeric, dash, underscore (no shell injection)
  if (!/^[a-zA-Z0-9._-]{1,32}$/.test(username)) {
    return { ok: false, isAdmin: false };
  }

  try {
    // su -c runs a command as the target user; if password is wrong, it fails
    // We use printf to avoid shell expansion issues
    await execAsync(
      `printf '%s\\n' ${JSON.stringify(password)} | su -c 'echo ok' ${JSON.stringify(username)} 2>/dev/null`,
      { timeout: 5000 }
    );

    // Check if user is in the wheel group (admin)
    let isAdmin = false;
    try {
      const { stdout } = await execAsync(
        `id -nG ${JSON.stringify(username)} 2>/dev/null`,
        { timeout: 3000 }
      );
      isAdmin = stdout.trim().split(/\s+/).includes("wheel");
    } catch {
      // Not in wheel or user doesn't exist — treat as regular user
    }

    return { ok: true, isAdmin };
  } catch {
    return { ok: false, isAdmin: false };
  }
}

export async function POST(request: Request) {
  const { username, password } = await request.json();

  if (!username || !password) {
    return NextResponse.json(
      { error: "Username and password required" },
      { status: 400 }
    );
  }

  // Authenticate against server system users via PAM/su
  const { ok, isAdmin } = await authenticateSystemUser(username, password);

  if (!ok) {
    return NextResponse.json(
      { error: "Invalid credentials" },
      { status: 401 }
    );
  }

  const role = isAdmin ? "admin" : "user";

  // Create session token: base64(userId:role:expiry)
  const expiry = Date.now() + 24 * 60 * 60 * 1000;
  const token = Buffer.from(`${username}:${role}:${expiry}`).toString("base64");

  const response = NextResponse.json({ success: true, role });

  // Self-hosted platform: always set secure=false for cookie compatibility
  // (ISO runs behind nginx on HTTP port 80; HTTPS termination is external)
  response.cookies.set("session", token, {
    httpOnly: true,
    secure: false,
    sameSite: "lax",
    path: "/",
    maxAge: 24 * 60 * 60,
  });

  // CSRF token cookie (readable by JS for X-CSRF-Token header)
  const csrfToken = crypto.randomUUID();
  response.cookies.set("csrf-token", csrfToken, {
    httpOnly: false,
    secure: false,
    sameSite: "lax",
    path: "/",
    maxAge: 24 * 60 * 60,
  });

  return response;
}
