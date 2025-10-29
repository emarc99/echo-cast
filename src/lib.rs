use async_graphql::{Request, Response};
use linera_sdk::{
    graphql::GraphQLMutationRoot,
    linera_base_types::{AccountOwner, Amount, ChainId, ContractAbi, ServiceAbi, Timestamp},
};
use serde::{Deserialize, Serialize};

pub struct PredictionMarketAbi;

impl ContractAbi for PredictionMarketAbi {
    type Operation = Operation;
    type Response = u64;
}

impl ServiceAbi for PredictionMarketAbi {
    type Query = Request;
    type QueryResponse = Response;
}

/// Application operations
#[derive(Debug, Deserialize, Serialize, GraphQLMutationRoot)]
pub enum Operation {
    /// Create a new prediction market
    CreateMarket {
        question: String,
        outcomes: Vec<String>,
        resolution_time: Timestamp,
    },

    /// Place a stake on an outcome (from user's microchain)
    Stake {
        market_id: u64,
        outcome_index: u32,
        amount: Amount,
    },

    /// Update odds based on sentiment (oracle only)
    UpdateOdds {
        market_id: u64,
        new_odds: Vec<f64>,
        sentiment_score: i32,
    },

    /// Resolve market with winning outcome (oracle only)
    Resolve {
        market_id: u64,
        winning_outcome: u32,
    },

    /// Subscribe to sentiment updates for a market
    Subscribe,

    /// Add authorized oracle
    AddOracle { oracle_chain: ChainId },
}

/// Cross-chain messages for oracle broadcasts
#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    /// Broadcast sentiment update to all subscribed chains
    SentimentUpdate {
        market_id: u64,
        odds: Vec<f64>,
        sentiment_score: i32,
    },

    /// Notify user chain of payout
    Payout {
        market_id: u64,
        amount: Amount,
        user: AccountOwner,
    },
}
