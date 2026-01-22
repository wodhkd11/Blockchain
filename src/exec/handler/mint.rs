use std::collections::HashMap;

use crate::{block::{db::Storage, types::{Account, Address, GlobalBalance, StateDiff, TokenTicker}}, exec::schema::*};

pub fn handle_mint(
    state: &mut GlobalBalance,
    from: Address,
    to: Address,
    value: u64,
    fee: u64,
    params: serde_json::Value, // ticker가 들어있음. 규칙 확인후 권한 있는지 확인
    cur_height: u64,
    db: &Storage
) -> Result<StateDiff, String>{

    let (min_gas, gas_token, gov_threshold, gov_token) = {
        let rule = &state.config;
        (rule.min_gas_price, rule.gas_token.clone(), rule.governance_threshold, rule.gov_token.clone())
    };

    let json_params: MintParams = serde_json::from_value(params)
        .map_err(|e| format!("JSON PARSING ERROR: {e}"))?;
    let token = json_params.ticker;

    if !state.token_metadata.contains_key(&token){ return Err("Unsupported tokena".into()); }

    let gas_token_balance = state.get_token_balance(&from, &gas_token, cur_height, db);
    if gas_token_balance < fee || fee < min_gas { return Err("Insufficient balance for gas fee".into()); }

    let gov_balance = state.get_token_balance(&from, &gov_token, cur_height, db);
    if gov_balance < gov_threshold {
        return Err("[GOVERNANCE]: Permission Denied".into());
    }
    state.pay_gas(&from, fee, cur_height, db)?;
    state.add_balance(&to, &token, value, cur_height, db);
    
    state.inc_nonce(&from, cur_height, db);
    let mut changed_accounts = HashMap::new();
    changed_accounts.insert(to, state.get_account_read(&to, cur_height, db));
    changed_accounts.insert(from, state.get_account_read(&from, cur_height, db));
    Ok(StateDiff{
        accounts: changed_accounts,
        token_changed: Some(token),
    })
}
