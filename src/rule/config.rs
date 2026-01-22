use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkConfig{
    pub min_gas_price: u64,
    pub gas_token: String,
    pub governance_threshold: u64,
    pub gov_token: String,
    pub last_updated_height: u64,
}

impl Default for NetworkConfig{
    fn default() -> Self {
        Self{
            min_gas_price: 0,
            gas_token: "KRW".into(),
            governance_threshold: 1,
            gov_token: "GOV".into(),
            last_updated_height: 0,
            
        }
    }
}

