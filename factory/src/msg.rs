use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr};

/// Instantiation message
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    /// entropy used to generate prng seed
    pub entropy: String,
    /// offspring contract info
    pub offspring_contract: OffspringContractInfo,
}

/// Handle messages
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// CreateOffspring will instantiate a new offspring contract
    CreateOffspring {
        /// String used to label when instantiating offspring contract.
        label: String,
        /// Used to generate the password for the offspring contract
        entropy: String,
        //  the rest are meant to be contract specific data
        /// address of the owner associated to this offspring contract
        owner: HumanAddr,
        /// the count for the counter offspring template
        count: i32,
        #[serde(default)]
        description: Option<String>,
    },

    /// RegisterOffspring saves the offspring info of a newly instantiated contract and adds it to the list
    /// of active offspring contracts as well
    ///
    /// Only offspring will use this function
    RegisterOffspring {
        /// owner of the offspring
        owner: HumanAddr,
        /// offspring information needed by the factory
        offspring: RegisterOffspringInfo,
    },

    /// DeactivateOffspring tells the factory that the offspring is inactive.
    DeactivateOffspring {
        /// offspring's owner
        owner: HumanAddr,
    },

    /// Allows the admin to add a new offspring contract version
    NewOffspringContract {
        offspring_contract: OffspringContractInfo,
    },

    /// Create a viewing key to be used with all factory and offspring authenticated queries
    CreateViewingKey { entropy: String },

    /// Set a viewing key to be used with all factory and offspring authenticated queries
    SetViewingKey {
        key: String,
        // optional padding can be used so message length doesn't betray key length
        padding: Option<String>,
    },

    /// Allows an admin to start/stop all offspring creation
    SetStatus { stop: bool },
}

/// Queries
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// lists all offspring whose owner is the given address.
    ListMyOffspring {
        // address whose activity to display
        address: HumanAddr,
        /// viewing key
        viewing_key: String,
        /// optional filter for only active or inactive offspring.  If not specified, lists all
        #[serde(default)]
        filter: Option<FilterTypes>,
        /// start page for the offsprings returned and listed (applies to both active and inactive). Default: 0
        #[serde(default)]
        start_page: Option<u32>,
        /// optional number of offspring to return in this page (applies to both active and inactive). Default: DEFAULT_PAGE_SIZE
        #[serde(default)]
        page_size: Option<u32>,
    },
    /// lists all active offspring
    ListActiveOffspring {
        /// start page for the offsprings returned and listed. Default: 0
        #[serde(default)]
        start_page: Option<u32>,
        /// optional number of offspring to return in this page. Default: DEFAULT_PAGE_SIZE
        #[serde(default)]
        page_size: Option<u32>,
    },
    /// lists inactive offspring in reverse chronological order.
    ListInactiveOffspring {
        /// start page for the offsprings returned and listed. Default: 0
        #[serde(default)]
        start_page: Option<u32>,
        /// optional number of offspring to return in this page. Default: DEFAULT_PAGE_SIZE
        #[serde(default)]
        page_size: Option<u32>,
    },
    /// authenticates the supplied address/viewing key. This should be called by offspring.
    IsKeyValid {
        /// address whose viewing key is being authenticated
        address: HumanAddr,
        /// viewing key
        viewing_key: String,
    },
}

/// the filter types when viewing an address' offspring
#[derive(Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FilterTypes {
    Active,
    Inactive,
    All,
}

/// responses to queries
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// List the offspring where address is associated.
    ListMyOffspring {
        /// lists of the address' active offspring
        #[serde(skip_serializing_if = "Option::is_none")]
        active: Option<Vec<StoreOffspringInfo>>,
        /// lists of the address' inactive offspring
        #[serde(skip_serializing_if = "Option::is_none")]
        inactive: Option<Vec<StoreInactiveOffspringInfo>>,
    },
    /// List active offspring sorted by pair
    ListActiveOffspring {
        /// active offspring sorted by pair
        active: Vec<StoreOffspringInfo>,
    },
    /// List inactive offspring in reverse chronological order
    ListInactiveOffspring {
        /// inactive offspring in reverse chronological order
        inactive: Vec<StoreInactiveOffspringInfo>,
    },
    /// Viewing Key Error
    ViewingKeyError { error: String },
    /// result of authenticating address/key pair
    IsKeyValid { is_valid: bool },
}

/// success or failure response
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ResponseStatus {
    Success,
    Failure,
}

/// Responses from handle functions
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    /// response from creating a viewing key
    ViewingKey { key: String },
    /// generic status response
    Status {
        /// success or failure
        status: ResponseStatus,
        /// execution description
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: HumanAddr,
}

/// Info needed to instantiate an offspring
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct OffspringContractInfo {
    /// code id of the stored offspring contract
    pub code_id: u64,
    /// code hash of the stored offspring contract
    pub code_hash: String,
}

/// active offspring info
#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct OffspringInfo {
    /// offspring address
    pub address: HumanAddr,
    /// label used when initializing offspring
    pub label: String,
}

/// active offspring info for storage/display
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct RegisterOffspringInfo {
    /// label used when initializing offspring
    pub label: String,
    /// offspring password
    pub password: [u8; 32],
}

impl RegisterOffspringInfo {
    /// takes the register offspring information and creates a store offspring info struct
    pub fn to_store_offspring_info(&self, address: HumanAddr) -> StoreOffspringInfo {
        StoreOffspringInfo {
            address,
            label: self.label.clone(),
        }
    }
}

// In general, data that is stored for user display may be different from the data used
// for internal functions of the smart contract. That is why we have StoreOffspringInfo.

/// active offspring info for storage/display
#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
pub struct StoreOffspringInfo {
    /// offspring address
    pub address: HumanAddr,
    /// label used when initializing offspring
    pub label: String,
}

impl StoreOffspringInfo {
    /// takes the active offspring information and creates a inactive offspring info struct
    pub fn to_store_inactive_offspring_info(
        &self,
    ) -> StoreInactiveOffspringInfo {
        StoreInactiveOffspringInfo {
            address: self.address.clone(),
            label: self.label.clone(),
        }
    }
}

// in general, when an offspring contract is deactivated, it may require
// different data to be stored with it, and thus, in theory InactiveOffspringInfo
// could be different to OffspringInfo. That's why we have InactiveOffspringInfo.

/// inactive offspring info
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct InactiveOffspringInfo {
    /// label used when initializing offspring
    pub label: String,
    /// offspring address
    pub address: HumanAddr,
}

/// inactive offspring storage/display format
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct StoreInactiveOffspringInfo {
    /// offspring address
    pub address: HumanAddr,
    /// label used when initializing offspring
    pub label: String,
}
