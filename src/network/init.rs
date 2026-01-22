use std::{collections::{HashMap, HashSet}, sync::Arc};

use tokio::sync::RwLock;

use crate::{block::{db::Storage, genesis::*, types::{BlockData, GlobalBalance, TokenInfo}}, network::node::*};



impl NodeManage{
    pub fn new(port:u16, addr: &str, wallet: [u8;20], path: &str, is_genesis: bool) -> Self{

        let node_addr = addr.parse().expect("INVALID ADDR");
        let storage = Arc::new(Storage::new(path));
        let last_block = if storage.is_empty(){
            if is_genesis {
                println!("I am genesis NODE");
                let g = BlockData::create_genesis_block(wallet);
                g
            }else{
                println!("[NODE]: Load Genesis setting");
                GENESIS_BLOCK.clone()
            }
        } else{ storage.get_latest_block().unwrap() };
        let block_height = last_block.header.height;
        let mut global_state = GlobalBalance::new();
        let owner = hex::decode("0fa41b6927a59eccb1f253a62e0164b5ce96f7c5")
            .expect("");
        let mut owner_addr = [0u8;20];
        owner_addr.copy_from_slice(&owner);
            
        global_state.token_metadata.insert("KRW".to_string(), TokenInfo {
            name: "Korean Won".to_string(),
            symbol: "KRW".to_string(),
            decimals: 1,
            total_supply: TOTAL_SUPPLY, // 소수점 포함 계산
            admin: owner_addr,
        });

        // 2. GOV 토큰 메타데이터 등록
        global_state.token_metadata.insert("GOV".to_string(), TokenInfo {
            name: "Governance Token".to_string(),
            symbol: "GOV".to_string(),
            decimals: 1,
            total_supply: TOTAL_SUPPLY,
            admin: owner_addr,
        });

        global_state.add_balance(&owner_addr, &"GOV".to_string(), TOTAL_SUPPLY, 0, &storage);
        global_state.add_balance(&owner_addr, &"KRW".to_string(), 100000*DECIMALS, 0, &storage);
        
        let genesis = &*GENESIS_BLOCK;

        println!("{:?}",genesis.hash) ;              
        Self { 
            state: Arc::new(RwLock::new(Node{
                port,
                addr: node_addr,
                wallet: wallet,
                chain_id: 6699,
                peers: HashMap::new(),
                unconnected_addrs: HashSet::new(),
                max_peers: 100, // Default: 10, need to change
                recent_seen_message: HashMap::new(),
                mempool: HashMap::new(),
                global_state: Arc::new(global_state.into()),
                storage,
                last_block: genesis.clone(),
                block_height: 0,
            })),
         }
    }

}