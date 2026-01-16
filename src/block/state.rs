use std::collections::HashMap;
use serde::{Deserialize, Serialize};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBalance{
    pub balances: HashMap<[u8; 20], u64>,
    pub gov_shares: HashMap<[u8; 20], u64>,
    pub gas_pool: u64,
}

impl GlobalBalance{
    pub fn new() -> Self{
        let mut gov_shares = HashMap::new();
        gov_shares.insert([0x11; 20], 60);
        gov_shares.insert([0x22; 20], 40);
        Self{
            balances: HashMap::new(),
            gov_shares,
            gas_pool: 0,
        }
    }

    pub fn get_balance(&self, address: &[u8; 20]) -> u64{
        *self.balances.get(address).unwrap_or(&0)
    }
    pub(crate) fn set_balance(&mut self, address: [u8; 20], amount: u64){
        self.balances.insert(address, amount);
    }
    pub(crate) fn add_gas(&mut self, fee: u64){
        self.gas_pool = self.gas_pool.saturating_add(fee);
    }
    pub(crate) fn drain_gas_pool(&mut self) -> u64{
        let amount = self.gas_pool;
        self.gas_pool = 0;
        amount
    }
}