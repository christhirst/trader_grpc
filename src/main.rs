use apca::data::v2::stream::{drive, Bar, Data, MarketData, Quote, RealtimeData, Trade, IEX};
use apca::{data, ApiInfo, Client, Error};
use futures::{FutureExt, StreamExt, TryStreamExt};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
mod broker;
mod decisions;

mod settings;

use depot::depot_client::DepotClient;
use settings::Settings;
use trader_bot::grpc_depot::init::depot;

use crate::broker::actions::Alpaca;

fn buffer<T>(size: usize, data: T) -> Vec<T> {
    let mut buf = vec![];

    if buf.len() >= size {
        buf.pop();
    }
    buf.push(data);

    buf
}

struct Actor {}

impl Actor {
    pub fn new() -> Self {
        Self {}
    }

    async fn trader(&self, client: &mut Alpaca, data: Data<Bar, Quote, Trade>) {
        info!("Received data: {:?}", data);
        match data {
            Data::Trade(trade) => {
                info!("Received trade: {:?}", trade);
                let data = buffer(100, trade.clone());
                client.eval_trade(data).await;
            }
            Data::Bar(_) => {
                print!("test");
            }
            Data::Quote(_) => {
                print!("test");
            }
            _ => {}
        }
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
    let indicator_url = settings.indicator_url.clone();
    let api_key = settings.api_key_id.clone();
    let api_secret = settings.api_secret_key.clone();
    let api_base = settings.api_base_url.clone();
    info!("Starting trader with API Base: {}", api_base);

    // Connect to Depot
    info!("Connecting to Depot at {}", depot_url);
    let depot_client = DepotClient::connect(depot_url).await?;

    // Setup Alpaca
    info!("Connecting to Alpaca");
    //TODO put in function
    let api_info = ApiInfo::from_parts(api_base.clone(), api_key.clone(), api_secret.clone())?;
    let client_broker = Client::new(api_info);
    let (mut stream, mut subscription) = client_broker
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await?;
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

    // Initialize Alpaca with all clients
    let indicator_url = settings.indicator_url.clone();
    let client = std::sync::Arc::new(Alpaca::new().await);

    info!("Connected to Alpaca");

    info!("Stream started...");

    let actor = std::sync::Arc::new(Actor::new());

    let () = stream
        // Stop after receiving 50 updates.
        .map_err(Error::WebSocket)
        .try_for_each(move |result| {
            let client = client.clone();
            let actor = actor.clone();
            async move {
                match result {
                    Ok(data) => {
                        let mut client_clone = (*client).clone();
                        actor.trader(&mut client_clone, data).await;
                        Ok::<(), apca::Error>(())
                    }
                    Err(e) => Err(Error::Json(e)),
                }
            }
        })
        .await
        .unwrap();

    Ok(())
}
