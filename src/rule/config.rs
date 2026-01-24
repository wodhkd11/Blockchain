use primitive_types::U256;
use serde::{Serialize, Deserialize};

use crate::{block::types::Balance, network::init::DECIMALS_POW};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkConfig{
    pub min_gas_price: Balance,
    pub gas_token: String,
    pub governance_threshold: Balance,
    pub gov_token: String,
    pub last_updated_height: u64,
}

impl NetworkConfig{
    pub fn new(decimals: u8) -> Self {
        let decimals_pow: U256 = U256::from(10).pow(U256::from(decimals));
        Self{
            min_gas_price: U256::zero(),
            gas_token: "KRW".into(),
            governance_threshold: decimals_pow,
            gov_token: "GOV".into(),
            last_updated_height: 0,
            
        }
    }
}

