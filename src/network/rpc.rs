use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use std::sync::Arc;
use crate::{block::model_struct::{Hash, TransactionData}, network::{message::NetworkMessage, node::NodeManage}};

#[derive(serde::Deserialize)]
pub struct TransactionRequest{
    pub sender: [u8; 20],
    pub receiver: [u8; 20],
    pub nonce: u64,
    pub data: String,
} // after get string, to_hex and do Transaction::New()

pub async fn start_rpc_server(manager: Arc<NodeManage>, rpc_port: u16){
    let app = Router::new()
        .route("/transaction", post(handle_tx_submission))
        .with_state(manager);
    let addr = format!("0.0.0.0:{}", rpc_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("[RCP SERVER]: running at http://{addr}");
    axum::serve(listener, app).await.unwrap();
}


/**
 * This function gets transaction and broadcast.
 * This function returns transaction's hash value.
 */
async fn handle_tx_submission(
    State(manager): State<Arc<NodeManage>>,
    Json(payload): Json<TransactionRequest>,
) -> Result<Json<Hash>, StatusCode>{
    let hex_data = payload.data.trim_start_matches("0x");
    let raw_data = hex::decode(hex_data).map_err(|e|{
        println!("[RPC]: Hex decoding failed{e}");
        StatusCode::BAD_REQUEST
    })?;
    let tx = TransactionData::new(
        payload.sender,
        payload.receiver,
        raw_data,
        payload.nonce,
    );
    let tx_hash = tx.hash;
    {
        let mut state = manager.state.write().await;
        state.mempool.insert(tx.hash, tx.clone());
        println!("[NEW TRANSACTION] New transaction got: {:?}",tx_hash);
    }
    let msg = NetworkMessage::Transaction(tx);
    let manager_clone = manager.clone();
    tokio::spawn(async move{manager_clone.broadcast(msg).await;});

    Ok(Json(tx_hash))
}