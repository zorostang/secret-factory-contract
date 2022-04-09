use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, InitResult, HandleResult, QueryResult, HumanAddr,
};
use secret_toolkit::utils::HandleCallback;

use crate::factory_msg::{FactoryOffspringInfo, FactoryHandleMsg};
use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

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
        index:  msg.index,
        password: msg.password,
        active: true,
        offspring_addr: env.contract.address,
        description: msg.description,
        count: msg.count,
        owner: deps.api.canonical_address(&msg.owner.clone())?,
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
    let cosmos_msg = reg_offspring_msg.to_cosmos_msg(
        msg.factory.code_hash, msg.factory.address, None)?;

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
        HandleMsg::Increment {} => try_increment(deps, env),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
    }
}

/// Returns HandleResult
///
/// increases the counter. Can be executed by anyone.
///
/// # Arguments
///
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `_env` - Env of contract's environment
pub fn try_increment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
) -> HandleResult {
    enforce_active(deps)?;
    config(&mut deps.storage).update(|mut state| {
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
    enforce_active(deps)?;
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    config(&mut deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
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
pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
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
fn query_count<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    _address: &HumanAddr, 
    _viewing_key: String
) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.count })
}

/// Returns StdResult<()>
///
/// makes sure that the contract state is active
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies.
fn enforce_active<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) -> StdResult<()> {
    let config = config_read(&deps.storage).load()?;

    if config.active {
        Ok(())
    } else {
        return Err(StdError::generic_err(
            "This contract is inactive.",
        ));
    }
}