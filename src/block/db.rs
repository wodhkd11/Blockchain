use std::collections::HashMap;

use rocksdb::{DB, Options, WriteBatch};

use crate::block::{model_struct::{Address, BlockData, Hash}, state::Account};


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

    fn acc_key(address: &Address) -> Vec<u8>{
        let mut key = b"acc_".to_vec();
        key.extend_from_slice(address);
        key
    }
    fn blk_key(hash: &Hash) -> Vec<u8>{
        let mut key = b"blk_".to_vec();
        key.extend_from_slice(hash);
        key
    }
    fn idx_key(height: u64) -> String{
        format!("idx_{height}")
    }


    pub fn is_exist(&self, address: &Address) -> bool{
        self.db.key_may_exist(address) &&self.db.get(address).ok().flatten().is_some()
    }

    pub fn get_account(&self, address: &Address) -> Option<Account>{
        let key = Self::acc_key(address);
        match self.db.get(key).expect("DB read failed"){
            Some(bytes) => Some(postcard::from_bytes(&bytes).expect("Account deserialize failed")),
            None => None,
        }        
    }

    pub fn put_account(&self, address: &Address, account: &Account){
        let key = Self::acc_key(address);
        let bytes = postcard::to_allocvec(account).expect("Account serialzied failed");
        self.db.put(key, bytes).expect("DB write failed");
    }

    pub fn get_block(&self, hash: &Hash) -> Option<BlockData>{
        let key = Self::blk_key(hash);
        match self.db.get(key).expect("DB block read failed"){
            Some(bytes) => Some(postcard::from_bytes(&bytes).expect("Block deserialize failed")),
            None => None,
        }
    }

    pub fn get_hash_by_height(&self, height: u64) -> Option<Hash>{
        let key = Self::idx_key(height);
        match self.db.get(key.as_bytes()).expect("DB index read failed"){
            Some(bytes) => {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&bytes);
                Some(hash)
            }
            None => None,
        }
    }

    pub fn commit_block(&self, block: &BlockData, state_update: &HashMap<Address, Account>){
        let mut batch = WriteBatch::default();

        let blk_key = Self::blk_key(&block.hash);
        let blk_bytes = postcard::to_allocvec(block).expect("Block serialize failed");
        batch.put(blk_key, blk_bytes);

        let idx_key = Self::idx_key(block.header.height);
        batch.put(idx_key.as_bytes(), block.hash);

        for (addr, acc) in state_update{
            let acc_key = Self::acc_key(addr);
            let acc_bytes = postcard::to_allocvec(acc).expect("Acocunt serialize failed");
            batch.put(acc_key, acc_bytes);
        }
        
        self.db.write(batch).expect("Atomic block commit failed");
    }

}