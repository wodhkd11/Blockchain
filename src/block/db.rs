use std::{collections::{HashMap, HashSet}, sync::Arc};

use eth_trie::DB as TrieDB;

use rocksdb::{DB, IteratorMode, Options, WriteBatch};

use crate::block::types::{Account, Address, Balance, BlockData, GlobalBalance, Hash, TokenTicker, TransactionForDB};

const PREFIX_BLOCK: u8 = b'b';
const PREFIX_INDEX: u8 = b'i';
const PREFIX_ACCOUNT: u8 = b'a';
const PREFIX_TOKEN: u8 = b't';
const PREFIX_POINTER: u8 = b'p';
const PREFIX_GLOBAL_STATE: u8 = b'g';
const PREFIX_STAKER: u8 = b's';
const KEY_LAST_BLOCK: &[u8] = b"last_block";


pub struct Storage{
    pub db: Arc<DB>,
}

#[derive(Debug)]
pub struct TrieError(rocksdb::Error);
impl std::fmt::Display for TrieError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
        write!(f, "Trie DB Error: {}", self.0)
    }
}
impl std::error::Error for TrieError{}

#[derive(Clone)]
pub struct TrieDb{pub inner: Arc<DB>,}

impl TrieDB for TrieDb{
    type Error = TrieError;
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error>{
        self.inner.get(key).map_err(TrieError)
    }
    fn insert(&self, key: &[u8], value: Vec<u8>) -> Result<(), Self::Error> {
        self.inner.put(key, value).map_err(TrieError)
    }
    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        self.inner.delete(key).map_err(TrieError)
    }
    fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}


impl Storage{
    pub fn new(path: &str) -> Self{
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = Arc::new(DB::open(&opts, path).expect("DB open failed"));
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

    //flat db 사용해서 mpt 이외에 최신 상태를 저장함.
    pub fn get_account_flat(&self, addr: &Address) -> Option<Account>{
        let mut key = vec![PREFIX_ACCOUNT];
        key.extend_from_slice(addr);
        self.db.get(key).ok().flatten().and_then(|bytes| postcard::from_bytes(&bytes).ok())
    }
    


    pub fn get_global_snapshot(&self) -> Option<GlobalBalance>{
        self.db.get(b"global_state_snapshot")
            .ok()
            .flatten()
            .and_then(|data| postcard::from_bytes(&data).ok())
    }

    pub fn get_block(&self, hash: &Hash) -> Option<BlockData>{
        let key = Self::blk_key(hash);
        self.db.get(key)
            .ok()
            .flatten()
            .and_then(|data| postcard::from_bytes(&data).ok())
    }

    pub fn get_latest_block(&self) -> Option<BlockData> {
        let last_hash_bytes = self.db.get(KEY_LAST_BLOCK).ok().flatten()?;
        let mut last_hash = [0u8;32];
        if last_hash_bytes.len() == 32{
            last_hash.copy_from_slice(&last_hash_bytes);
            self.get_block(&last_hash)
        } else {
            None
        }
    }

    pub fn get_hash_by_height(&self, height: u64) -> Option<Hash>{
        let key = Self::idx_key(height);
        let bytes = self.db.get(key).ok().flatten()?;
        let mut hash = [0u8;32];
        if bytes.len() == 32{
            hash.copy_from_slice(&bytes);
            Some(hash)
        } else { None }
    }


    pub fn put_staker(&self, addr: &Address, amount: u64) -> Result<(), rocksdb::Error>{
        let key = Self::staker_key(addr);
        self.db.put(key, amount.to_be_bytes())
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


    pub fn commit_block(&self, block: &BlockData, state_update: &HashMap<Address, Account>, updated_tokens: &HashSet<TokenTicker>, global_state: &GlobalBalance)
     -> Result<(), Box<dyn std::error::Error>>{
        let mut batch = WriteBatch::default();
        let height = block.header.height;
        for (idx, ctx) in block.body.iter().enumerate(){
            let receipt = TransactionForDB{
                hash: ctx.hash,
                block_height: height,
                block_hash: block.hash,
                index: idx as u32,
                status: 1,
            };
            let receipt_bytes = postcard::to_allocvec(&receipt)?;
            let mut key = vec![PREFIX_POINTER];
            key.extend_from_slice(&ctx.hash);
            batch.put(key,receipt_bytes);
        }

        let blk_bytes = postcard::to_allocvec(block)?;
        batch.put(Self::blk_key(&block.hash), blk_bytes);
        batch.put(Self::idx_key(height), block.hash);
        batch.put(KEY_LAST_BLOCK, block.hash);

        for ticker in updated_tokens{
            if let Some(info) = global_state.token_metadata.get(ticker) {
                let mut t_key = vec![PREFIX_TOKEN];
                t_key.extend_from_slice(ticker.as_bytes());
                batch.put(t_key, postcard::to_allocvec(info)?);
            }
        }

        for (addr, acc) in state_update{
            let mut key = vec![PREFIX_ACCOUNT];
            key.extend_from_slice(addr);
            let bytes = postcard::to_allocvec(acc)?;
            batch.put(key, bytes);
        }

        let gs_bytes = postcard::to_allocvec(global_state)?;
        batch.put(b"global_state_snapshot", &gs_bytes);

        if height % 10 == 0{
            let mut history_key = vec![PREFIX_GLOBAL_STATE];
            history_key.extend_from_slice(&height.to_be_bytes());
            batch.put(history_key, &gs_bytes);
            batch.put(b"latest_snapshot_height", height.to_be_bytes());
        }

        self.db.write(batch)?;
        Ok(())

    }

    pub fn is_empty(&self) -> bool{
        self.db.get(KEY_LAST_BLOCK).ok().flatten().is_none()
    }


    
}