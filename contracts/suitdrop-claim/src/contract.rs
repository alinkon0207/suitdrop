use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdResult, SubMsg, Uint128, WasmMsg, WasmQuery,
};

use crate::state::{Config, MerkleRoot, CLAIM_INFO, CONFIG, MERKLE_ROOT};

use crate::error::ContractError;
use crate::vesting::{
    ClaimInfo, ConfigResponse, ExecuteMsg, InstantiateMsg, MerkleRootResponse, MigrateMsg, QueryMsg,
};

use cw2::set_contract_version;
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use sha2::Digest;

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "suitdrop-claim";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a new contract with the specified parameters in [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_canonicalize(_info.sender.as_str())?,
        cw20_token_address: deps.api.addr_validate(&msg.cw20_token_address)?,
        claim_amount: msg.claim_amount,
    };

    CONFIG.save(deps.storage, &config)?;
    CLAIM_INFO.save(
        deps.storage,
        &_info.sender,
        &ClaimInfo {
            amount: Uint128::zero(),
            claimed_timestamp: 0,
        },
    )?;
    Ok(Response::new())
}

/// Exposes execute functions available in the contract.
///
/// ## Variants
/// * **ExecuteMsg::Claim { recipient, amount }** Claims vested tokens and transfers them to the vesting recipient.
///
/// * **ExecuteMsg::Receive(msg)** Receives a message of type [`Cw20ReceiveMsg`] and processes it
/// depending on the received template.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            cw20_token_address,
            claim_amount,
        } => execute_update_config(deps, env, info, owner, cw20_token_address, claim_amount),
        ExecuteMsg::RegisterMerkleRoot { root } => {
            execute_register_merkle_root(deps, env, info, root)
        }
        ExecuteMsg::WithdrawAll {} => try_withdraw_all(deps, env, info),
        ExecuteMsg::Claim { proof } => claim(deps, env, info, proof),
    }
}

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    cw20_token_address: Option<String>,
    claim_amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;
        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(cw20_token_address) = cw20_token_address {
        let _ = deps.api.addr_validate(&cw20_token_address)?;

        config.cw20_token_address = deps.api.addr_validate(&cw20_token_address)?;
    }

    if let Some(claim_amount) = claim_amount {
        config.claim_amount = claim_amount;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Claims vested tokens and transfers them to the vesting recipient.
///
/// * **recipient** vesting recipient for which to claim tokens.
///
/// * **amount** amount of vested tokens to claim.
pub fn claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let merkle_info = MERKLE_ROOT
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    let hash = sha2::Sha256::digest(info.sender.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    let merkle_root = merkle_info.root;
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }
    let total_balance = query_token_balance(
        &deps.querier,
        config.cw20_token_address.clone(),
        env.contract.address.clone(),
    )?;
    if total_balance.clone().is_zero() {
        return Err(ContractError::Insufficient {});
    }

    let mut response = Response::new();

    if let Some(claim_info) = CLAIM_INFO.may_load(deps.storage, &info.sender)? {
        return Err(ContractError::Claimed {});
    } else {
        response = response.add_submessage(SubMsg::new(WasmMsg::Execute {
            contract_addr: config.cw20_token_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: config.claim_amount,
            })?,
        }));

        CLAIM_INFO.save(
            deps.storage,
            &info.sender,
            &ClaimInfo {
                amount: config.claim_amount,
                claimed_timestamp: env.block.time.seconds(),
            },
        )?;
    }

    Ok(response.add_attributes(vec![
        attr("action", "claim"),
        attr("address", &info.sender),
        attr("claimed_amount", config.claim_amount),
    ]))
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    root: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut merkle_info = MerkleRoot {
        root: String::from(""),
    };

    // check merkle root length
    if let Some(root) = root {
        let mut root_buf: [u8; 32] = [0; 32];
        hex::decode_to_slice(root.to_string(), &mut root_buf)?;
        merkle_info.root = root;
    }

    MERKLE_ROOT.save(deps.storage, &merkle_info)?;

    Ok(Response::new().add_attributes(vec![attr("action", "register_merkle_root")]))
}

pub fn try_withdraw_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    //running on only stage 1
    let config = CONFIG.load(deps.storage)?;
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let total_balance = query_token_balance(
        &deps.querier,
        config.cw20_token_address.clone(),
        env.contract.address.clone(),
    )?;

    // create transfer cw20 msg
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.into(),
        amount: total_balance,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: config.cw20_token_address.to_string(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();

    Ok(Response::new()
        .add_attribute("action", "withdraw_all")
        .add_attribute("amount", total_balance)
        .add_submessages(vec![SubMsg::new(cw20_transfer_cosmos_msg)]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot {} => to_binary(&query_merkle_root(deps)?),
        QueryMsg::ClaimInfo { address } => to_binary(&query_claim_info(deps, address)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        cw20_token_address: config.cw20_token_address.to_string(),
        claim_amount: config.claim_amount,
    })
}

pub fn query_merkle_root(deps: Deps) -> StdResult<MerkleRootResponse> {
    let merkle_info = MERKLE_ROOT.load(deps.storage)?;

    Ok(MerkleRootResponse {
        root: merkle_info.root,
    })
}

pub fn query_claim_info(deps: Deps, address: String) -> StdResult<ClaimInfo> {
    let receipent = deps.api.addr_validate(&address)?;

    if let Some(claim_info) = CLAIM_INFO.may_load(deps.storage, &receipent)? {
        Ok(claim_info)
    } else {
        Ok(ClaimInfo {
            amount: Uint128::zero(),
            claimed_timestamp: 0,
        })
    }
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
