use cosmwasm_std::HumanAddr;
use serde::{Deserialize, Serialize};

use secret_toolkit::utils::{HandleCallback, Query};

use crate::state::BLOCK_SIZE;

/// Factory handle messages to be used by offspring.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryHandleMsg {
    /// RegisterOffspring saves the offspring info of a newly instantiated contract and adds it to the list
    /// of active offspring contracts as well
    ///
    /// Only offspring will use this function
    RegisterOffspring {
        /// owner of the offspring
        owner: HumanAddr,
        /// offspring information needed by the factory
        offspring: FactoryOffspringInfo,
    },

    /// DeactivateOffspring tells the factory that the offspring is inactive.
    DeactivateOffspring {
        /// offspring index
        index: u32,
        /// offspring's owner
        owner: HumanAddr,
    },
}

impl HandleCallback for FactoryHandleMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/// the factory's query messages this offspring will call
#[derive(Serialize)]
pub struct FactoryOffspringInfo {
    /// index with the factory
    pub index: u32,
    /// offspring password
    pub password: [u8; 32],
}

/// Queries
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryQueryMsg {
    /// authenticates the supplied address/viewing key. This should be called by offspring.
    IsKeyValid {
        /// address whose viewing key is being authenticated
        address: HumanAddr,
        /// viewing key
        viewing_key: String,
    },
}

impl Query for FactoryQueryMsg {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/// result of authenticating address/key pair
#[derive(Serialize, Deserialize, Debug)]
pub struct IsKeyValid {
    pub is_valid: bool,
}

/// IsKeyValid wrapper struct
#[derive(Serialize, Deserialize, Debug)]
pub struct IsKeyValidWrapper {
    pub is_key_valid: IsKeyValid,
}