
use std::collections::HashMap;

use crate::block::db::Storage;
use crate::block::types::{Account, Address, Balance, GlobalBalance, TokenTicker};
use crate::rule::config::{self, NetworkConfig};


/// 여기서 상태 전이 함수를 정의해야함.
impl GlobalBalance{

    pub fn new() -> Self{
        let gov_shares = HashMap::new();
        let balances = HashMap::new();
        Self{
            balances,
            gov_shares,
            gas_pool: Balance::zero(),
            token_metadata: HashMap::new(),
            config: crate::rule::config::NetworkConfig::new(18),
        }
    }

    pub fn remove_from_memory(&mut self, cur_height: u64, retation: u64){
        let before_count = self.balances.len();
        self.balances.retain(|_, acc|{
            (cur_height.saturating_sub(acc.last_seen_block)) < retation
        });
        let after_count = self.balances.len();
        if before_count != after_count{
            println!("[GLOBAL STATE]: REMOVED {} account from RAM", before_count - after_count);
        }
    }

    //methods
    pub fn get_account_mut(&mut self, address: &Address, cur_height: u64, db:&Storage) -> &mut Account{
        let account = self.balances.entry(*address).or_insert_with(||{
            db.get_account(address, cur_height)
        });
        account.last_seen_block = cur_height;
        account
    }
    pub fn get_account_read(&self, address: &Address, cur_height: u64, db: &Storage) -> Account{
        if let Some(acc) = self.balances.get(address){ return acc.clone(); }
        db.get_account(address, cur_height)
    }

    pub fn get_token_balance(&mut self, address: &Address, token: &TokenTicker, cur_height: u64, db: &Storage) -> Balance{
        let account = self.get_account_mut(address, cur_height, db);
        *account.balance.get(token).unwrap_or(&Balance::zero())
    }

    pub fn get_nonce(&mut self, addr: &Address, cur_height: u64, db: &Storage) -> u64{
        let account = self.get_account_mut(addr, cur_height, db);
        account.nonce
    }

    pub fn check_nocne(&self, addr: &Address, tx_nonce: u64, cur_height: u64, db: &Storage) -> bool{
        let account = self.get_account_read(addr,  cur_height, db);
        account.nonce == tx_nonce
    }

    // pub fn inc_nonce(&mut self, addr: &Address, cur_height: u64, db: &Storage) {
        // let account = self.get_account_mut(addr, cur_height, db);
        // account.nonce += 1;
    // }

    // pub fn add_balance(&mut self, addr: &Address, token: &TokenTicker, amount: u64, cur_height: u64, db: &Storage) {
        // let account = self.get_account_mut(addr, cur_height, db);
        // let balance = account.balance.entry(token.clone()).or_insert(0);
        // *balance = balance.saturating_add(amount);
    // }
    // pub fn sub_balance(&mut self, addr: &Address, token: &TokenTicker, amount: u64, cur_height: u64, db: &Storage) -> Result<(), String>{
        // let account = self.get_account_mut(addr, cur_height, db);
        // let balance = account.balance.entry(token.clone()).or_insert(0);
        // if *balance < amount {return Err("INSUFFICIENT BALANCE".into());}
        // *balance = balance.saturating_sub(amount);
        // Ok(())
    // }
    
    // pub fn pay_gas(&mut self, addr: &Address, fee: u64, cur_height: u64, db: &Storage) -> Result<(), String>{
        // let ticker = self.config.gas_token.clone();
        // let account = self.get_account_mut(addr, cur_height, db);
        // let balance = account.balance.entry(ticker).or_insert(0);

        // if *balance < fee {return Err("INSUFFICIENT GAS FEE".into());}
        // *balance = balance.saturating_sub(fee);
        // self.gas_pool = self.gas_pool.saturating_add(fee);
        // Ok(())
    // }

    pub fn distribute_gas(&mut self, cur_height: u64, db: &Storage){
        let gas_tkn = self.config.gas_token.clone();
        if self.gas_pool == Balance::zero(){return;}
        let total_gas = self.gas_pool;
        self.gas_pool = Balance::zero();
        let shares: Vec<(Address, Balance)> = self.gov_shares
            .iter()
            .map(|(addr, share)| (*addr, *share))
            .collect();
        let total_shares: Balance = self.gov_shares
            .iter()
            .map(|(_, share)| *share)
            .fold(Balance::zero(), |acc, x| acc + x);
        println!("[TOTAL GAS]: {total_gas}KRW");
        for (addr, share) in shares{
            let reward = if total_shares == Balance::zero() {
                Balance::zero()
            } else {
                (total_gas * share) / total_shares
            };
            if reward > Balance::zero(){
                self.get_account_mut(&addr, cur_height, db).add_balance(&gas_tkn, reward);
            }
        }
    }
}
