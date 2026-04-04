pub mod service;

use std::str::FromStr;

/// Role of this node in the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeRole {
    Peer,
    Orderer,
    PeerAndOrderer,
}

impl FromStr for NodeRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "peer" => Ok(NodeRole::Peer),
            "orderer" => Ok(NodeRole::Orderer),
            "" | "peerandorderer" => Ok(NodeRole::PeerAndOrderer),
            other => Err(format!("unknown node role: {other}")),
        }
    }
}

impl NodeRole {
    /// Read from the `NODE_ROLE` environment variable; defaults to `PeerAndOrderer`.
    pub fn from_env() -> Self {
        std::env::var("NODE_ROLE")
            .unwrap_or_default()
            .to_lowercase()
            .parse()
            .unwrap_or(NodeRole::PeerAndOrderer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_peer() {
        assert_eq!("peer".parse::<NodeRole>().unwrap(), NodeRole::Peer);
    }

    #[test]
    fn parse_orderer() {
        assert_eq!("orderer".parse::<NodeRole>().unwrap(), NodeRole::Orderer);
    }

    #[test]
    fn parse_empty_defaults_to_peer_and_orderer() {
        assert_eq!("".parse::<NodeRole>().unwrap(), NodeRole::PeerAndOrderer);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!("invalid".parse::<NodeRole>().is_err());
    }
}
