use std::collections::HashMap;

use primitive_types::H256;
use rlp::{Decodable, Encodable};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::rule::config::NetworkConfig;


// Type aliases
pub type Address = [u8; 20];
pub type Hash = [u8; 32];
pub type Signature = [u8; 65];
pub type TokenTicker = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenInfo{
    pub name: String,
    pub symbol: TokenTicker,
    pub decimals: u8,
    pub total_supply: Balance,
    pub admin: Address,
}
// Block types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader{
    pub height: u64,
    pub prev_block_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u64,
    pub valdiator: Address,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockData{
    pub header: BlockHeader,
    pub body: Vec<crate::block::transaction::ConfirmedTransaction>,
    pub hash: Hash,
    #[serde(with = "BigArray")]
    pub signature: Signature,
}

// Account and Balance types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account{
    pub balance: HashMap<TokenTicker, Balance>, //Symbol, value
    pub nonce: u64,
    pub last_seen_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBalance{
    pub balances: HashMap<Address, Account>,
    pub gov_shares: HashMap<Address, Balance>,
    pub gas_pool: Balance,
    pub token_metadata: HashMap<TokenTicker, TokenInfo>,
    pub config: NetworkConfig,
}

pub struct StateDiff{
    pub accounts: HashMap<Address, Account>,
    pub token_changed: Option<TokenTicker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionForDB{
    pub hash: Hash,
    pub block_height: u64,
    pub block_hash: Hash,
    pub index: u32,
    pub status: u8,
}

pub type Balance = primitive_types::U256;
pub type Nonce = u64;

#[derive(Debug, Clone)]
pub struct PrimaryAsset{
    pub ticker: TokenTicker,
    pub amount: Balance,
}

pub struct AccountState{
    pub nonce: u64,
    pub primary_assets: Vec<PrimaryAsset>,
    pub asset_root: H256,
}

impl Encodable for PrimaryAsset{
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(2);
        s.append(&self.ticker);
        s.append(&self.amount);
    }
}
impl Decodable for PrimaryAsset{
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(Self{
            ticker: rlp.val_at(0)?,
            amount: rlp.val_at(1)?,
        })
    }
}

impl Encodable for AccountState{
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        s.begin_list(3);
        s.append(&self.nonce);
        s.append_list(&self.primary_assets);
        s.append(&self.asset_root);
    }
}
impl Decodable for AccountState{
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        Ok(Self { nonce: rlp.val_at(0)?, primary_assets: rlp.list_at(1)?, asset_root: rlp.val_at(2)? })
    }
}