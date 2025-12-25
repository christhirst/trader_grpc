use apca::{
    api,
    data::v2::{bars::Bar, stream::Trade},
    ApiInfo, Client,
};
use depot::{depot_client::DepotClient, BuyRequest, SellRequest};
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
            Ok(res) => info!("Buy Response: {:?}", res.into_inner()),
            Err(e) => error!("Buy RPC error: {:?}", e),
        }
    }
    pub async fn sell(&mut self, req: SellRequest) {
        let c = &mut self.client;
        match c.sell_shares(req).await {
            Ok(res) => info!("Sell Response: {:?}", res.into_inner()),
            Err(e) => error!("Sell RPC error: {:?}", e),
        }
    }

    pub async fn eval_bar(&mut self, _b: Bar) {}
    pub async fn eval_trade(&mut self, _t: Vec<Trade>) {}
}
