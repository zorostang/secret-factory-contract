use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, ReadonlyStorage, StdError, StdResult, Storage,
};

use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use std::collections::{HashMap, HashSet};

use secret_toolkit::{
    storage::{AppendStore, AppendStoreMut},
    utils::{pad_handle_result, pad_query_result, InitCallback},
};

use crate::rand::sha_256;
use crate::state::{
    load, may_load, remove, save, Config, ACTIVE_KEY, BLOCK_SIZE, CONFIG_KEY, PENDING_KEY,
    PREFIX_ACTIVE_INFO, PREFIX_INACTIVE_INFO, PREFIX_OWNERS_ACTIVE, PREFIX_OWNERS_INACTIVE,
    PREFIX_VIEW_KEY, PRNG_SEED_KEY,
};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use crate::{
    msg::{
        ContractInfo, FilterTypes, HandleAnswer, HandleMsg, InactiveOffspringInfo, InitMsg,
        OffspringContractInfo, OffspringInfo, QueryAnswer, QueryMsg, RegisterOffspringInfo,
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
    let active: HashSet<u32> = HashSet::new();

    let config = Config {
        version: msg.offspring_contract,
        symdecmap: HashMap::new(),
        index: 0,
        stopped: false,
        admin: deps.api.canonical_address(&env.message.sender)?,
    };

    save(&mut deps.storage, CONFIG_KEY, &config)?;
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;
    save(&mut deps.storage, ACTIVE_KEY, &active)?;

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
        HandleMsg::DeactivateOffspring { index, owner } => {
            try_deactivate_offspring(deps, env, index, &owner)
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
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;
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
        index: config.index,
        password: password.clone(),
        owner,
        count,
        description,
    };
    // increment the index for the next offspring
    config.index += 1;
    save(&mut deps.storage, CONFIG_KEY, &config)?;

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
/// Registers the calling offsspring by saving its info and adding it to the appropriate lists
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
    let offspring = reg_offspring.to_store_offspring_info(offspring_addr);

    // save the offspring info keyed by its index
    let mut info_store = PrefixedStorage::new(PREFIX_ACTIVE_INFO, &mut deps.storage);
    save(
        &mut info_store,
        &reg_offspring.index.to_le_bytes(),
        &offspring,
    )?;

    // add the offspring address to list of active offspring
    let mut active: HashSet<u32> = load(&deps.storage, ACTIVE_KEY)?;
    active.insert(reg_offspring.index);
    save(&mut deps.storage, ACTIVE_KEY, &active)?;

    // get list of owner's active offspring
    let owner_raw = &deps.api.canonical_address(&owner)?;
    let mut owner_store = PrefixedStorage::new(PREFIX_OWNERS_ACTIVE, &mut deps.storage);
    let load_offsprings: Option<HashSet<u32>> = may_load(&owner_store, owner_raw.as_slice())?;
    let mut my_active = load_offsprings.unwrap_or_default();
    // add this offspring to owner's list
    my_active.insert(reg_offspring.index);
    save(&mut owner_store, owner_raw.as_slice(), &my_active)?;

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
/// * `index` - offspring's index in factory
/// * `owner` - offspring's owner
fn try_deactivate_offspring<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    index: u32,
    owner: &HumanAddr,
) -> HandleResult {
    let offspring_addr = &deps.api.canonical_address(&env.message.sender)?;

    // verify offspring is in active list, and not a spam attempt
    let (may_active, may_info, may_error) =
        authenticate_offspring(&deps.storage, offspring_addr, index)?;
    if let Some(error) = may_error {
        return error;
    }
    // delete the active offspring info
    let mut info_store = PrefixedStorage::new(PREFIX_ACTIVE_INFO, &mut deps.storage);
    info_store.remove(&index.to_le_bytes());
    // remove the offspring from the active list
    let mut active = may_active.unwrap();
    active.remove(&index);
    save(&mut deps.storage, ACTIVE_KEY, &active)?;

    // set the inactive offspring info
    let offspring_info = may_info.unwrap();
    let inactive_info = offspring_info.to_store_inactive_offspring_info();
    let mut inactive_info_store = PrefixedStorage::new(PREFIX_INACTIVE_INFO, &mut deps.storage);
    let mut inactive_store = AppendStoreMut::attach_or_create(&mut inactive_info_store)?;
    let inactive_index = inactive_store.len();
    inactive_store.push(&inactive_info)?;

    // remove offspring from owner's active list
    let owner_raw = &deps.api.canonical_address(owner)?;
    remove_from_persons_active(&mut deps.storage, PREFIX_OWNERS_ACTIVE, owner_raw, index)?;
    // add to owner's inactive list
    let mut owner_store = PrefixedStorage::multilevel(
        &[PREFIX_OWNERS_INACTIVE, owner_raw.as_slice()],
        &mut deps.storage,
    );
    let mut owner_inactive = AppendStoreMut::attach_or_create(&mut owner_store)?;
    owner_inactive.push(&inactive_index)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: None,
    })
}

/// Returns StdResult<(Option<HashSet<u32>>, Option<StoreOffspringInfo>, Option<HandleResult>)>
///
/// verifies that the offspring is in the active list, and returns the active offspring
/// list, the offspring information, or a possible error
///
/// # Arguments
///
/// * `storage` - a reference to contract's storage
/// * `offspring` - a reference to the offspring's address
/// * `index` - index/key of the offspring
#[allow(clippy::type_complexity)]
fn authenticate_offspring<S: ReadonlyStorage>(
    storage: &S,
    offspring: &CanonicalAddr,
    index: u32,
) -> StdResult<(
    Option<HashSet<u32>>,
    Option<StoreOffspringInfo>,
    Option<HandleResult>,
)> {
    let mut error: Option<HandleResult> = None;
    let mut info: Option<StoreOffspringInfo> = None;
    let active: Option<HashSet<u32>> = may_load(storage, ACTIVE_KEY)?;
    if let Some(active_set) = active.as_ref() {
        // get the offspring information
        let info_store = ReadonlyPrefixedStorage::new(PREFIX_ACTIVE_INFO, storage);
        info = may_load(&info_store, &index.to_le_bytes())?;
        if let Some(offspring_info) = info.as_ref() {
            if offspring_info.address != *offspring || !active_set.contains(&index) {
                error = Some(Ok(HandleResponse {
                    messages: vec![],
                    log: vec![log(
                        "Unauthorized",
                        "You are not an active offspring this factory created",
                    )],
                    data: None,
                }));
            }
        } else {
            error = Some(Ok(HandleResponse {
                messages: vec![],
                log: vec![
                    log(
                        "Error",
                        "Unable to register action with the factory contract",
                    ),
                    log("Reason", "Missing offspring information"),
                ],
                data: None,
            }));
        }
    // if you can't load the active offspring list, it is an error but still let offspring process
    } else {
        error = Some(Ok(HandleResponse {
            messages: vec![],
            log: vec![
                log(
                    "Error",
                    "Unable to register action with the factory contract",
                ),
                log("Reason", "Missing active offspring list"),
            ],
            data: None,
        }));
    }
    Ok((active, info, error))
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
/// * `index` - index of the offspring to remove
fn remove_from_persons_active<S: Storage>(
    storage: &mut S,
    prefix: &[u8],
    person: &CanonicalAddr,
    index: u32,
) -> StdResult<()> {
    let mut store = PrefixedStorage::new(prefix, storage);
    let load_active: Option<HashSet<u32>> = may_load(&store, person.as_slice())?;
    if let Some(mut active) = load_active {
        active.remove(&index);
        save(&mut store, person.as_slice(), &active)?;
    }
    Ok(())
}

/// Returns StdResult<(HashSet<u32>, bool)> which is the address' updated active list
/// and a bool that is true if the list has been changed from what was in storage
///
/// remove any inactive offsprings from a list
/// (useful for other lists that might be added on top of this template in the future)
///
/// # Arguments
///
/// * `storage` - a reference to active list storage subspace
/// * `address` - a reference to the canonical address of the person the list belongs to
/// * `active` - a mutable reference to the HashSet list of active offspring
fn _filter_only_active<S: ReadonlyStorage>(
    storage: &S,
    address: &CanonicalAddr,
    active: &mut HashSet<u32>,
) -> StdResult<(HashSet<u32>, bool)> {
    // get person's current list
    let load_offsprings: Option<HashSet<u32>> = may_load(storage, address.as_slice())?;

    // if there are active offspring in the list
    if let Some(my_offspring) = load_offsprings {
        let start_len = my_offspring.len();
        // only keep the intersection of the person's list and the active offspring list
        let my_active: HashSet<u32> = my_offspring.iter().filter_map(|v| active.take(v)).collect();
        let updated = start_len != my_active.len();
        return Ok((my_active, updated));
        // if not just return an empty list
    }
    Ok((HashSet::new(), false))
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
        } => try_list_my(deps, &address, viewing_key, filter),
        QueryMsg::ListActiveOffspring {} => try_list_active(deps),
        QueryMsg::ListInactiveOffspring { before, page_size } => {
            try_list_inactive(deps, before, page_size)
        }
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
fn try_list_active<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    to_binary(&QueryAnswer::ListActiveOffspring {
        active: display_active_list(&deps.api, &deps.storage, None, ACTIVE_KEY)?,
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
fn try_list_my<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    viewing_key: String,
    filter: Option<FilterTypes>,
) -> QueryResult {
    let addr_raw = &deps.api.canonical_address(address)?;
    // if key matches
    if is_key_valid(&deps.storage, addr_raw, viewing_key)? {
        let mut active_list: Option<Vec<OffspringInfo>> = None;
        let mut inactive_list: Option<Vec<InactiveOffspringInfo>> = None;
        // if no filter default to ALL
        let types = filter.unwrap_or(FilterTypes::All);

        // list the active offspring
        if types == FilterTypes::Active || types == FilterTypes::All {
            active_list = display_active_list(
                &deps.api,
                &deps.storage,
                Some(PREFIX_OWNERS_ACTIVE),
                addr_raw.as_slice(),
            )?;
        }
        // list the inactive offspring
        if types == FilterTypes::Inactive || types == FilterTypes::All {
            inactive_list = display_addr_inactive(
                &deps.api,
                &deps.storage,
                PREFIX_OWNERS_INACTIVE,
                addr_raw.as_slice(),
            )?;
        }

        return to_binary(&QueryAnswer::ListMyOffspring {
            active: active_list,
            inactive: inactive_list,
        });
    }
    to_binary(&QueryAnswer::ViewingKeyError {
        error: "Wrong viewing key for this address or viewing key not set".to_string(),
    })
}

/// Returns StdResult<Option<Vec<OffspringInfo>>>
///
/// provide the appropriate list of active offspring
///
/// # Arguments
///
/// * `api` - reference to the Api used to convert canonical and human addresses
/// * `storage` - a reference to the contract's storage
/// * `prefix` - optional storage prefix to load from
/// * `key` - storage key to read
fn display_active_list<S: ReadonlyStorage, A: Api>(
    api: &A,
    storage: &S,
    prefix: Option<&[u8]>,
    key: &[u8],
) -> StdResult<Option<Vec<OffspringInfo>>> {
    let load_list: Option<HashSet<u32>> = if let Some(pref) = prefix {
        // reading a person's list
        let read = &ReadonlyPrefixedStorage::new(pref, storage);
        // if reading a prefix list (owner's list in this template)
        may_load(read, key)?
    // read the factory's active list
    } else {
        may_load(storage, key)?
    };
    // turn list of active offspring to a vec of displayable offspring infos
    let actives = match load_list {
        Some(list) => {
            let mut display_list = Vec::new();
            let read_info = &ReadonlyPrefixedStorage::new(PREFIX_ACTIVE_INFO, storage);
            for index in list.iter() {
                // get this offspring's info
                let load_info: Option<StoreOffspringInfo> =
                    may_load(read_info, &index.to_le_bytes())?;
                if let Some(info) = load_info {
                    display_list.push(OffspringInfo {
                        address: api.human_address(&info.address)?,
                        password: info.password,
                    });
                }
            }
            display_list
        }
        None => Vec::new(),
    };
    if actives.is_empty() {
        return Ok(None);
    }
    Ok(Some(actives))
}

/// Returns StdResult<Option<Vec<InactiveOffspringInfo>>>
///
/// provide the appropriate list of inactive offspring
///
/// # Arguments
///
/// * `api` - reference to the Api used to convert canonical and human addresses
/// * `storage` - a reference to the contract's storage
/// * `prefix` - storage prefix to load from
/// * `key` - storage key to read
fn display_addr_inactive<S: ReadonlyStorage, A: Api>(
    api: &A,
    storage: &S,
    prefix: &[u8],
    key: &[u8],
) -> StdResult<Option<Vec<InactiveOffspringInfo>>> {
    let list_store = ReadonlyPrefixedStorage::multilevel(&[prefix, key], storage);
    let may_read_list = AppendStore::<u32, _>::attach(&list_store);
    let mut inactive_vec = Vec::new();
    if let Some(inactive_list) = may_read_list.and_then(|r| r.ok()) {
        let info_store = ReadonlyPrefixedStorage::new(PREFIX_INACTIVE_INFO, storage);
        let may_read_info = AppendStore::<StoreInactiveOffspringInfo, _>::attach(&info_store);
        if let Some(closed_info) = may_read_info.and_then(|r| r.ok()) {
            // grab backwards from the starting point
            for index_res in inactive_list.iter().rev() {
                if let Ok(index) = index_res {
                    // get this offspring's info
                    let load_info = closed_info.get_at(index);
                    if let Ok(info) = load_info {
                        inactive_vec.push(InactiveOffspringInfo {
                            index: None,
                            address: api.human_address(&info.address)?,
                            password: info.password,
                        });
                    }
                }
            }
        }
    }
    if inactive_vec.is_empty() {
        return Ok(None);
    }
    Ok(Some(inactive_vec))
}

/// Returns QueryResult listing the inactive offspring
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `before` - optional u32 index of the earliest offspring you do not want to display
/// * `page_size` - optional number of offspring to display
fn try_list_inactive<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    before: Option<u32>,
    page_size: Option<u32>,
) -> QueryResult {
    let read_store = ReadonlyPrefixedStorage::new(PREFIX_INACTIVE_INFO, &deps.storage);
    let may_read_store = AppendStore::<StoreInactiveOffspringInfo, _>::attach(&read_store);
    let mut inactive_vec = Vec::new();
    if let Some(inactive_store) = may_read_store.and_then(|r| r.ok()) {
        // start iterating from the last close or before given index
        let len = inactive_store.len();
        let mut pos = before.unwrap_or(len);
        if pos > len {
            pos = len;
        }
        let skip = (len - pos) as usize;
        let quant = page_size.unwrap_or(200) as usize;
        // grab backwards from the starting point
        for (i, res) in inactive_store
            .iter()
            .enumerate()
            .rev()
            .skip(skip)
            .take(quant)
        {
            if let Ok(info) = res {
                inactive_vec.push(InactiveOffspringInfo {
                    index: Some(i as u32),
                    address: deps.api.human_address(&info.address)?,
                    password: info.password,
                });
            }
        }
    }
    let inactive = if inactive_vec.is_empty() {
        None
    } else {
        Some(inactive_vec)
    };
    to_binary(&QueryAnswer::ListInactiveOffspring { inactive })
}
