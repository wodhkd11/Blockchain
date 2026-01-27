use std::{collections::{HashMap, HashSet}, sync::{Arc, LazyLock}};

use primitive_types::U256;
use tokio::sync::RwLock;

use crate::{block::{db::Storage, genesis::*, types::{Balance, BlockData, GlobalBalance, TokenInfo}}, network::node::*, state::statemanager::StateManager};

pub static DECIMALS_POW: LazyLock<U256> = LazyLock::new(|| {
    U256::from(10).pow(U256::from(18))
});

pub static TOTAL_SUPPLY: LazyLock<U256> = LazyLock::new(|| {
    U256::from(100).checked_mul(*DECIMALS_POW).unwrap()
});

impl NodeManage{
    pub fn new(port:u16, addr: &str, wallet: [u8;20], path: &str, is_genesis: bool) -> Self{

        let node_addr = addr.parse().expect("INVALID ADDR");
        let storage = Arc::new(Storage::new(path));
        let last_block = if storage.is_empty(){
            if is_genesis {
                println!("I am genesis NODE");
                BlockData::create_genesis_block(wallet)
            }else{
                println!("[NODE]: Load Genesis setting");
                GENESIS_BLOCK.clone()
            }
        } else{ storage.get_latest_block().unwrap() };

        let state_root = last_block.header.state_root;
        let state_manager = Arc::new(RwLock::new(StateManager::new(storage.clone(), state_root).expect("MPT ERROR")));

        let mut global_state = GlobalBalance::new();

        let owner = hex::decode("0fa41b6927a59eccb1f253a62e0164b5ce96f7c5")
            .expect("");
        let mut owner_addr = [0u8;20];
        owner_addr.copy_from_slice(&owner);
            
        global_state.token_metadata.insert("KRW".to_string(), TokenInfo {
            name: "Korean Won".to_string(),
            symbol: "KRW".to_string(),
            decimals: 18,
            total_supply: *TOTAL_SUPPLY, // 소수점 포함 계산
            admin: owner_addr,
        });

        // 2. GOV 토큰 메타데이터 등록
        global_state.token_metadata.insert("GOV".to_string(), TokenInfo {
            name: "Governance Token".to_string(),
            symbol: "GOV".to_string(),
            decimals: 18,
            total_supply: *TOTAL_SUPPLY,
            admin: owner_addr,
        });
        global_state.gov_shares.insert(owner_addr, *TOTAL_SUPPLY);
        let admin = global_state.get_account_safe(&owner_addr, 0, &storage);
        admin.add_balance(&"GOV".to_string(), *TOTAL_SUPPLY);
// 1. 100000을 U256으로 먼저 변환
// 2. 그 다음 U256끼리 곱하기 (checked_mul 사용)
        let initial_krw = U256::from(100_000)
            .checked_mul(*DECIMALS_POW)
            .expect("KRW supply overflow");

        admin.add_balance(&"KRW".to_string(), initial_krw);        


        let block_height = last_block.header.height;
        let block_hash = last_block.hash;

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
                state_manager,
                last_block: genesis.clone(),
                block_height,
            })),
         }
    }

}