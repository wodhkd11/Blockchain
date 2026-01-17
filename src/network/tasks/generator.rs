/*
use std::net::SocketAddr;
use std::sync::Arc;
use rand::Rng;
use hex;

use crate::block::transaction::TransactionData;
use crate::network::node::NodeManage;
use crate::network::message::NetworkMessage;

impl NodeManage{
    pub async fn start_transaction_generator(self:Arc<Self>){
        loop{
            let delay ={
                let mut rng = rand::thread_rng();
                rng.gen_range(2..=6)
            };
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            let tx:TransactionData = {
                let mut rng = rand::thread_rng();
                let mut sender = [0u8; 20];
                let mut receiver = [0u8; 20];
                rng.fill(&mut sender);
                rng.fill(&mut receiver);
                TransactionData::new(
                    sender,
                    receiver,
                    value,
                    nonce:0
                    payload: vec![0xFF; 1024],
                    signature: [0xFF;65],
                )
            };
            let msg = NetworkMessage::Transaction(tx);
            // let self_addr:SocketAddr = "127.0.0.1:0".parse().unwrap();
            let manager = self.clone();
            tokio::spawn(async move{
                manager.broadcast(msg).await;
            });
        }
    }

}
    */