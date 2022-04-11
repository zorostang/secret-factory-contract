use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// factory contract code hash and address
    pub factory: ContractInfo,
    /// index within the factory
    pub index: u32,
    /// label used when initializing offspring
    pub label: String,
    /// password to be used by factory
    pub password: [u8; 32],
    /// Optional text description of this offspring
    pub description: Option<String>,

    
    pub owner: HumanAddr,
    pub count: i32,
}

/// Handle messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Increment {},
    Reset { count: i32 },
    // Deactivate can only be called by owner in this template
    Deactivate {},
}

/// Queries
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number. Can only be queried by the owner,
    // to demonstrate how to use the viewing key in the factory.
    GetCount {
        /// address to authenticate as a viewer
        address: HumanAddr,
        /// viewer's viewing key
        viewing_key: String,
    },
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, JsonSchema, Clone, PartialEq, Debug)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: HumanAddr,
}

/// responses to queries
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    CountResponse {
        count: i32,
    }
}