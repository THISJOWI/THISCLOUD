use serde::{Deserialize, Deserializer};

/// Local node identity for the self-registering agent.
///
/// `init`/`join` write this section. A node with `masters` configured registers
/// with the master(s) and POSTs heartbeats to them; a master / single node
/// refreshes its own seeded entry in the shared store.
#[derive(Debug, Clone, Deserialize)]
pub struct NodeIdentityConfig {
    /// Local node id (assigned by the master on `join`). When unset the daemon
    /// seeds an entry derived from its role: `master` → hostname, otherwise
    /// the legacy `master-1`.
    #[serde(default)]
    pub id: Option<String>,
    /// Local role: `master` or `worker`. Used for self-registration when no id
    /// is assigned yet.
    #[serde(default)]
    pub role: Option<String>,
    /// Master API base URLs (e.g. `http://192.168.1.12:8080`). When set the
    /// daemon registers + heartbeats against the first reachable master;
    /// otherwise the local store is updated in place. Accepts either a TOML
    /// array or the legacy single `master = "url"` string.
    #[serde(default, alias = "master", deserialize_with = "de_masters")]
    pub masters: Vec<String>,
    /// Seconds between self-heartbeats. Must be below the node TTL (30s).
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
}

/// Accept `masters = [...]` (list) or legacy `master = "url"` (single string).
fn de_masters<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value {
        toml::Value::String(s) => Ok(vec![s]),
        toml::Value::Array(items) => items
            .into_iter()
            .map(toml::Value::try_into::<String>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(
            "expected a string or array of strings for masters",
        )),
    }
}

fn default_heartbeat_interval() -> u64 {
    10
}

impl Default for NodeIdentityConfig {
    fn default() -> Self {
        Self {
            id: None,
            role: None,
            masters: Vec::new(),
            heartbeat_interval_secs: default_heartbeat_interval(),
        }
    }
}