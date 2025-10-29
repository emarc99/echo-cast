use linera_sdk::views::{linera_views, MapView, RegisterView, RootView, ViewStorageContext};
use linera_sdk::linera_base_types::{AccountOwner, Amount, ChainId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: u64,
    pub creator: AccountOwner,
    pub question: String,
    pub outcomes: Vec<String>,
    pub odds: Vec<f64>,
    pub resolution_time: Timestamp,
    pub status: MarketStatus,
    pub total_staked: Vec<Amount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Eq, PartialEq)]
pub enum MarketStatus {
    Active,
    Resolved,
    Cancelled,
}

#[derive(RootView)]
#[view(context = ViewStorageContext)]
pub struct PredictionMarketState {
    /// Next available market ID
    pub next_market_id: RegisterView<u64>,

    /// All markets by ID
    pub markets: MapView<u64, Market>,

    /// Stakes: (market_id, outcome_index, user_chain) -> amount
    pub stakes: MapView<(u64, u32, ChainId), Amount>,

    /// Subscribed chains receiving sentiment updates
    pub subscribers: MapView<ChainId, ()>,

    /// Authorized oracle chains that can update odds
    pub authorized_oracles: MapView<ChainId, ()>,

    /// Winning outcome for resolved markets
    pub winning_outcomes: MapView<u64, u32>,
}
