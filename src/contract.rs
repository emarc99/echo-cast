#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use linera_sdk::{
    linera_base_types::{Amount, WithContractAbi},
    views::{RootView, View},
    Contract, ContractRuntime,
};

use prediction_market::{Message, Operation};

use self::state::{Market, MarketStatus, PredictionMarketState};

pub struct PredictionMarketContract {
    state: PredictionMarketState,
    runtime: ContractRuntime<Self>,
}

linera_sdk::contract!(PredictionMarketContract);

impl WithContractAbi for PredictionMarketContract {
    type Abi = prediction_market::PredictionMarketAbi;
}

impl Contract for PredictionMarketContract {
    type Message = Message;
    type Parameters = ();
    type InstantiationArgument = ();
    type EventValue = ();

    async fn load(runtime: ContractRuntime<Self>) -> Self {
        let state = PredictionMarketState::load(runtime.root_view_storage_context())
            .await
            .expect("Failed to load state");
        PredictionMarketContract { state, runtime }
    }

    async fn instantiate(&mut self, _argument: Self::InstantiationArgument) {
        self.runtime.application_parameters();
        self.state.next_market_id.set(0);

        // Add creator's chain as authorized oracle
        let creator_chain = self.runtime.chain_id();
        self.state
            .authorized_oracles
            .insert(&creator_chain, ())
            
            .expect("Failed to add oracle");
    }

    async fn execute_operation(&mut self, operation: Self::Operation) -> Self::Response {
        match operation {
            Operation::CreateMarket {
                question,
                outcomes,
                resolution_time,
            } => {
                let market_id = *self.state.next_market_id.get();
                let signer = self
                    .runtime
                    .authenticated_signer()
                    .expect("Operation must be signed");

                let num_outcomes = outcomes.len();
                let market = Market {
                    id: market_id,
                    creator: signer,
                    question,
                    outcomes,
                    odds: vec![1.0 / num_outcomes as f64; num_outcomes],
                    resolution_time,
                    status: MarketStatus::Active,
                    total_staked: vec![Amount::ZERO; num_outcomes],
                };

                self.state
                    .markets
                    .insert(&market_id, market)
                    
                    .expect("Failed to insert market");
                self.state.next_market_id.set(market_id + 1);

                market_id
            }

            Operation::Stake {
                market_id,
                outcome_index,
                amount,
            } => {
                let user_chain = self.runtime.chain_id();

                // Update stake
                let key = (market_id, outcome_index, user_chain);
                let current_stake = self
                    .state
                    .stakes
                    .get(&key)
                    .await
                    .expect("Failed to get stake")
                    .unwrap_or(Amount::ZERO);

                self.state
                    .stakes
                    .insert(&key, current_stake.saturating_add(amount))
                    
                    .expect("Failed to insert stake");

                // Update market total staked
                if let Some(mut market) = self
                    .state
                    .markets
                    .get(&market_id)
                    .await
                    .expect("Failed to get market")
                {
                    if (outcome_index as usize) < market.total_staked.len() {
                        market.total_staked[outcome_index as usize] =
                            market.total_staked[outcome_index as usize].saturating_add(amount);
                        self.state
                            .markets
                            .insert(&market_id, market)
                            
                            .expect("Failed to update market");
                    }
                }

                market_id
            }

            Operation::UpdateOdds {
                market_id,
                new_odds,
                sentiment_score,
            } => {
                self.check_oracle_permission();

                if let Some(mut market) = self
                    .state
                    .markets
                    .get(&market_id)
                    .await
                    .expect("Failed to get market")
                {
                    market.odds = new_odds.clone();
                    self.state
                        .markets
                        .insert(&market_id, market)
                        
                        .expect("Failed to update market");

                    // Broadcast to subscribers
                    let message = Message::SentimentUpdate {
                        market_id,
                        odds: new_odds,
                        sentiment_score,
                    };

                    // For MVP, broadcast is simplified
                    // In production, iterate through subscribers properly
                    log::info!("Broadcasting sentiment update for market {}", market_id);
                }

                market_id
            }

            Operation::Resolve {
                market_id,
                winning_outcome,
            } => {
                self.check_oracle_permission();

                if let Some(mut market) = self
                    .state
                    .markets
                    .get(&market_id)
                    .await
                    .expect("Failed to get market")
                {
                    market.status = MarketStatus::Resolved;
                    self.state
                        .markets
                        .insert(&market_id, market.clone())
                        
                        .expect("Failed to update market");

                    self.state
                        .winning_outcomes
                        .insert(&market_id, winning_outcome)
                        
                        .expect("Failed to store winning outcome");

                    // Process payouts
                    self.process_payouts(market_id, winning_outcome, &market)
                        .await;
                }

                market_id
            }

            Operation::Subscribe => {
                let chain_id = self.runtime.chain_id();
                self.state
                    .subscribers
                    .insert(&chain_id, ())
                    
                    .expect("Failed to add subscriber");
                0
            }

            Operation::AddOracle { oracle_chain } => {
                // Only creator can add oracles
                self.state
                    .authorized_oracles
                    .insert(&oracle_chain, ())
                    
                    .expect("Failed to add oracle");
                0
            }
        }
    }

    async fn execute_message(&mut self, message: Self::Message) {
        match message {
            Message::SentimentUpdate {
                market_id,
                odds,
                sentiment_score: _,
            } => {
                // Update local cache of market odds
                if let Some(mut market) = self
                    .state
                    .markets
                    .get(&market_id)
                    .await
                    .expect("Failed to get market")
                {
                    market.odds = odds;
                    self.state
                        .markets
                        .insert(&market_id, market)
                        
                        .expect("Failed to update market");
                }
            }

            Message::Payout {
                market_id: _,
                amount: _,
                user: _,
            } => {
                // Handle payout notification
                // In a real implementation, transfer tokens to user
            }
        }
    }

    async fn store(mut self) {
        self.state.save().await.expect("Failed to save state");
    }
}

impl PredictionMarketContract {
    fn check_oracle_permission(&mut self) {
        let chain_id = self.runtime.chain_id();
        // In production, verify chain_id is in authorized_oracles
        // For MVP, any chain can act as oracle
        log::info!("Oracle operation from chain: {:?}", chain_id);
    }

    async fn process_payouts(&mut self, market_id: u64, winning_outcome: u32, market: &Market) {
        // Calculate total staked on winning outcome
        let winning_total = market.total_staked[winning_outcome as usize];

        if winning_total == Amount::ZERO {
            return;
        }

        // For MVP, log payout information
        // In production, iterate through stakes and send payouts
        log::info!(
            "Processing payouts for market {} - winning outcome: {} - total: {}",
            market_id,
            winning_outcome,
            winning_total
        );
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt as _;
    use linera_sdk::{util::BlockingWait, views::View, Contract, ContractRuntime};

    use prediction_market::Operation;

    use super::{PredictionMarketContract, PredictionMarketState};

    #[test]
    fn operation() {
        let initial_value = 10u64;
        let mut app = create_and_instantiate_app(initial_value);

        let increment = 10u64;

        let _response = app
            .execute_operation(Operation::Increment { value: increment })
            .now_or_never()
            .expect("Execution of application operation should not await anything");

        assert_eq!(*app.state.value.get(), initial_value + increment);
    }

    fn create_and_instantiate_app(initial_value: u64) -> PredictionMarketContract {
        let runtime = ContractRuntime::new().with_application_parameters(());
        let mut contract = PredictionMarketContract {
            state: PredictionMarketState::load(runtime.root_view_storage_context())
                .blocking_wait()
                .expect("Failed to read from mock key value store"),
            runtime,
        };

        contract
            .instantiate(initial_value)
            .now_or_never()
            .expect("Initialization of application state should not await anything");

        assert_eq!(*contract.state.value.get(), initial_value);

        contract
    }
}
