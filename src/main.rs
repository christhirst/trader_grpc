use apca::data::v2::stream::{Data, IEX, MarketData, RealtimeData};
use apca::{ApiInfo, Client};
use futures::StreamExt;
use num_traits::ToPrimitive;
use rand::Rng;
use std::env;
use tonic::transport::Channel;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

pub mod depot {
    tonic::include_proto!("depot");
}

use depot::depot_client::DepotClient;
use depot::{BuyRequest, SellRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let api_key = env::var("APCA_API_KEY_ID").map_err(|_| "APCA_API_KEY_ID not set")?;
    let api_secret = env::var("APCA_API_SECRET_KEY").map_err(|_| "APCA_API_SECRET_KEY not set")?;
    let api_base = env::var("APCA_API_BASE_URL")
        .unwrap_or_else(|_| "https://paper-api.alpaca.markets".to_string());

    info!("Starting trader with API Base: {}", api_base);

    // Connect to Depot
    let depot_url = "http://[::1]:50051";
    info!("Connecting to Depot at {}", depot_url);
    let channel = Channel::from_static(depot_url).connect().await?;
    let mut depot_client = DepotClient::new(channel);
    info!("Connected to Depot");

    // Setup Alpaca
    let api_info = ApiInfo::from_parts(api_base, api_key, api_secret)?;
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await?;

    let symbol = "AAPL";
    let mut market_data = MarketData::default();
    market_data.set_trades([symbol]);
    market_data.set_quotes([symbol]);

    info!("Subscribing to {}", symbol);
    if let Err(e) = subscription.subscribe(&market_data).await {
        error!("Subscription failed: {:?}", e);
        return Ok(());
    }

    info!("Stream started...");

    while let Some(msg_res) = stream.next().await {
        match msg_res {
            Ok(Ok(data)) => {
                match data {
                    Data::Trade(trade) => {
                        info!(
                            "Trade: {:.2} (Size: {})",
                            trade.trade_price, trade.trade_size
                        );

                        let mut rng = rand::thread_rng();
                        if rng.gen_bool(0.1) {
                            let price = trade.trade_price.to_f64().unwrap_or(0.0);
                            let count = 1;

                            if rng.gen_bool(0.5) {
                                // Buy
                                info!("Decided to BUY {} shares of {} at {}", count, symbol, price);
                                let req = BuyRequest {
                                    symbol: symbol.to_string(),
                                    count,
                                    price_per_share: price,
                                };
                                match depot_client.buy_shares(req).await {
                                    Ok(res) => info!("Buy Response: {:?}", res.into_inner()),
                                    Err(e) => error!("Buy RPC error: {:?}", e),
                                }
                            } else {
                                // Sell
                                info!(
                                    "Decided to SELL {} shares of {} at {}",
                                    count, symbol, price
                                );
                                let req = SellRequest {
                                    symbol: symbol.to_string(),
                                    count,
                                    price_per_share: price,
                                };
                                match depot_client.sell_shares(req).await {
                                    Ok(res) => info!("Sell Response: {:?}", res.into_inner()),
                                    Err(e) => error!("Sell RPC error: {:?}", e),
                                }
                            }
                        }
                    }
                    Data::Quote(_quote) => {
                        // info!("Quote received");
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => error!("JSON Stream error: {:?}", e),
            Err(e) => error!("WebSocket Stream error: {:?}", e),
        }
    }

    Ok(())
}
