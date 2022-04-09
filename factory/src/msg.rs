use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr};

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
        /// offspring index
        index: u32,
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
    },
    /// lists all active offspring
    ListActiveOffspring {},
    /// lists inactive offsspring in reverse chronological order.  If you specify page size, it returns
    /// only that number of offspring (default is 200). If you specify the before parameter, it will
    /// start listing from the first offspring whose index is less than "before".  If you are
    /// paginating, you would take the index of the last offspring you receive, and specify that as the
    /// before parameter on your next query so it will continue where it left off
    ListInactiveOffspring {
        /// optionally only show offspring with index less than specified value
        #[serde(default)]
        before: Option<u32>,
        /// optional number of offspring to return
        #[serde(default)]
        page_size: Option<u32>,
    },
    /// authenticates the supplied address/viewing key.  This should only be called by offspring.
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
        active: Option<Vec<OffspringInfo>>,
        /// lists of the address' inactive offspring
        #[serde(skip_serializing_if = "Option::is_none")]
        inactive: Option<Vec<InactiveOffspringInfo>>,
    },
    /// List active offspring sorted by pair
    ListActiveOffspring {
        /// active offspring sorted by pair
        #[serde(skip_serializing_if = "Option::is_none")]
        active: Option<Vec<OffspringInfo>>,
    },
    /// List inactive offspring in reverse chronological order
    ListInactiveOffspring {
        /// inactive offspring in reverse chronological order
        #[serde(skip_serializing_if = "Option::is_none")]
        inactive: Option<Vec<InactiveOffspringInfo>>,
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

/// active offspring display info
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct OffspringInfo {
    /// offspring address
    pub address: HumanAddr,
    /// offspring password
    pub password: [u8; 32],
}

/// active offspring info for storage
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct RegisterOffspringInfo {
    /// index with the factory
    pub index: u32,
    /// offspring password
    pub password: [u8; 32],
}

impl RegisterOffspringInfo {
    /// takes the register offspring information and creates a store offspring info struct
    pub fn to_store_offspring_info(&self, address: CanonicalAddr) -> StoreOffspringInfo {
        StoreOffspringInfo {
            address,
            password: self.password.clone(),
        }
    }
}

/// active offspring info for storage
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct StoreOffspringInfo {
    /// offspring address
    pub address: CanonicalAddr,
    /// offspring password
    pub password: [u8; 32],
}

impl StoreOffspringInfo {
    /// takes the active offspring information and creates a inactive offspring info struct
    pub fn to_store_inactive_offspring_info(
        &self,
    ) -> StoreInactiveOffspringInfo {
        StoreInactiveOffspringInfo {
            address: self.address.clone(),
            password: self.password.clone(),
        }
    }
}

// in general, when an offspring contract is deactivated, it may require
// extra data to be stored with it, and thus, in theory InactiveOffspringInfo
// could be different to OffspringInfo. That's why they are two different structs.

/// inactive offspring display info
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InactiveOffspringInfo {
    /// index in inactive offspring list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    /// offspring address
    pub address: HumanAddr,
    /// offspring password
    pub password: [u8; 32],
}

/// inactive offspring storage format
#[derive(Serialize, Deserialize)]
pub struct StoreInactiveOffspringInfo {
    /// offspring address
    pub address: CanonicalAddr,
    /// offspring password
    pub password: [u8; 32],
}
