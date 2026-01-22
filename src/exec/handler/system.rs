//보안 중요함

//기존 토큰의 메타데이터에 수정자로 접근이 가능하므로, 덮어쓰기도 못하게 하는 등 여러 보안 필요

use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, GlobalBalance, StateDiff, TokenInfo, TokenTicker}}, exec::schema::RegisterTokenParams};


/**
 * param input:
 * pub name: String,
 * pub symbol: TokenTicker,
 * pub admin: Address,
 * pub initial_supply: u64, 
 * pub decimals: u8,

 */
pub fn register_token(
    state: &mut GlobalBalance,
    from: Address,
    to: Address, //None 0
    value: u64, //None 0
    fee: u64,
    params: serde_json::Value,
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{
    let json_params: RegisterTokenParams = serde_json::from_value(params).expect("INVALID_JSON");
    
    let ticker = json_params.symbol.to_uppercase();
    if !state.gov_shares.contains_key(&from){return Err("PERMISSION_DENIED".into());}
    let threshold = 20 as u64;
    if state.gov_shares.get(&from).unwrap() < &threshold {return Err("THRESHOLD_ERROR".into());}
    if state.token_metadata.contains_key(&ticker){
        return Err(format!("TOKEN_ALREADY_EXISTS_{ticker}"));
    }
    if ticker.len() < 2 || ticker.len() > 10 || !ticker.chars().all(|c| c.is_alphabetic()){
        return Err("INVALID_TOKEN_TICKER_FORMAT".into());
    }
    let new_metadata = TokenInfo::new(
        &json_params.name,
        &ticker,
        json_params.decimals,
        json_params.initial_supply,
        to,
    );
    state.pay_gas(&from, fee,  cur_height, db)?;
    state.token_metadata.insert(ticker.clone(),new_metadata);
    state.add_balance(&to, &ticker, value, cur_height, db);
    println!("[NEW TOKEN] Registered: {ticker} by {}", hex::encode(from));
    state.inc_nonce(&from, cur_height, db);
    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read(&to, cur_height, db));
    changed_accounts.insert(from, state.get_account_read(&from, cur_height, db));
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: Some(ticker)
    })
}

