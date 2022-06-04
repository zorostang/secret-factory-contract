use std::any::type_name;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};

use secret_toolkit::serialization::{Bincode2, Serde};

use crate::msg::OffspringContractInfo;

/// prefix for storage of owners' inactive offspring
pub const PREFIX_OWNERS_INACTIVE: &[u8] = b"ownersinactive";
/// prefix for storage of owners' active offspring
pub const PREFIX_OWNERS_ACTIVE: &[u8] = b"ownersactive";
/// prefix for storage of an active offspring info
pub const PREFIX_ACTIVE_INFO: &[u8] = b"activeinfo";
/// prefix for storage of a inactive offspring info
pub const PREFIX_INACTIVE_INFO: &[u8] = b"inactiveinfo";
/// prefix for viewing keys
pub const PREFIX_VIEW_KEY: &[u8] = b"viewingkey";
/// storage key for prng seed
pub const PRNG_SEED_KEY: &[u8] = b"prngseed";
/// storage key for the factory config
pub const CONFIG_KEY: &[u8] = b"config";
/// storage key for the active offspring list
pub const ACTIVE_KEY: &[u8] = b"active";
/// storage key for the password of the offspring we just instantiated
pub const PENDING_KEY: &[u8] = b"pending";
/// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
/// response size
pub const BLOCK_SIZE: usize = 256;

/// grouping the data primarily used when creating a new offspring
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// code hash and address of the offspring contract
    pub version: OffspringContractInfo,
    /// unique id to give created offspring
    pub index: u32,
    /// factory's create offspring status
    pub stopped: bool,
    /// address of the factory admin
    pub admin: CanonicalAddr,
}

/// Returns StdResult<()> resulting from saving an item to storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

/// Removes an item from storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn remove<S: Storage>(storage: &mut S, key: &[u8]) {
    storage.remove(key);
}

/// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
/// StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

/// Returns StdResult<Option<T>> from retrieving the item with the specified key.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Bincode2::deserialize(&value).map(Some),
        None => Ok(None),
    }
}
