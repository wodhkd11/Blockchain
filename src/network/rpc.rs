use axum::{Json, Router, extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{post, get}};
use reqwest::Method;
use serde::Deserialize;
use serde_json::json;
use serde_with::serde_as;
use sha3::{Digest, Keccak256};
use tower_http::cors::{Any, CorsLayer};
use std::{collections::HashMap, sync::Arc};
use crate::{block::{types::Hash, transaction::TransactionData}, network::{message::NetworkMessage, node::NodeManage}};
use hex;

#[derive(Deserialize, Debug)]
struct RpcRequest{
    method: String,
    params: Vec<serde_json::Value>,
    id: serde_json::Value,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TransactionRequest{
    pub sender: [u8; 20],
    pub receiver: [u8; 20],
    pub value: u64,
    pub nonce: u64,
    pub payload: Vec<u8>,
    #[serde_as(as = "[_; 65]")]
    pub signature: [u8; 65],
} // after get string, to_hex and do Transaction::New()

pub async fn start_rpc_server(manager: Arc<NodeManage>, rpc_port: u16){
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", post(handle_eth_request))
        .route("/transaction", post(handle_tx_submission))
        .route("/nonce/{address}", get(get_nonce_handler))
        .layer(cors)
        .with_state(manager);
    let addr = format!("0.0.0.0:{}", rpc_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("[RCP SERVER]: running at http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn handle_eth_request(
    State(manager): State<Arc<NodeManage>>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    println!("{:?}", req);
    let ret = match req.method.as_str() {
        // 1. 체인 ID 응답 (메타마스크 네트워크 연결용)
        "eth_chainId" => {
            let chain_id = { manager.state.read().await.chain_id };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", chain_id)
            })).into_response()
        },

        // 2. 네트워크 버전 응답
        "net_version" => {
            let chain_id = { manager.state.read().await.chain_id };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": chain_id.to_string()
            })).into_response()
        },

        // 3. 최신 블록 높이 응답 (이게 응답되어야 잔액 조회가 시작됨)
        "eth_blockNumber" => {
            let height = { manager.state.read().await.block_height };
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", height)
            })).into_response()
        },

        // 4. 기본 잔액(Native Token, 예: GOV) 응답
        "eth_getBalance" => {
            let addr_str = req.params.get(0).and_then(|v| v.as_str()).unwrap_or("");
            let address = hex_to_address(&addr_str.to_string());
            
            let state = manager.state.read().await;
            let balance = state.global_state.read().await.balances.get(&address)
                .and_then(|acc| acc.balance.get(&"GOV".to_string()))
                .cloned()
                .unwrap_or(0);

            // [핵심] 메타마스크 18자리 소수점 보정 (1 GOV -> 10^18 wei)
            let display_balance = balance as u128 * 10_u128.pow(18);
            let test_balance= 777_u128 * 10_u128.pow(18);

            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": format!("0x{:x}", test_balance)
            })).into_response()
        },

        // 5. 커스텀 토큰(예: KRW) 잔액 및 기타 호출 응답
        "eth_call" => {
            let Some(params) = req.params.get(0) else {
                return Json(json!({"jsonrpc": "2.0", "id": req.id, "error": "no params"})).into_response();
            };
            let to_address = params.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let data = params.get("data").and_then(|v| v.as_str()).unwrap_or("");

            // balanceOf(address) 요청 파싱 (시그니처: 0x70a08231)
            if data.starts_with("0x70a08231") {
                let user_addr_str = &data[34..];
                let user_addr = hex_to_address(&user_addr_str.to_string());
                
                let state = manager.state.read().await;
                let token_symbol = match to_address.to_lowercase().as_str() {
                    "0x0000000000000000000000000000000000000001" => "KRW",
                    _ => "GOV",
                };

                let balance = state.global_state.read().await.balances.get(&user_addr)
                    .and_then(|acc| acc.balance.get(&token_symbol.to_string()))
                    .cloned()
                    .unwrap_or(0);

                // [핵심] 토큰도 18자리 보정 + 32바이트 패딩 응답
                let display_balance = balance as u128 * 10_u128.pow(18);
                return Json(json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "result": format!("0x{:0>64x}", display_balance)
                })).into_response();
            }

            // 그 외 알 수 없는 eth_call에 대한 기본 응답 (에러 방지용)
            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": "0x0000000000000000000000000000000000000000000000000000000000000000"
            })).into_response()
        },
        "eth_getBlockByNumber" => {
            // params: [block_height_hex, full_tx_obj_bool]
            let height_hex = req.params.get(0).and_then(|v| v.as_str()).unwrap_or("0x0");
            let height = u64::from_str_radix(height_hex.trim_start_matches("0x"), 16).unwrap_or(0);

            let state = manager.state.read().await;
            // 실제 저장된 블록이 있으면 가져오고, 없으면 마지막 블록이나 제네시스를 임시로 반환
            let block = &state.last_block; 

            Json(json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "number": format!("0x{:x}", height),
                    "hash": format!("0x{}", hex::encode(block.hash)),
                    "parentHash": format!("0x{}", hex::encode(block.header.prev_block_hash)),
                    "timestamp": "0x65ada600", // 임시 타임스탬프
                    "transactions": [], // 일단 빈 배열로 응답해도 무방
                    "gasLimit": "0xffffff",
                    "gasUsed": "0x0"
                }
            })).into_response()
        },
        _ => {
            Json(json!({
                "jsonrpc": "2.0", 
                "id": req.id, 
                "error": {"code": -32601, "message": "Method not found"}
            })).into_response()
        }
    };
    println!("{:?}",ret);
    ret
}

/**
 * This function gets transaction and broadcast.
 * This function returns transaction's hash value.
 */
async fn handle_tx_submission(
    State(manager): State<Arc<NodeManage>>,
    Json(payload): Json<TransactionRequest>,
) -> Result<Json<Hash>, StatusCode>{
    let tx = payload.to_core_data().ok_or(StatusCode::UNAUTHORIZED)?;
    let tx_id = tx.calculate_hash();
    
    let mut hasher = Keccak256::new();
    hasher.update(&payload.signature);
    let sig_hash: [u8; 32] = hasher.finalize().into();

    {
        let mut node_state = manager.state.write().await;
        let storage = &node_state.storage;
        
        let is_nonce_valid = {
            let global_state= node_state.global_state.read().await;
            global_state.check_nocne(&tx.sender, tx.nonce, node_state.block_height + 1, storage)
        };
        if !is_nonce_valid{ return Err(StatusCode::BAD_REQUEST); }
        if node_state.mempool.contains_key(&tx_id){ return Err(StatusCode::CONFLICT); }
        node_state.mempool.insert(tx_id, tx.clone());
    }
    let manager_clone = manager.clone();
    let msg = NetworkMessage::NewTransaction(tx.clone());
    tokio::spawn(async move{manager_clone.broadcast(msg).await;});
    Ok(Json(sig_hash))
}

async fn get_nonce_handler(
    State(manager): State<Arc<NodeManage>>,
    Path(address): Path<String>,
) -> impl IntoResponse{
    let address = hex_to_address(&address);
    let manager_clone = manager.state.read().await;
    let storage = &manager_clone.storage;
    let nonce = manager_clone.global_state.write().await.get_nonce(&address, manager_clone.block_height + 1, storage);
    Json(nonce)
}

fn hex_to_address(hex_str: &String) -> [u8;20]{
    let clean_hex = hex_str.trim_start_matches("0x");
    let decoded = hex::decode(clean_hex).expect("INVALID HEX");
    let mut address = [0u8;20];
    address.copy_from_slice(&decoded[..20]);
    address
}

impl TransactionRequest{
    pub fn to_core_data(&self) -> Option<TransactionData>{
        if !self.verify_signature(){
            println!("[RPC]: INVALID SIGNATURE");
            return None;
        }

        //is signed?
        Some(TransactionData::new(
            self.sender,
            self.receiver,
            self.value,
            self.payload.clone(),
            self.nonce,
            self.signature,
        ))
    }
    fn verify_signature(&self) -> bool{
        let mut v = Vec::new();
        v.extend_from_slice(&self.sender);
        v.extend_from_slice(&self.receiver);
        v.extend_from_slice(&self.value.to_be_bytes());
        v.extend_from_slice(&self.nonce.to_be_bytes());
        v.extend_from_slice(&self.payload);

        crate::crypto::signature::verify(self.sender, &self.signature, &v)
    }
}






//async fn handle_eth_request(
    //State(manager): State<Arc<NodeManage>>,
    //Json(req): Json<RpcRequest>,
//) -> impl IntoResponse{
    //println!("{:?}", req);
    //match req.method.as_str(){
        //"eth_chainId" => {
            //let chain_id = {manager.state.read().await.chain_id};
            //Json(json!({
                //"jsonrpc": "2.0",
                //"id": req.id,
                //"result": format!("0x{:x}", chain_id)
            //}))
        //},
        //"eth_blockNumber" => {
            //let height = { manager.state.read().await.block_height };
            //Json(json!({
                //"jsonrpc": "2.0",
                //"id": req.id,
                //"result": format!("0x{:x}", height) // 반드시 16진수 format
            //}))
        //},
        //"not_version" => {
            //let chain_id = {manager.state.read().await.chain_id};
            //Json(json!({
                //"jsonrpc": "2.0",
                //"id": req.id,
                //"result": chain_id.to_string()
            //}))
        //},
        //"eth_getBalance" => {
            //let addr_str = req.params.get(0).and_then(|v| v.as_str()).unwrap_or("");
            //let address = hex_to_address(&addr_str.to_string());
            //let state = manager.state.read().await;
            //let balance = state.global_state.balances.get(&address)
                //.and_then(|acc| acc.balance.get(&"GOV".to_string()))
                //.cloned()
                //.unwrap_or(0);
            //Json(json!({
                //"jsonrpc": "2.0",
                //"id": req.id,
                //"result": format!("0x{:x}", balance)
            //}))
        //},
        //"eth_call" => {
            //let Some(params) = req.params.get(0) else {
                //return Json(json!({"jsonrpc": "2.0", "id": req.id, "error": "no params"})).into_response();
            //};

            //// 2. to_address 추출 에러 수정 (and_then은 Option을 반환해야 함)
            //let to_address = params.get("to").and_then(|v| v.as_str()).unwrap_or("");
            //let data = params.get("data").and_then(|v| v.as_str()).unwrap_or("");

            //// 3. balanceOf(address) 요청 처리 (0x70a08231)
            //if data.starts_with("0x70a08231") || data.starts_with("70a08231") {
                //// data가 0x로 시작할 경우와 아닐 경우를 대비해 오프셋 조정
                //let offset = if data.starts_with("0x") { 34 } else { 32 };
        
                //// 주소 파싱 시 범위 초과 방지
                //if data.len() >= offset + 40 {
                    //let user_addr_str = &data[offset..offset + 40];
                    //let user_addr = hex_to_address(&user_addr_str.to_string());
            
                    //let state = manager.state.read().await;
            
                    //// 4. 토큰 주소 매핑 (소문자로 변환하여 비교하는 것이 안전)
                    //let token_symbol = match to_address.to_lowercase().as_str() {
                        //"0x0000000000000000000000000000000000000001" => "KRW",
                        //_ => "GOV",
                    //};

                    //let balance = state.global_state.balances.get(&user_addr)
                        //.and_then(|acc| acc.balance.get(&token_symbol.to_string()))
                        //.cloned()
                        //.unwrap_or(0);

                    //return Json(json!({
                        //"jsonrpc": "2.0",
                        //"id": req.id,
                        //"result": format!("0x{:0>64x}", balance)
                    //})).into_response();
                //}
            //}
    
            //// 기본 응답 (메타마스크 에러 방지용)
            //Json(json!({
                //"jsonrpc": "2.0", 
                //"id": req.id, 
                //"result": "0x0000000000000000000000000000000000000000000000000000000000000000"
            //}))
        //},
        //_ => Json(json!({"jsonrpc": "2.0", "id": req.id, "error": {"code": 032601, "message": "Method not found"}})),
    //}.into_response()
//}
