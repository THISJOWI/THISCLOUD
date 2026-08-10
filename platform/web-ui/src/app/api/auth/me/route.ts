import { NextResponse } from "next/server";
import { cookies } from "next/headers";

export async function GET() {
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;

  if (!session) {
    return NextResponse.json({ error: "Not authenticated" }, { status: 401 });
  }

  // TODO: Validate session token against your auth system
  // For now, decode a simple session token format: "userId:role:expiry"
  try {
    const parts = Buffer.from(session, "base64").toString("utf-8").split(":");
    const [userId, role, expiry] = parts;

    if (expiry && Date.now() > parseInt(expiry, 10)) {
      return NextResponse.json({ error: "Session expired" }, { status: 401 });
    }

    return NextResponse.json({
      id: userId,
      role,
      isAdmin: role === "admin",
    });
  } catch {
    return NextResponse.json({ error: "Invalid session" }, { status: 401 });
  }
}
