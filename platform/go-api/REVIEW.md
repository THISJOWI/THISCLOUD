# Code Review: Security & Robustness Fixes

**Reviewed:** 2026-08-04
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Three fixes were applied to the Go API: (1) `readBody` now enforces a 10MB size limit and properly handles read errors, (2) state store uses atomic writes via temp-file-then-rename, and (3) resource type validation on create. All fixes are functionally correct — the build succeeds, all existing tests pass, and `go vet` is clean. However, I found one new bug introduced by Fix 1 and one robustness gap in Fix 2, plus pre-existing issues worth noting.

**Build:** `go build ./...` — PASS
**Tests:** `go test ./...` — PASS (all 10 tests)
**Race detector:** `go test -race` — PASS
**go vet:** — PASS
**gofmt:** — FAIL (pre-existing formatting drift, not from these fixes)

---

## Fix 1: readBody error handling + 10MB size limit — PASS with WARNING

**Verdict:** The core logic is correct. `io.LimitReader` caps reads at 10MB+1, non-EOF errors are captured, and oversized bodies are rejected. The `+1` trick correctly detects bodies that exceed the limit. The test `TestCreateUnknownTypeRejected` exercises the type validation path.

### WR-01: `strings.TrimSpace(string(buf))` allocates a full copy of the body

**File:** `internal/api/server.go:229`
**Severity:** WARNING

```go
if len(strings.TrimSpace(string(buf))) == 0 {
```

For bodies approaching 10MB, `string(buf)` allocates a ~10MB copy, then `TrimSpace` creates another. Use `bytes.TrimSpace` instead, which operates on the existing slice with zero allocation.

**Fix:**
```go
import "bytes"

// line 229:
if len(bytes.TrimSpace(buf)) == 0 {
```

### WR-02: Missing type validation when type comes from URL path only

**File:** `internal/api/server.go:89-92`
**Severity:** WARNING

The new check `if meta.Type == ""` after `meta.Type = typeFilter` correctly handles the case where the type is empty in both body and path. However, the type from the URL path (`typeFilter`) is never validated against the set of known resource types before being passed to `decode()`. While `decode()` does reject unknown types, the error message is a generic `"unknown resource type: X"` with no HTTP status hint. This is *functionally* correct but the path is slightly confusing to trace. No action needed — the existing behavior is correct.

### IN-01: `apply()` always returns nil, making error check dead code

**File:** `internal/api/server.go:101,150-164`
**Severity:** INFO

```go
// line 101:
if err := s.apply(r, res); err != nil {
```

`apply()` catches backend errors and always returns `nil`. The `err` check in the caller can never be true. This is intentional per the comment, but the function signature is misleading. Consider removing the `error` return or adding a comment that the nil-return is by design.

---

## Fix 2: Atomic state writes (tmp + rename) — PASS with WARNING

**Verdict:** The atomic write pattern is correct. `os.Rename` on the same directory is atomic on POSIX. The temp file prevents partial writes from corrupting the state file.

### WR-03: Stale temp file not cleaned up on rename failure

**File:** `internal/state/store.go:82-86`
**Severity:** WARNING

```go
tmp := s.path + ".tmp"
if err := os.WriteFile(tmp, data, 0o644); err != nil {
    return err
}
return os.Rename(tmp, s.path)
```

If `os.Rename` fails (e.g., permissions, cross-filesystem symlink edge case), the `.tmp` file is orphaned on disk. Add best-effort cleanup:

**Fix:**
```go
tmp := s.path + ".tmp"
if err := os.WriteFile(tmp, data, 0o644); err != nil {
    return err
}
if err := os.Rename(tmp, s.path); err != nil {
    os.Remove(tmp) // best-effort: clean up failed temp file
    return err
}
return nil
```

### IN-02: No cleanup of stale temp files from prior crashes

**File:** `internal/state/store.go`
**Severity:** INFO

If the process crashes between `WriteFile` and `Rename`, a `.tmp` file is left behind. On next startup, `load()` reads the original path (not `.tmp`), so the stale file is harmless but wastes disk space. For a production orchestrator, consider cleaning up `s.path + ".tmp"` in `NewStore()` before `load()`.

---

## Fix 3: Resource type validation on create — PASS

**Verdict:** Correct and complete. Unknown types are rejected by `decode()` with a 400 status. Empty type (neither in body nor URL) returns 400 with a clear error message. Test `TestCreateUnknownTypeRejected` covers this path. No new issues introduced.

---

## Pre-existing Issues (not from these fixes, but discovered during review)

### WR-04: HTTP server has no timeouts configured

**File:** `cmd/api-server/main.go:31`
**Severity:** WARNING

```go
if err := http.ListenAndServe(bind, server.Handler()); err != nil {
```

`http.ListenAndServe` creates a server with zero timeouts. This makes the server vulnerable to slowloris attacks and connection exhaustion. An attacker can open many connections and send headers byte-by-byte, tying up goroutines indefinitely.

**Fix:**
```go
srv := &http.Server{
    Addr:              bind,
    Handler:           server.Handler(),
    ReadHeaderTimeout: 10 * time.Second,
    ReadTimeout:       30 * time.Second,
    WriteTimeout:      30 * time.Second,
    IdleTimeout:       120 * time.Second,
}
if err := srv.ListenAndServe(); err != nil {
    log.Fatal(err)
}
```

### WR-05: `collectionFor` returns empty string for unknown types

**File:** `internal/api/server.go:192-203`
**Severity:** WARNING

```go
default:
    return ""
```

If an unknown resource type somehow reaches `collectionFor` (currently prevented by `decode()` validation), it returns `""`, causing `backend.Create` to POST to the root URL. While unreachable today, this is a latent bug. Return an error or log a warning.

---

## Overall Assessment

| Fix | Verdict | Notes |
|-----|---------|-------|
| readBody error handling + size limit | **PASS** | Correct. One minor allocation issue (WR-01). |
| Atomic state writes | **PASS** | Correct pattern. Missing cleanup on rename failure (WR-03). |
| Resource type validation | **PASS** | Correct and well-tested. |
| **New bugs introduced** | **0** | No correctness regressions. |
| **New warnings introduced** | **1** | WR-01 (allocation in empty-body check). |
| **Pre-existing issues** | **2** | WR-04 (no server timeouts), WR-05 (empty collectionFor). |

The fixes are solid. The warnings are minor robustness improvements, not correctness issues.
