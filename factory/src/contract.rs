use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, ReadonlyStorage, StdError, StdResult, Storage,
};

use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use secret_toolkit::{
    utils::{pad_handle_result, pad_query_result, InitCallback},
};

use secret_toolkit_incubator::{CashMap, ReadOnlyCashMap};

use crate::{rand::sha_256, state::DEFAULT_PAGE_SIZE};
use crate::state::{
    load, may_load, remove, save, Config, ACTIVE_KEY, BLOCK_SIZE, CONFIG_KEY, PENDING_KEY, INACTIVE_KEY, PREFIX_OWNERS_ACTIVE, PREFIX_OWNERS_INACTIVE,
    PREFIX_VIEW_KEY, PRNG_SEED_KEY,
};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use crate::{
    msg::{
        ContractInfo, FilterTypes, HandleAnswer, HandleMsg, InitMsg,
        OffspringContractInfo, QueryAnswer, QueryMsg, RegisterOffspringInfo,
        ResponseStatus::Success, StoreInactiveOffspringInfo, StoreOffspringInfo,
    },
    offspring_msg::OffspringInitMsg,
    rand::Prng,
};

////////////////////////////////////// Init ///////////////////////////////////////
/// Returns InitResult
///
/// Initializes the factory and creates a prng from the entropy String
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - InitMsg passed in with the instantiation message
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy).as_bytes()).to_vec();

    let config = Config {
        version: msg.offspring_contract,
        stopped: false,
        admin: deps.api.canonical_address(&env.message.sender)?,
    };

    save(&mut deps.storage, CONFIG_KEY, &config)?;
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;

    Ok(InitResponse::default())
}

///////////////////////////////////// Handle //////////////////////////////////////
/// Returns HandleResult
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `msg` - HandleMsg passed in with the execute message
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    let response = match msg {
        HandleMsg::CreateOffspring {
            label,
            entropy,
            owner,
            count,
            description,
        } => try_create_offspring(deps, env, label, entropy, owner, count, description),
        HandleMsg::RegisterOffspring { owner, offspring } => {
            try_register_offspring(deps, env, owner, &offspring)
        }
        HandleMsg::DeactivateOffspring { owner } => {
            try_deactivate_offspring(deps, env, &owner)
        }
        HandleMsg::CreateViewingKey { entropy } => try_create_key(deps, env, &entropy),
        HandleMsg::SetViewingKey { key, .. } => try_set_key(deps, env, &key),
        HandleMsg::NewOffspringContract { offspring_contract } => {
            try_new_contract(deps, env, offspring_contract)
        }
        HandleMsg::SetStatus { stop } => try_set_status(deps, env, stop),
    };
    pad_handle_result(response, BLOCK_SIZE)
}

/// Returns [u8;32]
///
/// generates new entropy from block data, does not save it to the contract.
///
/// # Arguments
///
/// * `env` - Env of contract's environment
/// * `seed` - (user generated) seed for rng
/// * `entropy` - Entropy seed saved in the contract
pub fn new_entropy(env: &Env, seed: &[u8], entropy: &[u8]) -> [u8; 32] {
    // 16 here represents the lengths in bytes of the block height and time.
    let entropy_len = 16 + env.message.sender.len() + entropy.len();
    let mut rng_entropy = Vec::with_capacity(entropy_len);
    rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
    rng_entropy.extend_from_slice(&env.message.sender.0.as_bytes());
    rng_entropy.extend_from_slice(entropy);

    let mut rng = Prng::new(seed, &rng_entropy);

    rng.rand_bytes()
}

/// Returns HandleResult
///
/// create a new offspring
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `password` - String containing the password to give the offspring
/// * `owner` - address of the owner associated to this offspring contract
/// * `count` - the count for the counter template
/// * `description` - optional free-form text string owner may have used to describe the offspring
#[allow(clippy::too_many_arguments)]
fn try_create_offspring<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    label: String,
    entropy: String,
    owner: HumanAddr,
    count: i32,
    description: Option<String>,
) -> HandleResult {
    let config: Config = load(&deps.storage, CONFIG_KEY)?;
    if config.stopped {
        return Err(StdError::generic_err(
            "The factory has been stopped. No new offspring can be created",
        ));
    }

    let factory = ContractInfo {
        code_hash: env.clone().contract_code_hash,
        address: env.clone().contract.address,
    };

    // generate and save new prng, and password. (we only register an offspring retuning the matching password)
    let prng_seed: Vec<u8> = load(&deps.storage, PRNG_SEED_KEY)?;
    let new_prng_bytes = new_entropy(&env, prng_seed.as_ref(), entropy.as_bytes());
    save(&mut deps.storage, PRNG_SEED_KEY, &new_prng_bytes.to_vec())?;

    // store the password for future authentication
    let password = sha_256(&new_prng_bytes);
    save(&mut deps.storage, PENDING_KEY, &password)?;

    let initmsg = OffspringInitMsg {
        factory,
        label: label.clone(),
        password: password.clone(),
        owner,
        count,
        description,
    };

    let cosmosmsg = initmsg.to_cosmos_msg(
        label,
        config.version.code_id,
        config.version.code_hash,
        None,
    )?;

    Ok(HandleResponse {
        messages: vec![cosmosmsg],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Status {
            status: Success,
            message: None,
        })?),
    })
}

/// Returns HandleResult
///
/// Registers the calling offspring by saving its info and adding it to the appropriate lists
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `owner` - reference to the address of the offspring's owner
/// * `reg_offspring` - reference to RegisterOffspringInfo of the offspring that is trying to register
fn try_register_offspring<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    reg_offspring: &RegisterOffspringInfo,
) -> HandleResult {
    // verify this is the offspring we are waiting for
    let load_password: Option<[u8; 32]> = may_load(&deps.storage, PENDING_KEY)?;
    let auth_password = load_password
        .ok_or_else(|| StdError::generic_err("Unable to authenticate registration."))?;
    if auth_password != reg_offspring.password {
        return Err(StdError::generic_err(
            "password does not match the offspring we are creating",
        ));
    }
    remove(&mut deps.storage, PENDING_KEY);

    // convert register offspring info to storage format
    let offspring_addr = deps.api.canonical_address(&env.message.sender)?;
    let offspring = reg_offspring.to_store_offspring_info(env.message.sender.clone());

    // save the offspring info
    let mut info_store = CashMap::init(ACTIVE_KEY, &mut deps.storage);
    info_store.insert(offspring_addr.as_slice(), offspring.clone())?;

    // get list of owner's active offspring
    let mut owners_store = PrefixedStorage::new(PREFIX_OWNERS_ACTIVE, &mut deps.storage);
    let mut my_active_store: CashMap<StoreOffspringInfo, _, _> = CashMap::init(owner.to_string().as_bytes(), &mut owners_store);
    // add this offspring to owner's list
    my_active_store.insert(offspring_addr.as_slice(), offspring)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("offspring_address", env.message.sender)],
        data: None,
    })
}

/// Returns HandleResult
///
/// deactivates the offspring by saving its info and adding/removing it to/from the
/// appropriate lists
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `owner` - offspring's owner
fn try_deactivate_offspring<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> HandleResult {

    let offspring_addr = &deps.api.canonical_address(&env.message.sender)?;

    // verify offspring is in active list, and not a spam attempt
    let may_info = authenticate_offspring(&deps.storage, offspring_addr)?;
    // delete the active offspring info
    let mut info_store: CashMap<StoreOffspringInfo, _, _> = CashMap::init(ACTIVE_KEY, &mut deps.storage);
    info_store.remove(offspring_addr.as_slice())?;

    // save owner's inactive offspring info
    let offspring_info = may_info;
    let inactive_info = offspring_info.to_store_inactive_offspring_info();
    let mut owners_inactive_store = PrefixedStorage::new(PREFIX_OWNERS_INACTIVE, &mut deps.storage);
    let mut inactive_store = CashMap::init(owner.to_string().as_bytes(), &mut owners_inactive_store);
    inactive_store.insert(offspring_addr.as_slice(), inactive_info.clone())?;

    // save inactive offspring info
    let mut inactive_store = CashMap::init(INACTIVE_KEY, &mut deps.storage);
    inactive_store.insert(offspring_addr.as_slice(), inactive_info)?;

    // remove offspring from owner's active list
    remove_from_persons_active(&mut deps.storage, PREFIX_OWNERS_ACTIVE, owner, offspring_addr)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

/// Returns StdResult<(StoreOffspringInfo)>
///
/// verifies that the offspring is in the active list, and returns the active offspring info
///
/// # Arguments
///
/// * `storage` - a reference to contract's storage
/// * `offspring` - a reference to the offspring's address
fn authenticate_offspring<S: ReadonlyStorage>(
    storage: &S,
    offspring: &CanonicalAddr,
) -> StdResult<StoreOffspringInfo> {
    let info_store: ReadOnlyCashMap<StoreOffspringInfo, _, _> = ReadOnlyCashMap::init(ACTIVE_KEY, storage);

    let info = info_store.get(offspring.as_slice());

    if let Some(offspring_info) = info {
        Ok(offspring_info)
    } else {
        return Err(StdError::generic_err(
            "This is not an active offspring registered with factory.",
        ));
    }
}

/// Returns HandleResult
///
/// allows admin to edit the offspring contract version.
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `offspring_contract` - OffspringContractInfo of the new offspring version
fn try_new_contract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    offspring_contract: OffspringContractInfo,
) -> HandleResult {
    // only allow admin to do this
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if config.admin != sender {
        return Err(StdError::generic_err(
            "This is an admin command. Admin commands can only be run from admin address",
        ));
    }
    config.version = offspring_contract;
    save(&mut deps.storage, CONFIG_KEY, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Status {
            status: Success,
            message: None,
        })?),
    })
}

/// Returns HandleResult
///
/// allows admin to change the factory status to (dis)allow the creation of new offspring
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `stop` - true if the factory should disallow offspring creation
fn try_set_status<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    stop: bool,
) -> HandleResult {
    // only allow admin to do this
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if config.admin != sender {
        return Err(StdError::generic_err(
            "This is an admin command. Admin commands can only be run from admin address",
        ));
    }
    config.stopped = stop;
    save(&mut deps.storage, CONFIG_KEY, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Status {
            status: Success,
            message: None,
        })?),
    })
}

/// Returns HandleResult
///
/// create a viewing key
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `entropy` - string slice to be used as an entropy source for randomization
fn try_create_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: &str,
) -> HandleResult {
    // create and store the key
    let prng_seed: Vec<u8> = load(&deps.storage, PRNG_SEED_KEY)?;
    let key = ViewingKey::new(&env, &prng_seed, entropy.as_ref());
    let message_sender = &deps.api.canonical_address(&env.message.sender)?;
    let mut key_store = PrefixedStorage::new(PREFIX_VIEW_KEY, &mut deps.storage);
    save(&mut key_store, message_sender.as_slice(), &key.to_hashed())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey {
            key: format!("{}", key),
        })?),
    })
}

/// Returns HandleResult
///
/// sets the viewing key
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `key` - string slice to be used as the viewing key
fn try_set_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    key: &str,
) -> HandleResult {
    // store the viewing key
    let vk = ViewingKey(key.to_string());
    let message_sender = &deps.api.canonical_address(&env.message.sender)?;
    let mut key_store = PrefixedStorage::new(PREFIX_VIEW_KEY, &mut deps.storage);
    save(&mut key_store, message_sender.as_slice(), &vk.to_hashed())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey {
            key: key.to_string(),
        })?),
    })
}

/// Returns StdResult<()>
///
/// remove an offspring from a person's list of active offspring. (This helper is implemented
/// in case there are multiple users associated to an offspring)
///
/// # Arguments
///
/// * `storage` - mutable reference to contract's storage
/// * `prefix` - prefix to storage of a person's active offspring list
/// * `person` - a reference to the canonical address of the person the list belongs to
/// * `offspring_addr` - a reference to the canonical address of the offspring to remove
fn remove_from_persons_active<S: Storage>(
    storage: &mut S,
    prefix: &[u8],
    person: &HumanAddr,
    offspring_addr: &CanonicalAddr,
) -> StdResult<()> {
    let mut store = PrefixedStorage::new(prefix, storage);
    let mut load_active: CashMap<StoreOffspringInfo, _, _> = CashMap::init(person.to_string().as_bytes(), &mut store);
    load_active.remove(offspring_addr.as_slice())?;
    Ok(())
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::ListMyOffspring {
            address,
            viewing_key,
            filter,
            start_page,
            page_size,
        } => try_list_my(deps, &address, viewing_key, filter, start_page, page_size),
        QueryMsg::ListActiveOffspring { start_page, page_size } => try_list_active(deps, start_page, page_size),
        QueryMsg::ListInactiveOffspring { start_page, page_size } => try_list_inactive(deps, start_page, page_size),
        QueryMsg::IsKeyValid {
            address,
            viewing_key,
        } => try_validate_key(deps, &address, viewing_key),
    };
    pad_query_result(response, BLOCK_SIZE)
}

/// Returns QueryResult indicating whether the address/key pair is valid
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `address` - a reference to the address whose key should be validated
/// * `viewing_key` - String key used for authentication
fn try_validate_key<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    viewing_key: String,
) -> QueryResult {
    let addr_raw = &deps.api.canonical_address(address)?;
    to_binary(&QueryAnswer::IsKeyValid {
        is_valid: is_key_valid(&deps.storage, addr_raw, viewing_key)?,
    })
}

/// Returns QueryResult listing the active offspring
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `start_page` - optional start page for the offsprings returned and listed
/// * `page_size` - optional number of offspring to return in this page
fn try_list_active<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> QueryResult {
    to_binary(&QueryAnswer::ListActiveOffspring {
        active: display_active_list(&deps.storage, None, ACTIVE_KEY, start_page, page_size)?,
    })
}

/// Returns StdResult<bool> result of validating an address' viewing key
///
/// # Arguments
///
/// * `storage` - a reference to the contract's storage
/// * `address` - a reference to the address whose key should be validated
/// * `viewing_key` - String key used for authentication
fn is_key_valid<S: ReadonlyStorage>(
    storage: &S,
    address: &CanonicalAddr,
    viewing_key: String,
) -> StdResult<bool> {
    // load the address' key
    let read_key = ReadonlyPrefixedStorage::new(PREFIX_VIEW_KEY, storage);
    let load_key: Option<[u8; VIEWING_KEY_SIZE]> = may_load(&read_key, address.as_slice())?;
    let input_key = ViewingKey(viewing_key);
    // if a key was set
    if let Some(expected_key) = load_key {
        // and it matches
        if input_key.check_viewing_key(&expected_key) {
            return Ok(true);
        }
    } else {
        // Checking the key will take significant time. We don't want to exit immediately if it isn't set
        // in a way which will allow to time the command and determine if a viewing key doesn't exist
        input_key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
    }
    Ok(false)
}

/// Returns QueryResult listing the offspring with the address as its owner
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `address` - a reference to the address whose offspring should be listed
/// * `viewing_key` - String key used to authenticate the query
/// * `filter` - optional choice of display filters
/// * `start_page` - optional start page for the offsprings returned and listed
/// * `page_size` - optional number of offspring to return in this page
fn try_list_my<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    viewing_key: String,
    filter: Option<FilterTypes>,
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> QueryResult {
    let addr_raw = &deps.api.canonical_address(address)?;
    // if key matches
    if !is_key_valid(&deps.storage, addr_raw, viewing_key)? {
        return to_binary(&QueryAnswer::ViewingKeyError {
            error: "Wrong viewing key for this address or viewing key not set".to_string(),
        });
    }
    let mut active_list: Option<Vec<StoreOffspringInfo>> = None;
    let mut inactive_list: Option<Vec<StoreInactiveOffspringInfo>> = None;
    // if no filter default to ALL
    let types = filter.unwrap_or(FilterTypes::All);

    // list the active offspring
    if types == FilterTypes::Active || types == FilterTypes::All {
        active_list = Some( display_active_list(
            &deps.storage,
            Some( PREFIX_OWNERS_ACTIVE ),
            address.to_string().as_bytes(),
            start_page,
            page_size,
        )?);
    }
    // list the inactive offspring
    if types == FilterTypes::Inactive || types == FilterTypes::All {
        inactive_list = Some( display_inactive_list(
            &deps.storage,
            Some( PREFIX_OWNERS_INACTIVE ),
            address.to_string().as_bytes(),
            start_page,
            page_size,
        )?);
    }

    return to_binary(&QueryAnswer::ListMyOffspring {
        active: active_list,
        inactive: inactive_list,
    });
}

/// Returns StdResult<Vec<StoreOffspringInfo>>
///
/// provide the appropriate list of active offspring
///
/// # Arguments
///
/// * `api` - reference to the Api used to convert canonical and human addresses
/// * `storage` - a reference to the contract's storage
/// * `prefix` - optional storage prefix to load from
/// * `key` - storage key to read (user addr byte)
/// * `start_page` - optional start page for the offsprings returned and listed
/// * `page_size` - optional number of offspring to return in this page
fn display_active_list<S: ReadonlyStorage>(
    storage: &S,
    prefix: Option<&[u8]>,
    key: &[u8],
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> StdResult<Vec<StoreOffspringInfo>> {
    let page_number = start_page.unwrap_or(0);
    let size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    let list: Vec<StoreOffspringInfo>;
    match prefix {
        Some(pref) => {
            // get owner's active list
            let read = &ReadonlyPrefixedStorage::new(pref, storage);
            let user_store: ReadOnlyCashMap<StoreOffspringInfo, _> = ReadOnlyCashMap::init(key, read);
            list = user_store.paging(page_number, size)?;
        },
        None => {
            // get factory's active list
            let active_store: ReadOnlyCashMap<StoreOffspringInfo, _> = ReadOnlyCashMap::init(key, storage);
            list = active_store.paging(page_number, size)?;
        }
    }
    Ok(list)
}

/// Returns StdResult<Vec<InactiveOffspringInfo>>
///
/// provide the appropriate list of inactive offspring
///
/// # Arguments
///
/// * `storage` - a reference to the contract's storage
/// * `prefix` - optional storage prefix to load from
/// * `key` - storage key to read
/// * `start_page` - optional start page for the offsprings returned and listed
/// * `page_size` - optional number of offspring to return in this page
fn display_inactive_list<S: ReadonlyStorage>(
    storage: &S,
    prefix: Option<&[u8]>,
    key: &[u8],
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> StdResult<Vec<StoreInactiveOffspringInfo>> {
    let page_number = start_page.unwrap_or(0);
    let size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    let list: Vec<StoreInactiveOffspringInfo>;
    match prefix {
        Some(pref) => {
            // get owner's inactive list
            let read = &ReadonlyPrefixedStorage::new(pref, storage);
            let user_store: ReadOnlyCashMap<StoreInactiveOffspringInfo, _> = ReadOnlyCashMap::init(key, read);
            list = user_store.paging(page_number, size)?;
        },
        None => {
            // get factory's inactive list
            let active_store: ReadOnlyCashMap<StoreInactiveOffspringInfo, _> = ReadOnlyCashMap::init(key, storage);
            list = active_store.paging(page_number, size)?;
        }
    }
    Ok(list)
}

/// Returns QueryResult listing the inactive offspring
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `start_page` - optional start page for the offsprings returned and listed
/// * `page_size` - optional number of offspring to display
fn try_list_inactive<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> QueryResult {
    to_binary(&QueryAnswer::ListInactiveOffspring {
        inactive: display_inactive_list(&deps.storage, None, INACTIVE_KEY, start_page, page_size)?,
    })
}
