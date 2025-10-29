#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use std::sync::Arc;

use async_graphql::{EmptySubscription, Object, Schema};
use linera_sdk::{
    graphql::GraphQLMutationRoot,
    linera_base_types::WithServiceAbi,
    views::View,
    Service, ServiceRuntime,
};

use prediction_market::Operation;

use self::state::PredictionMarketState;

pub struct PredictionMarketService {
    state: PredictionMarketState,
    runtime: Arc<ServiceRuntime<Self>>,
}

linera_sdk::service!(PredictionMarketService);

impl WithServiceAbi for PredictionMarketService {
    type Abi = prediction_market::PredictionMarketAbi;
}

impl Service for PredictionMarketService {
    type Parameters = ();

    async fn new(runtime: ServiceRuntime<Self>) -> Self {
        let state = PredictionMarketState::load(runtime.root_view_storage_context())
            .await
            .expect("Failed to load state");
        PredictionMarketService {
            state,
            runtime: Arc::new(runtime),
        }
    }

    async fn handle_query(&self, query: Self::Query) -> Self::QueryResponse {
        Schema::build(
            QueryRoot {
                next_market_id: *self.state.next_market_id.get(),
            },
            Operation::mutation_root(self.runtime.clone()),
            EmptySubscription,
        )
        .finish()
        .execute(query)
        .await
    }
}

struct QueryRoot {
    next_market_id: u64,
}

#[Object]
impl QueryRoot {
    /// Get the next market ID (total number of markets)
    async fn market_count(&self) -> u64 {
        self.next_market_id
    }
}
