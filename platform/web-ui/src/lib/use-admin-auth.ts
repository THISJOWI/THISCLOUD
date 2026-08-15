"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

export type AdminAuthState = {
  authorized: boolean | null;
  error: string;
};

export function useAdminAuth(redirectTo = "/?redirect=/admin"): AdminAuthState {
  const router = useRouter();
  const [authorized, setAuthorized] = useState<boolean | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    async function checkAuth() {
      try {
        const res = await fetch("/api/auth/me", { cache: "no-store" });
        if (!res.ok) {
          router.push(redirectTo);
          return;
        }
        const user = await res.json();
        if (!user.isAdmin) {
          setError("Access denied: admin privileges required");
          setAuthorized(false);
          return;
        }
        setAuthorized(true);
      } catch {
        router.push(redirectTo);
      }
    }
    checkAuth();
  }, [router, redirectTo]);

  return { authorized, error };
}