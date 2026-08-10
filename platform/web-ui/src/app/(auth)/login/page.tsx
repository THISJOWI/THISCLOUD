"use client";

import { Suspense, useState } from "react";
import { useSearchParams } from "next/navigation";
import styles from "./login.module.css";

function LoginForm() {
  const searchParams = useSearchParams();
  const redirect = searchParams.get("redirect") ?? "/";
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [capsLock, setCapsLock] = useState(false);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "CapsLock") {
      setCapsLock(e.getModifierState("CapsLock"));
    }
  }

  async function handleLogin(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError("");

    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });

      if (!res.ok) {
        const data = await res.json();
        setError(data.error ?? "Login failed");
        setLoading(false);
        return;
      }

      // Full page reload to ensure cookie is sent with the next request
      window.location.href = redirect;
    } catch {
      setError("Connection failed");
      setLoading(false);
    }
  }

  return (
    <form className={styles.form} onSubmit={handleLogin}>
      {error && <div className={styles.error}>{error}</div>}

      <div className={styles.field}>
        <label htmlFor="username">Username</label>
        <div className={styles.inputWrap}>
          <input
            id="username"
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            autoComplete="username"
            autoFocus
            placeholder="server username"
          />
        </div>
      </div>

      <div className={styles.field}>
        <label htmlFor="password">Password</label>
        <div className={styles.inputWrap}>
          <input
            id="password"
            type={showPassword ? "text" : "password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={handleKeyDown}
            required
            autoComplete="current-password"
            placeholder="server password"
          />
          <button
            type="button"
            className={styles.pwToggle}
            onClick={() => setShowPassword(!showPassword)}
            tabIndex={-1}
            aria-label={showPassword ? "Hide password" : "Show password"}
          >
            {showPassword ? "◉" : "○"}
          </button>
        </div>
        {capsLock && (
          <span className={styles.capsLock}>Caps Lock is on</span>
        )}
      </div>

      <button type="submit" disabled={loading} className={styles.submitBtn}>
        <span className={styles.btnInner}>
          {loading && <span className={styles.spinner} />}
          {loading ? "Signing in..." : "Sign In"}
        </span>
      </button>
    </form>
  );
}

export default function LoginPage() {
  return (
    <div className={styles.loginPage}>
      <div className={styles.loginCard}>
        <div className={styles.brand}>
          <div className={styles.brandIcon}>T</div>
          <div className={styles.brandName}>THISCLOUD</div>
          <div className={styles.brandSub}>Hypervisor OS</div>
        </div>
        <Suspense
          fallback={
            <div className={styles.form} style={{ alignItems: "center" }}>
              <span
                className={styles.spinner}
                style={{ width: 20, height: 20 }}
              />
            </div>
          }
        >
          <LoginForm />
        </Suspense>
        <div className={styles.footer}>
          Use your server credentials to sign in
        </div>
      </div>
    </div>
  );
}
