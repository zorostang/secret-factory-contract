use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, StdError, StdResult, Storage,
};
use secret_toolkit::utils::{HandleCallback, Query};

use crate::factory_msg::{
    FactoryHandleMsg, FactoryOffspringInfo, FactoryQueryMsg, IsKeyValidWrapper,
};
use crate::msg::{
    HandleMsg, InitMsg, QueryAnswer, QueryMsg,
};
use crate::state::{config, config_read, load, save, State, CONFIG_KEY};

////////////////////////////////////// Init ///////////////////////////////////////
/// Returns InitResult
///
/// Initializes the offspring contract state.
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
    let state = State {
        factory: msg.factory.clone(),
        index: msg.index,
        password: msg.password,
        active: true,
        offspring_addr: env.contract.address,
        description: msg.description,
        count: msg.count,
        owner: msg.owner.clone(),
    };

    config(&mut deps.storage).save(&state)?;

    // perform register callback to factory
    let offspring = FactoryOffspringInfo {
        index: msg.index,
        password: msg.password,
    };
    let reg_offspring_msg = FactoryHandleMsg::RegisterOffspring {
        owner: msg.owner,
        offspring,
    };
    let cosmos_msg =
        reg_offspring_msg.to_cosmos_msg(msg.factory.code_hash, msg.factory.address, None)?;

    Ok(InitResponse {
        messages: vec![cosmos_msg],
        log: vec![],
    })
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
    match msg {
        HandleMsg::Increment {} => try_increment(deps),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
        HandleMsg::Deactivate {} => try_deactivate(deps, env),
    }
}

/// Returns HandleResult
///
/// deactivates the offspring and lets the factory know.
///
/// # Arguments
///
/// * `deps`  - mutable reference to Extern containing all the contract's external dependencies
/// * `env`   - Env of contract's environment
pub fn try_deactivate<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let mut state: State = load(&deps.storage, CONFIG_KEY)?;
    enforce_active(&state)?;
    if env.message.sender != state.owner {
        return Err(StdError::Unauthorized { backtrace: None });
    }
    state.active = false;
    save(&mut deps.storage, CONFIG_KEY, &state)?;
    // let factory know
    let deactivate_msg = FactoryHandleMsg::DeactivateOffspring {
        index: state.index,
        owner: state.owner,
    }
    .to_cosmos_msg(state.factory.code_hash, state.factory.address, None)?;

    Ok(HandleResponse {
        messages: vec![deactivate_msg],
        log: vec![],
        data: None,
    })
}

/// Returns HandleResult
///
/// increases the counter. Can be executed by anyone.
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
pub fn try_increment<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) -> HandleResult {
    let mut config = config(&mut deps.storage);
    let state = &config.load()?;
    enforce_active(state)?;
    config.update(|mut state| {
        state.count += 1;
        debug_print!("count = {}", state.count);
        Ok(state)
    })?;
    debug_print("count incremented successfully");
    Ok(HandleResponse::default())
}

/// Returns HandleResult
///
/// resets the counter to count. Can only be executed by owner.
///
/// # Arguments
///
/// * `deps`  - mutable reference to Extern containing all the contract's external dependencies
/// * `env`   - Env of contract's environment
/// * `count` - The value to reset the counter to.
pub fn try_reset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    count: i32,
) -> HandleResult {
    config(&mut deps.storage).update(|mut state| {
        enforce_active(&state)?;
        if env.message.sender != state.owner {
            return Err(StdError::Unauthorized { backtrace: None });
        }
        state.count = count;
        Ok(state)
    })?;
    debug_print("count reset successfully");
    Ok(HandleResponse::default())
}

/////////////////////////////////////// Query /////////////////////////////////////
/// Returns QueryResult
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `msg` - QueryMsg passed in with the query call
pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    match msg {
        QueryMsg::GetCount {
            address,
            viewing_key,
        } => to_binary(&query_count(deps, &address, viewing_key)?),
    }
}

/// Returns StdResult<CountResponse> displaying the count.
///
/// # Arguments
///
/// * `deps` - reference to Extern containing all the contract's external dependencies
/// * `address` - a reference to the address whose viewing key is being validated.
/// * `viewing_key` - String key used to authenticate the query.
fn query_count<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    viewing_key: String,
) -> StdResult<QueryAnswer> {
    let state = config_read(&deps.storage).load()?;
    if state.owner == *address {
        enforce_valid_viewing_key(deps, &state, address, viewing_key)?;
        return Ok(QueryAnswer::CountResponse { count: state.count });
    } else {
        return Err(StdError::generic_err(
            // error message chosen as to not leak information.
            "This address does not have permission and/or viewing key is not valid",
        ));
    }
}

/// Returns StdResult<()>
///
/// makes sure that the address and the viewing key match in the factory contract.
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies.
/// * `state` - a reference to the State of the contract.
/// * `address` - a reference to the address whose viewing key is being validated.
/// * `viewing_key` - String key used to authenticate a query.
fn enforce_valid_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    state: &State,
    address: &HumanAddr,
    viewing_key: String,
) -> StdResult<()> {
    let state_clone = state.clone();
    let key_valid_msg = FactoryQueryMsg::IsKeyValid {
        address: address.clone(),
        viewing_key,
    };
    let key_valid_response: IsKeyValidWrapper = key_valid_msg.query(
        &deps.querier,
        state_clone.factory.code_hash,
        state_clone.factory.address,
    )?;
    // if authenticated
    if key_valid_response.is_key_valid.is_valid {
        Ok(())
    } else {
        return Err(StdError::generic_err(
            // error message chosen as to not leak information.
            "This address does not have permission and/or viewing key is not valid",
        ));
    }
}

/// Returns StdResult<()>
///
/// makes sure that the contract state is active
///
/// # Arguments
///
/// * `state` - a reference to the State of the contract.
fn enforce_active(state: &State) -> StdResult<()> {
    if state.active {
        Ok(())
    } else {
        return Err(StdError::generic_err("This contract is inactive."));
    }
}