use secret_toolkit::utils::InitCallback;
use serde::{Deserialize, Serialize};

use cosmwasm_std::HumanAddr;

use crate::{msg::ContractInfo, state::BLOCK_SIZE};

/// Instantiation message
#[derive(Serialize, Deserialize)]
pub struct OffspringInitMsg {
    /// factory contract code hash and address
    pub factory: ContractInfo,
    /// label used when initializing offspring
    pub label: String,
    /// String password for the offspring
    pub password: [u8; 32],

    pub owner: HumanAddr,
    pub count: i32,
    #[serde(default)]
    pub description: Option<String>,
}

impl InitCallback for OffspringInitMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}
