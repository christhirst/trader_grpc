use apca::data::v2::stream::{drive, Bar, Data, MarketData, Quote, RealtimeData, Trade, IEX};
use apca::{ApiInfo, Client, Error};
use futures::{FutureExt, StreamExt, TryStreamExt};

use rand::Rng;
use tonic::transport::{Channel, Endpoint};
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod depot {
    tonic::include_proto!("depot");
}

mod settings;

use depot::depot_client::DepotClient;
use depot::{BuyRequest, SellRequest};
use settings::Settings;

async fn trader(mut client: DepotClient<Channel>, data: Data<Bar, Quote, Trade>) {
    info!("Received data: {:?}", data);
    match data {
        Data::Trade(trade) => {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.1) {
                let symbol = trade.symbol;
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
                    match client.buy_shares(req).await {
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
                    match client.sell_shares(req).await {
                        Ok(res) => info!("Sell Response: {:?}", res.into_inner()),
                        Err(e) => error!("Sell RPC error: {:?}", e),
                    }
                }
            }
        }
        Data::Bar(_) => {
            todo!()
        }
        Data::Quote(_) => {
            todo!()
        }
        _ => {}
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration
    let settings = Settings::new()?;
    let depot_url = settings.depot_url.clone();
    let api_key = settings.api_key_id;
    let api_secret = settings.api_secret_key;
    let api_base = settings.api_base_url;

    info!("Starting trader with API Base: {}", api_base);

    // Connect to Depot
    info!("Connecting to Depot at {}", depot_url);
    let mut depot_client = DepotClient::connect(depot_url).await?;
    info!("Connected to Depot");

    // Setup Alpaca
    info!("Connecting to Alpaca");
    let api_info = ApiInfo::from_parts(api_base, api_key, api_secret)?;
    let client = Client::new(api_info);
    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await?;
    info!("Connected to Alpaca");

    let symbol = "AAPL";
    let mut market_data = MarketData::default();
    market_data.set_trades([symbol]);
    market_data.set_quotes([symbol]);

    info!("Subscribing to {}", symbol);
    let subscribe = subscription.subscribe(&market_data).boxed();

    // Actually subscribe with the websocket server.
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    info!("Stream started...");

    let () = stream
        // Stop after receiving 50 updates.
        .map_err(Error::WebSocket)
        .try_for_each(|result| {
            let client = depot_client.clone();
            async move {
                match result {
                    Ok(data) => {
                        trader(client, data).await;
                        Ok(())
                    }
                    Err(e) => Err(Error::Json(e)),
                }
            }
        })
        .await
        .unwrap();

    Ok(())
}
