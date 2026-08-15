import { NextRequest, NextResponse } from "next/server";

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const base = `${url.protocol}//${url.host}`;
  const response = NextResponse.redirect(new URL("/login", base));

  response.cookies.set("session", "", { maxAge: 0, path: "/" });
  response.cookies.set("csrf-token", "", { maxAge: 0, path: "/" });

  return response;
}
