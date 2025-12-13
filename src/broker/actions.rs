use crate::depot::{depot_client::DepotClient, BuyRequest, SellRequest};
use apca::Client;
use tonic::transport::Channel;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct Alpaca {
    pub client: DepotClient<Channel>,
    pub account: std::sync::Arc<Client>,
}

impl Alpaca {
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
}
