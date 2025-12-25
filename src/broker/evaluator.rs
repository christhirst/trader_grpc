use apca::data::v2::{bars::Bar, stream::Trade};
use rand::Rng;
use tonic::transport::{Channel, Error};
use tracing::info;
use trader_bot::indicator_client::init::calculate::{
    indicator_client::IndicatorClient, IndicatorType, ListNumbersRequest2,
};

use crate::{
    broker::actions::Alpaca,
    depot::{BuyRequest, SellRequest},
    settings::Settings,
};

struct Evaluator {
    ap: Alpaca,
    pub indicator_client: IndicatorClient<Channel>,
}

fn gen_sell() -> (f64, i32) {
    let mut rng = rand::thread_rng();
    let price = rng.gen_range(100.0..=200.0);
    let count = rng.gen_range(1..=10);
    (price, count)
}

impl Evaluator {
    pub async fn new() -> Self {
        let settings = Settings::new().unwrap();
        let _depot_url = settings.depot_url.clone();
        let indicator_url = settings.indicator_url.clone();
        let api_key = settings.api_key_id;
        let api_secret = settings.api_secret_key;
        let api_base = settings.api_base_url;
        let ap = Alpaca::new().await;

        let indicator_client = IndicatorClient::connect(indicator_url.to_string())
            .await
            .unwrap();

        Self {
            ap,
            indicator_client,
        }
    }

    pub async fn eval_bars(&mut self, b: Vec<Bar>) {
        //TODO convert bar data to f64 list for grpc call on close prices
        let bars_to_f64 = b
            .iter()
            .map(|bar| bar.close.to_f64().unwrap_or(0.0))
            .collect::<Vec<f64>>();
        let i = &mut self.indicator_client;
        let request = ListNumbersRequest2 {
            id: IndicatorType::BollingerBands as i32,
            opt: None,
            list: bars_to_f64,
        };
        let ic = i.gen_liste(request).await;

        let mut rng = rand::thread_rng();
        let (price, count) = gen_sell();
        if rng.gen_bool(0.1) {
            let req = SellRequest {
                symbol: "AAPL".to_string(),
                count,
                price_per_share: price,
            };
            self.ap.sell(req).await;
        }
    }

    pub async fn eval_trade(&mut self, t: Trade) -> Result<f64, Error> {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.1) {
            let symbol = t.symbol;
            let price = t.trade_price.to_f64().unwrap_or(0.0);
            let count = 1;

            if rng.gen_bool(0.5) {
                // Buy
                info!("Decided to BUY {} shares of {} at {}", count, symbol, price);
                let _req = BuyRequest {
                    symbol: symbol.to_string(),
                    count,
                    price_per_share: price,
                };
            } else {
                // Sell
                info!(
                    "Decided to SELL {} shares of {} at {}",
                    count, symbol, price
                );
                let _req = SellRequest {
                    symbol: symbol.to_string(),
                    count,
                    price_per_share: price,
                };
            }
        }
        Ok(0.1)
    }
}
