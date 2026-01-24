
pub mod decoder;
mod opcodes;
pub mod schema;
pub mod handler;


use std::{collections::{HashMap, HashSet}, fmt::format};

use primitive_types::U256;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use crate::{block::{db::Storage, transaction::TransactionData, types::{Account, Address, Balance, BlockData, GlobalBalance, StateDiff, TokenTicker}}, exec::{handler::{admin::config_update, mint::handle_mint, system::register_token, token::handle_transfer}, opcodes::*}};

// pub enum Instruction{
    // RegisterToken(RegisterTokenParams),
    // Transfer(TransferParams),
    // Mint,
    // Burn,

// }

/*
Transaction format
json{
sender
receiver
value
nonce
Payload{
opcode
fee
data
}
}
*/

/*
pub const OP_SYSTEM_REGISTER_TOKEN: u8 = 0x00;
pub const OP_TOKEN_MINT: u8 = 0x01;
pub const OP_TOKEN_TRANSFER: u8 = 0x02;
pub const OP_TOKEN_BURN: u8 = 0x03;
pub const OP_PAY_PURCHASE: u8 = 0x04;
 */
#[serde_as]
#[derive(Deserialize)]
pub struct RawPayload{
    pub opcode: u8,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fee: Option<Balance>,
    pub data: serde_json::Value,
}

pub fn apply_transaction(state: &mut GlobalBalance, tx: &TransactionData, cur_height: u64, db:&Storage)
 -> Result<StateDiff, String>{
    //let current_config = &state.config; //권환관련되서 로직해야됨

    let raw_payload: RawPayload = serde_json::from_slice(&tx.payload)
        .map_err(|_| "Invalid Payload JSON")?;
    let opcode = raw_payload.opcode;
    // let f = raw_payload.fee;
    // let fee = match f{
    //     Some(v) => {
    //         if v == 0{
                
    //         }
    //     }
    // }
    let fee = match raw_payload.fee{
        Some(f) => {
            if f == U256::zero(){
                state.config.min_gas_price
            } else{
                if f < state.config.min_gas_price {
                    return Err(format!("INSUFFICIENT_GAS_FEE"));
                }
                f
            }
        },
        None => {state.config.min_gas_price}
    };

    match opcode{
        OP_SYSTEM_REGISTER_TOKEN => {
            register_token(state, tx.sender, tx.receiver, Balance::from(tx.value), Balance::from(fee), raw_payload.data, cur_height, &db)
        },
        OP_TOKEN_TRANSFER => {
            handle_transfer(state, tx.sender, tx.receiver, Balance::from(tx.value), Balance::from(fee), raw_payload.data, cur_height, &db)
        }
        OP_TOKEN_MINT => {
            handle_mint(state, tx.sender, tx.receiver, Balance::from(tx.value), Balance::from(fee), raw_payload.data, cur_height, &db)
        }
        OP_CONFIG => {
            config_update(state, tx.sender, tx.receiver, Balance::from(tx.value), Balance::from(fee), raw_payload.data, cur_height, &db)
        }
        _ => Err("OP NOT FOUND".to_string())
    }
}

pub fn execute_block(state: &mut GlobalBalance, block: &BlockData, db: &Storage)
 -> Result<(HashMap<Address, Account>, HashSet<TokenTicker>), String>{
    let mut state_updates = HashMap::new();
    let mut token_updates = HashSet::new();

    for tx in &block.body{
        let diff = apply_transaction(state, &tx.tx_info, block.header.height, db)
            .map_err(|e| format!("Transaction failed: {}",e ))?;
        
        for (addr, acc) in diff.accounts{
            state_updates.insert(addr, acc);
        }
        if let Some(ticker) = diff.token_changed{
            token_updates.insert(ticker);
        }
    }
    state.distribute_gas(block.header.height, db);
    Ok((state_updates, token_updates))
}