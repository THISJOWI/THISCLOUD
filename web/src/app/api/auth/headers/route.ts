import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { verifySessionToken } from "@/lib/session";

export async function GET() {
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;

  if (!session) {
    return NextResponse.json({});
  }

  const claims = await verifySessionToken(session);
  if (!claims) {
    return NextResponse.json({}, { status: 401 });
  }

  // Forward the session as an Authorization header to the backend
  // In production, this would be a JWT or opaque token
  return NextResponse.json({
    Authorization: `Bearer ${session}`,
  });
}
