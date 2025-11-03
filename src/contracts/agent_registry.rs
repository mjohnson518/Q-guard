use ethers::prelude::*;

// ERC-8004 Agent Registry ABI
abigen!(
    AgentRegistry,
    r#"[
        function getReputation(address agent) view returns (uint256)
        function isRegistered(address agent) view returns (bool)
        function getAgentMetadata(address agent) view returns (string)
    ]"#
);

// Placeholder - will be replaced when contract is deployed
pub const REGISTRY_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

