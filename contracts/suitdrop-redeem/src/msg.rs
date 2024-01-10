use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::Addr;
use cw721_base::Extension;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub token_code_id: u64,
}
/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    Mint { uri: String, extension: Extension },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub cw721_address: Option<Addr>,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub unused_token_id: u32,
}
