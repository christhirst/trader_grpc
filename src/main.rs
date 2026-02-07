use std::collections::HashMap;

use apca::data::v2::stream::{drive, Bar, Data, MarketData, Quote, RealtimeData, Trade, IEX};
use apca::{data, ApiInfo, Client, Error};
use futures::{FutureExt, TryStreamExt};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
mod broker;
mod decisions;
mod error;
mod mocking;
mod pattern;
mod wrangling;

mod db;
mod settings; // Added module declaration

use depot::depot_client::DepotClient;
use settings::Settings;
use trader_bot::grpc_depot::init::depot;

use crate::broker::actions::Alpaca;
use crate::broker::evaluator::Evaluator;
use crate::db::Db; // Added import
use crate::wrangling::buffers::Buffer;

fn buffer<T>(size: usize, data: T, _symbol: &str) -> Vec<T> {
    let mut buf = vec![];

    if buf.len() >= size {
        buf.pop();
    }
    buf.push(data);

    buf
}

struct Actor {
    evaluator: Evaluator,
}

impl Actor {
    pub async fn new() -> Self {
        Self {
            evaluator: Evaluator::new().await,
        }
    }

    async fn trader(&mut self, client: &mut Alpaca, data: Data<Bar, Quote, Trade>) {
        //info!("Received data: {:?}", data);
        match data {
            Data::Trade(trade) => {
                let symbol = &trade.symbol;
                //info!("Received trade: {:?}", &trade);
                let data = buffer(100, trade.clone(), symbol);

                //client.eval_trade(data).await;
            }
            Data::Bar(bar) => {
                let symbol = &bar.symbol;
                let buf = match self.evaluator.buffer.get_mut(symbol) {
                    Some(s) => s,
                    None => {
                        info!("New symbol: {}", symbol);
                        self.evaluator
                            .buffer
                            .insert(symbol.to_string(), Buffer::new(symbol.to_string(), 100));
                        self.evaluator.buffer.get_mut(symbol).unwrap()
                    }
                };
                //add bars to buffer
                buf.add_bar(bar.clone());
                /* info!(
                    "New se: {:?}",
                    self.evaluator.buffer.get(symbol).unwrap().bar_count()
                ); */
                if self.evaluator.buffer.get(symbol).unwrap().bar_count() >= 20 {
                    self.evaluator
                        .eval_bars(symbol, bar.close_price.to_f64().unwrap())
                        .await;
                }
            }
            Data::Quote(_) => {}
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let settings = Settings::new()?;
    let depot_url = settings.depot_url.clone();
    let api_key = settings.api_key_id.clone();
    let api_secret = settings.api_secret_key.clone();
    let api_base = settings.api_base_url.clone();
    info!("Starting trader with API Base: {}", api_base);

    // Connect to Depot
    info!("Connecting to Depot at {}", depot_url);
    let _depot_client = DepotClient::connect(depot_url).await?;

    // Setup Alpaca
    info!("Connecting to Alpaca");
    //TODO put in function
    let api_info = ApiInfo::from_parts(api_base.clone(), api_key.clone(), api_secret.clone())?;
    let client_broker = Client::new(api_info);

    let (mut stream, mut subscription) = client_broker
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await?;
    let symbols = ["FAKEPACA"];
    info!("Subscribing to {:?}", symbols);
    let mut market_data = MarketData::default();
    market_data.set_trades(symbols);
    market_data.set_quotes(["FAKEPACA"]);

    info!("Subscribing to {:?}", symbols);
    let subscribe = subscription.subscribe(&market_data).boxed();

    // Actually subscribe with the websocket server.
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // Initialize Alpaca with all clients
    let client = std::sync::Arc::new(Alpaca::new().await);
    let actor = std::sync::Arc::new(tokio::sync::Mutex::new(Actor::new().await));

    if settings.use_mock_data {
        info!("Using mock data from {}", settings.mock_file_path);
        let eval_iterations = settings.eval_iterations;
        let top_n = settings.top_n_configs;
        let mut reset_client = (*client).clone();
        let symbol = "ORCL";

        // Initialize DB
        let db: Option<Db> = match Db::new().await {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::error!("Failed to connect to DB: {:?}", e);
                None
            }
        };

        for i in 0..eval_iterations {
            info!("Starting evaluation run {}/{}", i + 1, eval_iterations);

            // 1. Generate Config
            let config = crate::broker::evaluator::IndicatorConfig::random();
            info!("Generated config: {:#?}", config);
            let cash = 100000.0;
            let eval_config = crate::broker::evaluator::EvalConfig {
                cash,
                indicator_config: config.clone(),
            };

            // 2. Update Actor
            {
                let mut actor_guard = actor.lock().await;
                actor_guard.evaluator.eval_config = Some(eval_config);
                actor_guard.evaluator.buffer.clear();
            }

            // 3. Reset Environment
            reset_client.reset_cash().await;
            reset_client.reset_stock(symbol).await;
            reset_client.deposit(cash).await;

            // 4. Run Stream
            let stream =
                crate::mocking::mock::mock_data_stream(&settings.mock_file_path, symbol, 0).await?;

            let client_for_stream = client.clone();
            let actor_for_stream = actor.clone();

            stream
                .try_for_each(move |data| {
                    let client = client_for_stream.clone();
                    let actor = actor_for_stream.clone();
                    async move {
                        let mut client_clone = (*client).clone();
                        let mut actor_guard = actor.lock().await;
                        actor_guard.trader(&mut client_clone, data).await;
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    }
                })
                .await?;

            // 5. Sell remaining shares
            let last_price_opt = {
                let actor_guard = actor.lock().await;
                actor_guard.evaluator.buffer.get(symbol).and_then(|buf| {
                    buf.get_bars()
                        .unwrap()
                        .last()
                        .map(|b| b.close_price.to_f64().unwrap())
                })
            };

            if let Some(last_price) = last_price_opt {
                let shares = reset_client.get_position(symbol).await.unwrap_or(0);
                if shares > 0 {
                    info!("Selling remaining {} shares at {}", shares, last_price);
                    let req = depot::SellRequest {
                        symbol: symbol.to_string(),
                        count: shares,
                        price_per_share: last_price,
                    };
                    reset_client.sell(req).await;
                }
            }

            // 6. Evaluate
            if let Some(gain) = reset_client.get_gain().await {
                info!("Run {} result: Gain = {}", i + 1, gain);
                let mut actor_guard = actor.lock().await;
                actor_guard
                    .evaluator
                    .update_best_configs(gain as i32, config.clone(), top_n);

                // 7. Save to DB
                if let Some(db) = &db {
                    if let Some(sma_config) = config.sma.clone() {
                        let res = crate::db::RunResult {
                            id: None,
                            config: sma_config,
                            symbol: symbol.to_string(),
                            gain,
                            timestamp: chrono::Utc::now(),
                        };
                        if let Err(e) = db.add_result(res).await {
                            tracing::error!("Failed to save result to DB: {:?}", e);
                        } else {
                            info!("Saved result to DB");
                        }
                    }
                }
            } else {
                tracing::error!("Failed to get gain for run {}", i + 1);
            }
        }

        let actor_guard = actor.lock().await;
        info!("Best configurations:");
        for (gain, config) in &actor_guard.evaluator.best_eval_config {
            info!("Gain: {}, Config: {:?}", gain, config);
        }
    } else {
        // Setup Alpaca
        info!("Connecting to Alpaca");
        let api_info = ApiInfo::from_parts(api_base.clone(), api_key.clone(), api_secret.clone())?;
        let client_broker = Client::new(api_info);
        let (mut stream, mut subscription) = client_broker
            .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
            .await?;
        let symbols = ["FAKEPACA"];
        let mut market_data = MarketData::default();
        market_data.set_trades(symbols);
        market_data.set_quotes(["FAKEPACA"]);

        info!("Subscribing to {:?}", symbols);
        let subscribe = subscription.subscribe(&market_data).boxed();

        // Actually subscribe with the websocket server.
        let () = drive(subscribe, &mut stream)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        info!("Connected to Alpaca");
        info!("Stream started...");

        let () = stream
            .map_err(Error::WebSocket)
            .try_for_each(move |result| {
                let client = client.clone();
                let actor = actor.clone();
                async move {
                    match result {
                        Ok(data) => {
                            let mut client_clone = (*client).clone();
                            let mut actor_guard = actor.lock().await;
                            actor_guard.trader(&mut client_clone, data).await;
                            Ok::<(), apca::Error>(())
                        }
                        Err(e) => Err(Error::Json(e)),
                    }
                }
            })
            .await
            .unwrap();
    }

    Ok(())
}
