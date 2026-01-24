use serde::{Deserialize, Serialize};

use crate::{block::types::{Address, Balance, TokenTicker}, rule::config::NetworkConfig};



#[derive(Deserialize, Serialize)]
pub struct RegisterTokenParams{
    pub name: String,
    pub symbol: TokenTicker,
    pub admin: Address,
    pub initial_supply: Balance, //
    pub decimals: u8,
}

#[derive(Deserialize, Serialize)]
pub struct TransferParams{ //전이할 데이터: 토큰과 값, from과 to는 transaction에 존재
    pub ticker: TokenTicker,
}

#[derive(Deserialize, Serialize)]
pub struct MintParams{
    pub ticker: TokenTicker,
}


#[derive(Deserialize, Serialize)]
pub struct ChangeConfig{
    pub min_gas_price: Option<Balance>,
    pub gas_token: Option<String>,
    pub governance_threshold: Option<Balance>,
}

//pub fn MintParams{}
//pub fn BurnParams{}
