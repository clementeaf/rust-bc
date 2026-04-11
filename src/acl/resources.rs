//! Standard ACL resource identifiers, mirroring Hyperledger Fabric's resource names.

/// Well-known ACL resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AclResource {
    #[allow(dead_code)]
    ChaincodeInvoke,
    #[allow(dead_code)]
    ChaincodeQuery,
    #[allow(dead_code)]
    BlockEvents,
    #[allow(dead_code)]
    ChannelConfig,
    #[allow(dead_code)]
    PeerDiscovery,
    #[allow(dead_code)]
    PrivateDataRead,
    #[allow(dead_code)]
    PrivateDataWrite,
    #[allow(dead_code)]
    Custom(String),
}

impl AclResource {
    #[allow(dead_code)]
    /// Return the canonical string used as the ACL map key.
    pub fn resource_name(&self) -> &str {
        match self {
            Self::ChaincodeInvoke => "peer/ChaincodeToChaincode",
            Self::ChaincodeQuery => "peer/ChaincodeQuery",
            Self::BlockEvents => "peer/BlockEvents",
            Self::ChannelConfig => "peer/ChannelConfig",
            Self::PeerDiscovery => "peer/Discovery",
            Self::PrivateDataRead => "peer/PrivateData.Read",
            Self::PrivateDataWrite => "peer/PrivateData.Write",
            Self::Custom(name) => name.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_names() {
        assert_eq!(
            AclResource::ChaincodeInvoke.resource_name(),
            "peer/ChaincodeToChaincode"
        );
        assert_eq!(
            AclResource::ChaincodeQuery.resource_name(),
            "peer/ChaincodeQuery"
        );
        assert_eq!(AclResource::BlockEvents.resource_name(), "peer/BlockEvents");
        assert_eq!(
            AclResource::ChannelConfig.resource_name(),
            "peer/ChannelConfig"
        );
        assert_eq!(AclResource::PeerDiscovery.resource_name(), "peer/Discovery");
        assert_eq!(
            AclResource::PrivateDataRead.resource_name(),
            "peer/PrivateData.Read"
        );
        assert_eq!(
            AclResource::PrivateDataWrite.resource_name(),
            "peer/PrivateData.Write"
        );
        assert_eq!(
            AclResource::Custom("my/Resource".to_string()).resource_name(),
            "my/Resource"
        );
    }
}
