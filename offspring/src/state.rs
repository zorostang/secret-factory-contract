use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use crate::msg::ContractInfo;

pub static CONFIG_KEY: &[u8] = b"config";

/// State of the offspring contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    /// factory code hash and address
    pub factory: ContractInfo,
    /// index within the factory
    pub index: u32,
    /// this is relevant if the factory is listing offsprings by activity status.
    pub active: bool,
    /// used by factory for authentication
    pub password: [u8; 32],
    /// Optional text description of this offspring
    pub description: Option<String>,
    
    // rest are contract specific data
    /// the count for the counter
    pub count: i32,
    /// address of the owner associated to this offspring contract
    pub owner: CanonicalAddr,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
