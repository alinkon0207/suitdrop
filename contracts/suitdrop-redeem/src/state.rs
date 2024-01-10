use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub cw721_address: Option<Addr>,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub unused_token_id: u32,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
