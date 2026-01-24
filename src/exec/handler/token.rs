use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, Balance, GlobalBalance, StateDiff, TokenTicker}}, exec::schema::*};



pub fn handle_transfer(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: Balance,
    fee: Balance,
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
        if fee < Balance::from(min_gas) {
            return Err(format!("GAS FEE NEED {min_gas}"));
        }
    }
    let gas_tkn = state.config.gas_token.clone();
    {
        let from_acc = state.get_account_mut(&from, cur_height, db);
        from_acc.pay_gas(fee, &gas_tkn);
        from_acc.sub_balance(&token, value);
        from_acc.inc_nonce();
    }
    {
        let to_acc = state.get_account_mut(&to, cur_height, db);
        to_acc.add_balance(&token, value.saturating_sub(fee));
    }

    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read(&to, cur_height, db));
    changed_accounts.insert(from,state.get_account_read(&from, cur_height, db));
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: None,
    })
    
}
