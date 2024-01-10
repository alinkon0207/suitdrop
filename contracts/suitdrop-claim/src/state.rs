use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::vesting::ClaimInfo;
use cosmwasm_std::{Addr, CanonicalAddr, Uint128};
use cw_storage_plus::{Item, Map};

/// This structure stores the main parameters for the generator vesting contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// Address that's allowed to change contract parameters
    pub owner: CanonicalAddr,
    /// The address of the TERRA token
    pub cw20_token_address: Addr,
    pub claim_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRoot {
    pub root: String,
}

/// Stores the contract config at the given key.
pub const CONFIG: Item<Config> = Item::new("config");
pub const CLAIM_INFO: Map<&Addr, ClaimInfo> = Map::new("claim_info");
pub const MERKLE_ROOT: Item<MerkleRoot> = Item::new("merkle_root");
