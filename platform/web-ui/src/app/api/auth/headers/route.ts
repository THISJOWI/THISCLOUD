import { NextResponse } from "next/server";
import { cookies } from "next/headers";

export async function GET() {
  const cookieStore = await cookies();
  const session = cookieStore.get("session")?.value;

  if (!session) {
    return NextResponse.json({});
  }

  // Forward the session as an Authorization header to the backend
  // In production, this would be a JWT or opaque token
  return NextResponse.json({
    Authorization: `Bearer ${session}`,
  });
}
