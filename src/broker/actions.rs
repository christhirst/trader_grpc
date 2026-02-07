use apca::{
    api,
    data::v2::{bars::Bar, stream::Trade},
    ApiInfo, Client,
};
use depot::{
    depot_client::DepotClient, BuyRequest, DepositRequest, Empty, SellRequest, StockRequest,
};
use tonic::transport::Channel;
use tracing::{error, info};
use trader_bot::grpc_depot::init::depot;
use trader_bot::indicator_client::init::calculate::{
    indicator_client::IndicatorClient, IndicatorType, ListNumbersRequest2, Opt,
};

use crate::settings;

#[derive(Debug, Clone)]
pub struct Alpaca {
    pub client: DepotClient<Channel>,
    pub account: std::sync::Arc<Client>,
}

impl Alpaca {
    pub async fn new() -> Self {
        let conf = settings::Settings::new().unwrap();
        let depot_url = conf.depot_url.clone();
        let api_base = conf.api_base_url.clone();
        let api_key = conf.api_key_id.clone();
        let api_secret = conf.api_secret_key.clone();
        let client = DepotClient::connect(depot_url).await.unwrap();

        let api_info = ApiInfo::from_parts(api_base, api_key, api_secret).unwrap();
        let account = Client::new(api_info);
        Self {
            client,
            account: std::sync::Arc::new(account),
        }
    }
    pub async fn buy(&mut self, req: BuyRequest) {
        let c = &mut self.client;

        match c.buy_shares(req).await {
            Ok(res) => _ = res.into_inner(), //info!("Buy Response: {:?}", res.into_inner()),
            Err(e) => error!("Buy RPC error: {:?}", e),
        }
    }
    pub async fn sell(&mut self, req: SellRequest) {
        let c = &mut self.client;
        match c.sell_shares(req).await {
            Ok(res) => _ = res.into_inner(), //info!("Sell Response: {:?}", res.into_inner()),
            Err(e) => error!("Sell RPC error: {:?}", e),
        }
    }
    pub async fn get_position(&mut self, symbol: &str) -> Option<i32> {
        let c = &mut self.client;
        let req = StockRequest {
            symbol: symbol.to_string(),
        };
        match c.get_share_balance(req).await {
            Ok(res) => {
                let shares = res.into_inner().shares;
                shares.iter().find(|s| s.symbol == symbol).map(|s| s.count)
            }
            Err(e) => {
                error!("Get position RPC error: {:?}", e);
                None
            }
        }
    }

    pub async fn reset_stock(&mut self, symbol: &str) {
        let c = &mut self.client;
        let req = StockRequest {
            symbol: symbol.to_string(),
        };
        match c.reset_stock(req).await {
            Ok(_) => info!("Reset stock for {} successful", symbol),
            Err(e) => error!("Reset stock RPC error: {:?}", e),
        }
    }

    pub async fn reset_cash(&mut self) {
        let c = &mut self.client;
        let req = Empty {};
        match c.reset_cash(req).await {
            Ok(_) => info!("Reset cash successful"),
            Err(e) => error!("Reset cash RPC error: {:?}", e),
        }
    }

    pub async fn deposit(&mut self, amount: f64) {
        let c = &mut self.client;
        let req = DepositRequest { amount };
        match c.deposit(req).await {
            Ok(_) => info!("Deposit of {} successful", amount),
            Err(e) => error!("Deposit RPC error: {:?}", e),
        }
    }

    pub async fn get_gain(&mut self) -> Option<f64> {
        let c = &mut self.client;
        let req = Empty {};
        match c.get_gain(req).await {
            Ok(res) => Some(res.into_inner().gain),
            Err(e) => {
                error!("Get gain RPC error: {:?}", e);
                None
            }
        }
    }

    pub async fn get_cash_balance(&mut self) -> Option<f64> {
        let c = &mut self.client;
        let req = Empty {};
        match c.get_state(req).await {
            Ok(res) => Some(res.into_inner().cash),
            Err(e) => {
                error!("Get cash balance RPC error: {:?}", e);
                None
            }
        }
    }

    pub async fn get_portfolio_value(&mut self) -> Option<f64> {
        let c = &mut self.client;
        let req = Empty {};
        match c.get_state(req).await {
            Ok(res) => {
                let state = res.into_inner();
                let cash = state.cash;

                // Calculate total value of all positions
                let positions_value: f64 = state
                    .shares
                    .iter()
                    .map(|share| (share.count as f64) * share.price_per_share)
                    .sum();

                Some(cash + positions_value)
            }
            Err(e) => {
                error!("Get portfolio value RPC error: {:?}", e);
                None
            }
        }
    }

    pub async fn eval_bar(&mut self, _b: Bar) {}
    pub async fn eval_trade(&mut self, _t: Vec<Trade>) {}
}
