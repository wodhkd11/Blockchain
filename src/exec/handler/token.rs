use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, GlobalBalance, StateDiff, TokenTicker}}, exec::schema::*};



pub fn handle_transfer(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: u64,
    fee: u64,
    params: serde_json::Value,
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{
    let (min_gas, gas_token) = {
        let rule = &state.config;
        (rule.min_gas_price, rule.gas_token.clone())
    };
    let json_params:TransferParams = serde_json::from_value(params)
        .map_err(|e| format!("JSON PARSING FAILED:{e}"))?;
    let token = &json_params.ticker;

    if !state.token_metadata.contains_key(token){
        return Err("Unsupported token".into());
    }
    let gas_balance = state.get_token_balance(&from, &gas_token, cur_height, db);
    if token == &gas_token {
        if gas_balance < value.saturating_add(fee){
            return Err("INSUFFICIENT_KRW".into());
        }
    }else{
        if gas_balance < fee{
            return Err("INSUFFICIENT_GAS".into());
        }
        let token_balance = state.get_token_balance(&from, token, cur_height, db);
        if token_balance < value{
            return Err(format!("INSUFFICIENT_{token}_BALANCE"));
        }
        if fee < min_gas {
            return Err(format!("GAS FEE NEED {min_gas}"));
        }
    }

    state.pay_gas(&from, fee, cur_height, db)?;
    match state.sub_balance(&from, token, value, cur_height, db){
        Ok(()) => state.add_balance(&to, &token, value, cur_height, db),
        Err(e) => return Err(e),
    }

    state.inc_nonce(&from, cur_height, db);
    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read(&to, cur_height, db));
    changed_accounts.insert(from,state.get_account_read(&from, cur_height, db));
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: None,
    })
    
}
