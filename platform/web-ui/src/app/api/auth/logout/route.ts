import { NextResponse } from "next/server";

export async function GET() {
  const base = process.env.NEXT_PUBLIC_BASE_URL ?? "http://localhost:3000";
  const response = NextResponse.redirect(new URL("/login", base));

  response.cookies.set("session", "", { maxAge: 0, path: "/" });
  response.cookies.set("csrf-token", "", { maxAge: 0, path: "/" });

  return response;
}
