//use std::{sync::Arc, time::Duration};

//use sha3::{Digest, Keccak256};
//use tokio::time::interval;

//use crate::{block::model_struct::{BlockData, BlockHeader}, network::node::NodeManage};




//pub async fn run_block_tester(manager: Arc<NodeManage>){
    //let mut tick = interval(Duration::from_secs(10));
    //loop{
        //tick.tick().await;


        //let node = manager.state.read().await;
        //let my_address = [0xFF; 20];

        //let current_slot = std::


        //let mut node = manager.state.write().await;
        //if node.mempool.is_empty(){
            //println!("[PRODUCER]: Mempool is empty");
        //}

        //let transactions: Vec<_> = node.mempool.values().cloned().collect();
        //node.mempool.clear();

        //let cur_height = 1 as u64;
        //let header = BlockHeader{
            //height: cur_height,
            //prev_block_hash: [0u8;32],
            //merkle_root:[0u8;32],
            //timestamp: std::time::SystemTime::now()
                //.duration_since(std::time::UNIX_EPOCH)
                //.unwrap()
                //.as_secs(),
            //valdiator: [0x11; 20],
        //};
        //let block_hash = {
            //let mut hasher = Sha256::new();
            //hasher.update(header.height.to_be_bytes());
            //let res = hasher.finalize();
            //let mut h = [0u8;32];
            //h.copy_from_slice(&res);
            //h
        //};
        //let block = BlockData{
            //header,
            //body: transactions,
            //hash: block_hash,
            //signature: vec![],
        //};
        //let storage = node.storage.clone();
        //node.global_state.commit_to_db(&storage, &block);
        //println!("[NEW BLOCK]: {:?}", block)
    //}
//}