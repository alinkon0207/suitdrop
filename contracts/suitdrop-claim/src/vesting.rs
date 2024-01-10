use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

/// This structure describes the parameters used for creating a contract.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    /// The address of the token that's being vested
    pub cw20_token_address: String,
    pub claim_amount: Uint128,
}

/// This structure describes the execute messages available in the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Update Config
    UpdateConfig {
        owner: Option<String>,
        cw20_token_address: Option<String>,
        /// The amount of tokens to claim
        claim_amount: Option<Uint128>,
    },
    /// Set Merkle tree root address
    RegisterMerkleRoot { root: Option<String> },
    /// Claim claims vested tokens and sends them to a recipient
    Claim {
        // Proof for merkle tree
        proof: Vec<String>,
    },
    /// Withdraw all balance
    WithdrawAll {},
}

/// This structure stores user info for vesting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ClaimInfo {
    pub amount: Uint128,        // The remain token amount
    pub claimed_timestamp: u64, // The time to claim the vesting amount
}

/// This structure describes the query messages available in the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    MerkleRoot {},
    ClaimInfo { address: String },
}

/// This structure describes a custom struct used to return the contract configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    /// The address of the TERRA token
    pub cw20_token_address: String,
    pub claim_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MerkleRootResponse {
    pub root: String,
}

/// This structure describes a custom struct used to return vesting data about a specific vesting target.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ClaimResponse {
    pub amount: Uint128,
    pub is_claim: bool,
    pub is_reward: bool,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
