use std::{collections::{HashMap, HashSet}, sync::Arc};
use crate::{block::{transaction::ConfirmedTransaction, types::{Account, Address, BlockData}}, exec, network::{message::NetworkMessage, node::{self, Node, NodeManage}}};
use tokio::{sync::RwLock, time::{Duration, sleep}};

impl NodeManage{
    pub async fn start_miner(self: Arc<Self>){

        // 다른 노드들에 먼저 접속해서 최근 20개 블록 데이터? 를 받아와야함. 해시는 20개의 블록으로 하기
        println!("[MINER]BOOTING MINER");
        loop{
            sleep(Duration::from_secs(4)).await;
            let mut node_lock = self.state.write().await;
            let mut global_state = node_lock.global_state.write().await.clone();
            let mut state_update: HashMap<Address, Account> = HashMap::new();
            let mut token_updates = HashSet::new();


            let my_address = node_lock.wallet;
            let my_gov = global_state.gov_shares.get(&my_address).cloned().unwrap_or(0);
            let port = node_lock.port;
            if port != 9000{
                println!("PERMISSION DENIED ");
                continue;
            }
            //트랜잭션 없으면 쉬는 로직도 필요할듯?
            //if my_gov == 0{
                //println!("PERMISSION DENIED");
                //drop(node_lock);
                //continue;
            //}// 이외에도 POS 등 로직 넣어줘야하는곳. 현재 내가 마이닝노드인지 확인 알고리즘 필요함.
        
            let mut valid_transactions = Vec::new();
            let keys: Vec<_>  = node_lock.mempool.keys().take(100).cloned().collect();
            let storage = node_lock.storage.clone();
            let next_height = node_lock.block_height + 1;

            for key in keys{
                let tx = if let Some(tx) = node_lock.mempool.get(&key){
                    tx.clone()
                }else {continue;};
                
                if !tx.verify(){
                    node_lock.mempool.remove(&key);
                    continue;
                }

                match exec::apply_transaction(&mut global_state, &tx, next_height, &storage){
                    Ok(diff) => {
                        node_lock.mempool.remove(&key);
                        valid_transactions.push(ConfirmedTransaction::from(&tx));
                        for (addr, acc) in diff.accounts{
                            state_update.insert(addr, acc);
                        }
                        if let Some(ticker) = diff.token_changed{
                            token_updates.insert(ticker);
                        }
                    }
                    Err(e) => {
                        println!("[MINER]: Transaction Exec failed {e}");
                        node_lock.mempool.remove(&key);
                    }
                }
            }
            // if valid_transactions.is_empty() {
                // println!("[MINER]: No transaction, jump this block.");
                // continue;
            // }
            println!("[MINER]: New block generating: {} Transactions", valid_transactions.len());
            global_state.distribute_gas(next_height, &storage);
        
            let new_block = BlockData::new(
                &node_lock.last_block,
                valid_transactions,
                my_address
            );

            global_state.remove_from_memory(next_height, 20); // 이거 블록 20개가 아니라 cnofig에서 가져오는거도 해봐야됨.
            node_lock.storage.commit_block(&new_block, &state_update, &token_updates, &global_state);
            
            *node_lock.global_state.write().await = global_state;
            let block_hash = new_block.hash;
            node_lock.block_height = next_height;
            node_lock.last_block = new_block.clone();

            //global_state.balances.clear();
            println!("\n====================");
            println!("[MINER]: New block generated");
            println!("Height: {}",next_height);
            println!("Hash: {:?}",hex::encode(block_hash));
            println!("\n====================\n");
            let msg = NetworkMessage::NewBlock(new_block);
            drop(node_lock);
            self.broadcast(msg).await;

        }
    }


}