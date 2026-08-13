//! T5.1 metrics module test harness.
//!
//! The `metrics` module is not yet wired into `src/lib.rs` (central wiring —
//! `lib.rs` / `config/mod.rs` / `daemon.rs` — is handled by a parallel agent).
//! To keep this target green in the intermediate state, this harness compiles
//! the module sources directly via `#[path]` includes and re-exports the
//! lib's `auth`/`core` modules so the canonical `crate::auth::...` and
//! `crate::core::...` paths used inside the sources resolve unchanged.
//!
//! Once `src/lib.rs` declares `pub mod metrics`, the same source files compile
//! inside the lib crate (all paths resolve there too), and this harness keeps
//! working against the re-exported `auth`/`core`.

pub use thiscloudd::auth;
pub use thiscloudd::core;

// Module sources, compiled directly. `#[path]` at the top level resolves
// relative to this file's directory (tests/), same as tests/core.rs.
#[path = "../src/metrics/model.rs"]
pub mod model;
#[path = "../src/metrics/registry.rs"]
pub mod registry;
#[path = "../src/metrics/module.rs"]
pub mod module;
#[path = "../src/metrics/http.rs"]
pub mod http;

#[path = "core/test_metrics_model.rs"]
mod test_metrics_model;

#[path = "core/test_metrics_registry.rs"]
mod test_metrics_registry;

#[path = "core/test_metrics_http.rs"]
mod test_metrics_http;
