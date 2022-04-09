use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, InitResult, HandleResult, QueryResult, HumanAddr,
};

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
        description: msg.description,
        count: msg.count,
        owner: deps.api.canonical_address(&msg.owner.clone())?,
    };

    config(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized with the factory {:?} by {}", msg.factory, env.message.sender);

    // IMPLEMENT CALLBACK HERE.
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env};
//     use cosmwasm_std::{coins, from_binary, StdError};

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(20, &[]);

//         let msg = InitMsg { count: 17 };
//         let env = mock_env("creator", &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = init(&mut deps, env, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(&deps, QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

//     #[test]
//     fn increment() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));

//         let msg = InitMsg { count: 17 };
//         let env = mock_env("creator", &coins(2, "token"));
//         let _res = init(&mut deps, env, msg).unwrap();

//         // anyone can increment
//         let env = mock_env("anyone", &coins(2, "token"));
//         let msg = HandleMsg::Increment {};
//         let _res = handle(&mut deps, env, msg).unwrap();

//         // should increase counter by 1
//         let res = query(&deps, QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(18, value.count);
//     }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies(20, &coins(2, "token"));

//         let msg = InitMsg { count: 17 };
//         let env = mock_env("creator", &coins(2, "token"));
//         let _res = init(&mut deps, env, msg).unwrap();

//         // not anyone can reset
//         let unauth_env = mock_env("anyone", &coins(2, "token"));
//         let msg = HandleMsg::Reset { count: 5 };
//         let res = handle(&mut deps, unauth_env, msg);
//         match res {
//             Err(StdError::Unauthorized { .. }) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_env = mock_env("creator", &coins(2, "token"));
//         let msg = HandleMsg::Reset { count: 5 };
//         let _res = handle(&mut deps, auth_env, msg).unwrap();

//         // should now be 5
//         let res = query(&deps, QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
