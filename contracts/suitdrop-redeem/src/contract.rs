#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, ReplyOn,
    Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:suitdrop-redeem";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.max_tokens == 0 {
        return Err(ContractError::InvalidMaxTokens {});
    }

    let config = Config {
        owner: msg.owner.clone(),
        cw721_address: None,
        max_tokens: msg.max_tokens,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        unused_token_id: 1,
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: (WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_binary(
                &(Cw721InstantiateMsg {
                    name: msg.name.clone() + " cw721_base",
                    symbol: msg.symbol,
                    minter: env.contract.address.to_string(),
                }),
            )?,
            funds: vec![],
            admin: None,
            label: msg.name.clone(),
        })
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

/// Handling contract migration
/// To make a contract migratable, you need
/// - this entry_point implemented
/// - only contract admin can migrate, so admin has to be set at contract initiation time
/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        // Find matched incoming message variant and execute them with your custom logic.
        //
        // With `Response` type, it is possible to dispatch message to invoke external logic.
        // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    }
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint { uri, extension } => execute_mint(deps, info, uri, extension),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    uri: String,
    extension: Extension,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if config.cw721_address == None {
        return Err(ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(ContractError::MaxTokensExceed {});
    }

    let mint_msg: Cw721ExecuteMsg<Extension, Empty> = Cw721ExecuteMsg::Mint {
        token_id: config.unused_token_id.to_string(),
        owner: info.sender.clone().into(),
        token_uri: uri.clone().into(),
        extension: extension.clone(),
    };

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });

    config.unused_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_message(callback))
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        cw721_address: config.cw721_address,
        max_tokens: config.max_tokens,
        name: config.name,
        symbol: config.symbol,
        unused_token_id: config.unused_token_id,
    })
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.cw721_address != None {
        return Err(crate::ContractError::Cw721AlreadyLinked {});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(crate::ContractError::InvalidTokenReplyId {});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    config.cw721_address = Addr::unchecked(reply.contract_address).into();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}
