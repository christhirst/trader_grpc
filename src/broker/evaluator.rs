use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use apca::data::v2::stream::Trade;
use rand::Rng;
use tonic::transport::{Channel, Error};
use tracing::info;
use trader_bot::indicator_client::init::calculate::{
    indicator_client::IndicatorClient, IndicatorType, ListNumbersRequest2, Opt,
};

use crate::{
    broker::actions::Alpaca,
    broker::position_sizing::PositionSizer,
    depot::{BuyRequest, SellRequest},
    pattern::cross_gc_dc,
    settings::Settings,
    wrangling::buffers::Buffer,
};

pub struct Evaluator {
    pub ap: Alpaca,
    pub indicator_client: IndicatorClient<Channel>,
    pub buffer: HashMap<String, Buffer>,
    pub eval_config: Option<EvalConfig>,
    pub best_eval_config: HashMap<i32, IndicatorConfig>,
    pub position_sizer: PositionSizer,
}

pub struct EvalConfig {
    pub cash: f64,
    pub indicator_config: IndicatorConfig,
}

#[derive(Debug, Clone)]
pub struct IndicatorConfig {
    pub sma: Option<SMAConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SMAConfig {
    pub long_range: i32,
    pub short_range: i32,
}

impl IndicatorConfig {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let long_range = rng.random_range(10..=50);
        let short_range = rng.random_range(2..=long_range - 5); // Ensure short < long

        IndicatorConfig {
            sma: Some(SMAConfig {
                long_range,
                short_range,
            }),
        }
    }
}

fn gen_sell() -> (f64, i32) {
    let mut rng = rand::rng();
    let price = rng.random_range(100.0..=200.0);
    let count = rng.random_range(1..=10);
    (price, count)
}

fn gen_buy() -> (f64, i32) {
    let mut rng = rand::rng();
    let price = rng.random_range(100.0..=200.0);
    let count = rng.random_range(1..=10);
    (price, count)
}

impl Evaluator {
    pub async fn new() -> Self {
        let settings = Settings::new().unwrap();
        let _depot_url = settings.depot_url.clone();
        let indicator_url = settings.indicator_url.clone();
        let ap = Alpaca::new().await;

        let indicator_client = IndicatorClient::connect(indicator_url.to_string())
            .await
            .unwrap();

        let position_sizer =
            PositionSizer::new(settings.max_trade_percent, settings.max_position_percent);

        Self {
            ap,
            indicator_client,
            buffer: HashMap::new(),
            eval_config: None,
            best_eval_config: HashMap::new(),
            position_sizer,
        }
    }

    pub fn update_best_configs(&mut self, gain: i32, config: IndicatorConfig, top_n: usize) {
        self.best_eval_config.insert(gain, config);

        // Keep top N
        if self.best_eval_config.len() > top_n {
            let mut keys: Vec<i32> = self.best_eval_config.keys().cloned().collect();
            keys.sort_unstable(); // Ascending order
                                  // Remove the smallest gains (first items) until we have top_n
            let to_remove = keys.len() - top_n;
            for k in keys.iter().take(to_remove) {
                self.best_eval_config.remove(k);
            }
        }
    }

    pub async fn eval_bars(&mut self, symbol: &str, current_price: f64) {
        let bars = self.buffer.get(symbol).unwrap().get_bars().unwrap();
        let bars_to_f64 = bars
            .iter()
            .map(|bar| bar.close_price.to_f64().unwrap_or(0.0))
            .collect::<Vec<f64>>();
        let i = &mut self.indicator_client;

        // Determine periods from config or defaults
        let (long_period, short_period) = if let Some(config) = &self.eval_config {
            if let Some(sma) = &config.indicator_config.sma {
                (sma.long_range as i64, sma.short_range as i64)
            } else {
                (bars_to_f64.len() as i64, 5)
            }
        } else {
            (bars_to_f64.len() as i64, 5)
        };

        if (bars_to_f64.len() as i64) < long_period {
            return;
        }

        let opt = Opt {
            multiplier: 2.0,
            period: long_period,
        };
        let request = ListNumbersRequest2 {
            id: IndicatorType::SimpleMovingAverage as i32,
            opt: Some(opt),
            list: bars_to_f64.clone(),
        };
        //grpc call
        let long_range = i.gen_liste(request).await.unwrap().get_ref().result.clone();

        /* info!("long_range: {:?}", long_range);
        info!("bars_to_f64: {:?}", bars_to_f64.len()); */

        let opt = Opt {
            multiplier: 2.0,
            period: short_period,
        };
        let request = ListNumbersRequest2 {
            id: IndicatorType::SimpleMovingAverage as i32,
            opt: Some(opt),
            list: bars_to_f64.clone(),
        };

        let short_range = i.gen_liste(request).await.unwrap().get_ref().result.clone();
        //info!("short_range: {:?}", short_range);
        let gc = cross_gc_dc::gc(
            (
                *long_range.get(long_range.len() - 2).unwrap(),
                *long_range.get(long_range.len() - 1).unwrap(),
            ),
            (
                *short_range.get(short_range.len() - 2).unwrap(),
                *short_range.get(short_range.len() - 1).unwrap(),
            ),
        );

        let dc = cross_gc_dc::dc(
            (
                *long_range.get(long_range.len() - 2).unwrap(),
                *long_range.get(long_range.len() - 1).unwrap(),
            ),
            (
                *short_range.get(short_range.len() - 2).unwrap(),
                *short_range.get(short_range.len() - 1).unwrap(),
            ),
        );
        if let Some(gc) = gc {
            if gc {
                let portfolio_value = self.ap.get_portfolio_value().await.unwrap_or(0.0);
                let cash_available = self.ap.get_cash_balance().await.unwrap_or(0.0);
                let current_position = self.ap.get_position(symbol).await.unwrap_or(0);
                let current_position_value = (current_position as f64) * current_price;

                let shares_to_buy = self.position_sizer.calculate_buy_size(
                    current_price,
                    portfolio_value,
                    current_position_value,
                    cash_available,
                );

                if shares_to_buy > 0 {
                    let req = BuyRequest {
                        symbol: symbol.to_string(),
                        count: shares_to_buy,
                        price_per_share: current_price,
                    };
                    self.ap.buy(req).await;
                    info!(
                        "Golden Cross: Buying {} shares at {}",
                        shares_to_buy, current_price
                    );
                }
            }
        }

        if let Some(dc) = dc {
            if dc {
                let current_position = self.ap.get_position(symbol).await.unwrap_or(0);

                if current_position > 0 {
                    let portfolio_value = self.ap.get_portfolio_value().await.unwrap_or(0.0);
                    let shares_to_sell = self.position_sizer.calculate_sell_size(
                        current_position,
                        current_price,
                        portfolio_value,
                    );

                    if shares_to_sell > 0 {
                        let req = SellRequest {
                            symbol: symbol.to_string(),
                            count: shares_to_sell,
                            price_per_share: current_price,
                        };
                        self.ap.sell(req).await;
                        info!(
                            "Death Cross: Selling {} shares at {}",
                            shares_to_sell, current_price
                        );
                    }
                } else {
                    let portfolio_value = self.ap.get_portfolio_value().await.unwrap_or(0.0);
                    let shares_to_short = self.position_sizer.calculate_short_size(
                        current_price,
                        portfolio_value,
                        current_position,
                    );

                    if shares_to_short > 0 {
                        let req = SellRequest {
                            symbol: symbol.to_string(),
                            count: shares_to_short,
                            price_per_share: current_price,
                        };
                        self.ap.sell(req).await;
                        info!(
                            "Death Cross: Shorting {} shares at {}",
                            shares_to_short, current_price
                        );
                    }
                }
            }
        }
    }

    pub async fn eval_trade(&mut self, t: Trade) -> Result<f64, Error> {
        let mut rng = rand::rng();
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

#[cfg(test)]
mod tests {
    use crate::mocking::mock::data_stream_from_csv;

    use super::*;
    use apca::data::v2::stream::Data;
    use futures::StreamExt;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_data_stream_from_csv() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Read from actual orcl.csv file
        let path = "files/orcl.csv";
        let symbol = "ORCL";
        let stream = data_stream_from_csv(path, &symbol).await?;
        tokio::pin!(stream);

        let mut evaluator = Evaluator::new().await;

        // Get the first item from the stream
        loop {
            if let Some(result) = stream.next().await {
                let data = result.unwrap();
                match data {
                    Data::Bar(bar) => {
                        evaluator
                            .eval_bars(symbol, bar.close_price.to_f64().unwrap())
                            .await;
                    }
                    _ => panic!("Expected Data::Bar"),
                }
            } else {
                panic!("Stream was empty");
            }
        }
        Ok(())
    }
}
