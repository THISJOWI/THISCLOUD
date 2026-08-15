use serde::Deserialize;

/// Local node identity for the self-heartbeat loop.
///
/// `init`/`join` write this section. On a master the daemon heartbeats its own
/// seeded node entry (`master-1`); on a worker it POSTs heartbeats to the
/// master's `/api/v1/nodes/{id}/heartbeat` endpoint so the master's view of
/// the worker stays online.
#[derive(Debug, Clone, Deserialize)]
pub struct NodeIdentityConfig {
    /// Local node id (assigned by the master on `join`). When unset the daemon
    /// heartbeats the seeded `master-1` entry.
    #[serde(default)]
    pub id: Option<String>,
    /// Master API base URL (e.g. `http://192.168.1.12:8080`). When set,
    /// heartbeats are POSTed to the master; otherwise the local store is
    /// updated in place.
    #[serde(default)]
    pub master: Option<String>,
    /// Seconds between self-heartbeats. Must be below the node TTL (30s).
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
}

fn default_heartbeat_interval() -> u64 {
    10
}

impl Default for NodeIdentityConfig {
    fn default() -> Self {
        Self {
            id: None,
            master: None,
            heartbeat_interval_secs: default_heartbeat_interval(),
        }
    }
}