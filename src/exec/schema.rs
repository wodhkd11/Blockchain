use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use crate::{block::types::{Address, Balance, TokenTicker}, rule::config::NetworkConfig};


//Payload Structure
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct RawPayload{
    pub opcode: u8,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fee: Option<Balance>,
    pub data: serde_json::Value,
}


//0x00: Token Register
#[derive(Deserialize, Serialize)]
pub struct RegisterTokenParams{
    pub name: String,
    pub symbol: TokenTicker,
    pub admin: Address,
    pub initial_supply: Balance, //
    pub decimals: u8,
}

//0x02: Transfer
#[derive(Deserialize, Serialize)]
pub struct TransferParams{ //전이할 데이터: 토큰과 값, from과 to는 transaction에 존재
    pub ticker: TokenTicker,
}

//0x01: Minting
#[derive(Deserialize, Serialize)]
pub struct MintParams{
    pub ticker: TokenTicker,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BurnParams{
    pub ticker: TokenTicker,
}

//0xff: Admin only
#[derive(Deserialize, Serialize)]
pub struct ChangeConfig{
    pub min_gas_price: Option<Balance>,
    pub gas_token: Option<String>,
    pub governance_threshold: Option<Balance>,
}

//pub fn MintParams{}
//pub fn BurnParams{}
