use std::collections::{HashMap, HashSet};

use rocksdb::{DB, IteratorMode, Options, WriteBatch};

use crate::block::types::{Account, Address, BlockData, GlobalBalance, Hash, TokenTicker};

const PREFIX_BLOCK: u8 = b'b';
const PREFIX_INDEX: u8 = b'i';
const PREFIX_ACCOUNT: u8 = b'a';
const PREFIX_TOKEN: u8 = b't';
const PREFIX_GLOBAL_STATE: u8 = b'g';
const PREFIX_STAKER: u8 = b's';
const KEY_LAST_BLOCK: &[u8] = b"last_block";


pub struct Storage{
    db: DB,
}

impl Storage{
    pub fn new(path: &str) -> Self{
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).expect("DB open failed");
        Self {db}
    }

    fn blk_key(hash: &Hash) -> Vec<u8> {
        let mut key = vec![PREFIX_BLOCK];
        key.extend_from_slice(hash);
        key
    }
    fn idx_key(height: u64) -> Vec<u8> {
        let mut key = vec![PREFIX_INDEX];
        key.extend_from_slice(&height.to_be_bytes());
        key
    }
    fn acc_key(addr: &Address) -> Vec<u8> {
        let mut key = vec![PREFIX_ACCOUNT];
        key.extend_from_slice(addr);
        key
    }
    fn staker_key(addr: &Address) -> Vec<u8>{
        let mut key = vec![PREFIX_STAKER];
        key.extend_from_slice(addr);
        key
    }
    fn token_key(id:u32) -> Vec<u8> {
        let mut key = vec![PREFIX_TOKEN];
        key.extend_from_slice(&id.to_be_bytes());
        key
    }

    pub fn put_global_state(&self, state: &GlobalBalance){
        let bytes = postcard::to_allocvec(state).expect("GlobalState serialize failed");
        self.db.put(vec![PREFIX_GLOBAL_STATE],bytes).expect("DB write failed");
    }
    pub fn get_global_state(&self) -> Option<GlobalBalance>{
        let data = self.db.get(vec![PREFIX_GLOBAL_STATE]).ok().flatten()?;
        postcard::from_bytes(&data).ok()
    }

    pub fn put_staker(&self, addr: &Address, amount: u64){
        let key = Self::staker_key(addr);
        self.db.put(key, amount.to_be_bytes()).expect("Staker Save Failed");
    }
    pub fn get_all_stakers(&self) -> (Vec<(Address, u64)>, u64){
        let mut stakers = Vec::new();
        let mut total_stake = 0u64;
        let mode = IteratorMode::From(&[PREFIX_STAKER], rocksdb::Direction::Forward);
        let iter = self.db.iterator(mode);
        for item in iter{
            if let Ok((key, value)) = item{
                if key.is_empty() || key[0] != PREFIX_STAKER {break;}
                let mut addr = [0u8;20];
                addr.copy_from_slice(&key[1..21]);
                let mut amount_bytes = [0u8;8];
                amount_bytes.copy_from_slice(&value);
                let amount = u64::from_be_bytes(amount_bytes);
                stakers.push((addr,amount));
                total_stake += amount;
            }
        }
        (stakers, total_stake)
    }

    pub fn get_account(&self, address: &Address, cur_height: u64) -> Account{
        let key = Self::acc_key(address);
        self.db.get(key).ok().flatten()
            .and_then(|bytes| postcard::from_bytes(&bytes).ok())
            .unwrap_or(Account { balance: HashMap::new() , nonce: 0u64, last_seen_block: cur_height })
    }
    pub fn get_block(&self, hash: &Hash) -> Option<BlockData>{
        let key = Self::blk_key(hash);
        let data = self.db.get(key).ok().flatten()?;
        postcard::from_bytes(&data).ok()
    }
    pub fn get_block_by_height(&self, height: u64) -> Option<BlockData>{
        let hash = self.get_hash_by_height(height)?;
        self.get_block(&hash)
    }
    pub fn get_hash_by_height(&self, height:u64) -> Option<Hash>{
        let key = Self::idx_key(height);
        let bytes = self.db.get(key).ok().flatten()?;
        let mut hash = [0u8;32];
        hash.copy_from_slice(&bytes);
        Some(hash)
    }
    pub fn get_latest_block(&self) -> Option<BlockData>{
        let last_hash_bytes = self.db.get(KEY_LAST_BLOCK).ok().flatten()?;
        let mut last_hash = [0u8;32];
        last_hash.copy_from_slice(&last_hash_bytes);
        self.get_block(&last_hash)
    }

    pub fn commit_block(&self, block: &BlockData, state_update: &HashMap<Address, Account>, updated_tokens: &HashSet<TokenTicker>, global_state: &GlobalBalance){
        let mut batch = WriteBatch::default();
        let height = block.header.height;

        let blk_key = Self::blk_key(&block.hash);
        let blk_bytes = postcard::to_allocvec(block).expect("Block Serialize Failed");
        batch.put(blk_key, blk_bytes);
        
        //height - block hash 
        let idx_key = Self::idx_key(height);
        batch.put(idx_key, block.hash);
 
        //KEY_LAST_BLOCK은 최신의 블록 해시 하나의 값만 가짐
        batch.put(KEY_LAST_BLOCK, block.hash);

        for (addr, acc) in state_update{
            let acc_key = Self::acc_key(addr);
            let acc_bytes = postcard::to_allocvec(acc).expect("Account Serialize Failed");
            batch.put(acc_key, acc_bytes);

            let s_key = Self::staker_key(addr);
            if let Some(&gov_amount) = global_state.gov_shares.get(addr){
                if gov_amount > 0{
                    batch.put(s_key, gov_amount.to_be_bytes());
                } else{
                    batch.delete(s_key);
                }
            }
        }
        for ticker in updated_tokens{
            if let Some(_info) = global_state.token_metadata.get(ticker){
                let mut t_key = vec![PREFIX_TOKEN];
                t_key.extend_from_slice(ticker.as_bytes());
                let t_bytes = postcard::to_allocvec(_info).expect("Token Serialize Failed");
                batch.put(t_key, t_bytes);
            }
        }

        if height == 0 || height % 10 == 0 {
            let gs_bytes = postcard::to_allocvec(global_state).expect("Globalstate Serialize Failed");
            let mut history_key = vec![PREFIX_GLOBAL_STATE];
            history_key.extend_from_slice(&height.to_be_bytes());
            batch.put(history_key, gs_bytes);
            batch.put(b"latest_snapshot_height", height.to_be_bytes());
        }
        self.db.write(batch).expect("Block Commit Failed");
    }

    pub fn is_empty(&self) -> bool{
        self.db.get(KEY_LAST_BLOCK).ok().flatten().is_none()
    }

}