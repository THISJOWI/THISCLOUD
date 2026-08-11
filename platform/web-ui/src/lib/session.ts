export type SessionClaims = {
  userId: string;
  role: "admin" | "user";
  expiry: number;
  csrfToken: string;
};

const encoder = new TextEncoder();

function base64UrlEncode(input: string | ArrayBuffer): string {
  const bytes =
    typeof input === "string"
      ? encoder.encode(input)
      : new Uint8Array(input);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary)
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

function base64UrlDecode(input: string): string {
  const padded = input.replace(/-/g, "+").replace(/_/g, "/").padEnd(
    Math.ceil(input.length / 4) * 4,
    "="
  );
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) {
    return false;
  }

  let diff = 0;
  for (let i = 0; i < a.length; i += 1) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
}

function sessionSecret(): string {
  const secret = process.env.SESSION_SECRET ?? process.env.AUTH_SECRET;
  if (secret) {
    return secret;
  }
  if (process.env.NODE_ENV === "production") {
    throw new Error("SESSION_SECRET or AUTH_SECRET must be set");
  }
  return "thiscloud-dev-session-secret";
}

async function signingKey(): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    "raw",
    encoder.encode(sessionSecret()),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign", "verify"]
  );
}

async function sign(value: string): Promise<string> {
  const signature = await crypto.subtle.sign(
    "HMAC",
    await signingKey(),
    encoder.encode(value)
  );
  return base64UrlEncode(signature);
}

export async function createSessionToken(
  claims: SessionClaims
): Promise<string> {
  const payload = base64UrlEncode(JSON.stringify(claims));
  return `${payload}.${await sign(payload)}`;
}

export async function verifySessionToken(
  token: string | undefined
): Promise<SessionClaims | null> {
  if (!token) {
    return null;
  }

  const [payload, signature, extra] = token.split(".");
  if (!payload || !signature || extra !== undefined) {
    return null;
  }

  const expected = await sign(payload);
  if (!timingSafeEqual(signature, expected)) {
    return null;
  }

  try {
    const claims = JSON.parse(base64UrlDecode(payload)) as SessionClaims;
    if (
      !claims.userId ||
      !["admin", "user"].includes(claims.role) ||
      !Number.isFinite(claims.expiry) ||
      !claims.csrfToken ||
      Date.now() > claims.expiry
    ) {
      return null;
    }
    return claims;
  } catch {
    return null;
  }
}

export function isMutation(method: string): boolean {
  return ["POST", "PUT", "PATCH", "DELETE"].includes(method.toUpperCase());
}
