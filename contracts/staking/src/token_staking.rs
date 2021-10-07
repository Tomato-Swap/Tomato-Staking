#![no_main]
#![no_std]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("target arch should be wasm32: compile with '--target wasm32-unknown-unknown'");

extern crate alloc;

mod helpers;
mod entry_points;
mod named_keys;
pub mod constants;

use crate::helpers::{ set_key, get_key, get_immediate_caller_address, make_dictionary_item_key, dictionary_read, dictionary_write };
use crate::entry_points::default;

use crate::constants::{
    STAKE_CONTRACT_KEY_NAME, REWARD_TOKEN_HASH_KEY_NAME, STAKE_TOKEN_HASH_KEY_NAME, REWARD_RATE_KEY_NAME,
    TOTAL_SUPPLY_KEY_NAME, AMOUNT_KEY_NAME, BALANCES_KEY_NAME,
};

use alloc::string::String;

use casper_erc20::constants::{
    TRANSFER_ENTRY_POINT_NAME, TRANSFER_FROM_ENTRY_POINT_NAME, APPROVE_ENTRY_POINT_NAME,
    ALLOWANCE_ENTRY_POINT_NAME, BALANCE_OF_ENTRY_POINT_NAME, OWNER_RUNTIME_ARG_NAME,
    RECIPIENT_RUNTIME_ARG_NAME, AMOUNT_RUNTIME_ARG_NAME
};
use casper_contract::{contract_api::{runtime, storage}, unwrap_or_revert::UnwrapOrRevert};
use casper_types::{contracts::{NamedKeys, Address, URef} , CLValue, U256, ContractHash, Key, Address, URef, RuntimeArgs, runtime_args };

#[no_mangle]
fn call() {
    
    let stake_contract_name: String = runtime::get_named_arg(STAKE_CONTRACT_KEY_NAME);
    let stake_token_key: Key = runtime::get_named_arg(STAKE_TOKEN_KEY_NAME);
    let reward_token_key: Key = runtime::get_named_arg(REWARD_TOKEN_KEY_NAME);
    let reward_rate: U256 = runtime::get_named_arg(REWARD_RATE_KEY_NAME);

    let named_keys: NamedKeys = named_keys::default(stake_contract_name, stake_token_key, reward_token_key, reward_rate);

    let (contract_hash, _version) =
            storage::new_locked_contract(entry_points::default(), Some(named_keys), None, None);
    
    // Hash of the installed contract will be reachable through named keys.
    runtime::put_key(stake_contract_name: &str, Key::from(contract_hash));

    //let key: Key = runtime::get_key(CONTRACT_KEY_NAME).unwrap_or_revert();
    //let hash: HashAddr = key.into_hash().unwrap_or_revert();
    //let contract_hash = ContractHash::new(hash);

    //let _: () = runtime::call_contract(contract_hash, "init", RuntimeArgs::new());

}

#[no_mangle]
pub extern "C" fn stake() {
    
    let amount: U256 = runtime::get_named_arg(AMOUNT_KEY_NAME);

    let staker = get_immediate_caller_address().unwrap_or_revert();
    let balances_uref = get_key(BALANCES_KEY_NAME).unwrap_or_revert();
    let stake_contract = runtime::get_caller().unwrap_or_revert();

    update_reward();
    
    // update total_supply
    totall_supply_add(amount);

    // update balance of caller
    add_to_dictionary(balances_uref, staker, amount);

    // Transfer `amount` of Stake Token from caller to the stake contract
    let stake_token_contract: ContractHash = get_key(STAKE_TOKEN_HASH_KEY_NAME);
    runtime::call_contract(stake_token_contract, TRANSFER_FROM_ENTRY_POINT_NAME, runtime_args!{
        OWNER_RUNTIME_ARG_NAME => staker,
        RECIPIENT_RUNTIME_ARG_NAME => stake_contract,
        AMOUNT_RUNTIME_ARG_NAME => amount
    });

}

#[no_mangle]
pub extern "C" fn withdraw() {
    
    let amount: U256 = runtime::get_named_arg(AMOUNT_KEY_NAME).unwrap_or_revert();

    let staker: = get_immediate_caller_address().unwrap_or_revert();
    let balances_uref = get_key(BALANCES_KEY_NAME);

    update_reward();

    // update total_supply
    total_supply_sub(amount);

    // update balance of caller
    sub_to_dictionary(balances_uref, staker: Address, amount: U256);

    // Transfer `amount` of Stake Token from the stake contract to caller
    let stake_token_contract: ContractHash = get_key(STAKE_TOKEN_HASH_KEY_NAME).unwrap_or_revert();
    runtime::call_contract(stake_token_contract, TRANSFER_ENTRY_POINT_NAME, runtime_args!{
        RECIPIENT_RUNTIME_ARG_NAME => staker,
        AMOUNT_RUNTIME_ARG_NAME => amount
    });

}

#[no_mangle]
pub extern "C" fn get_reward() {
    
    update_reward();

    let staker: = get_immediate_caller_address().unwrap_or_revert();
    let balances_uref = get_key(BALANCES_KEY_NAME);
    let rewards_uref = get_key(REWARDS_KEY_NAME);

    // get reward_value of the caller stored in "rewards" dictionary
    let staker_reward: U256 = dictionary_read(rewards_uref, staker);
    
    // set reward_value of the caller in the dictionary to 0
    dictionary_write(rewards_uref, staker, U256::from(0));

    // Transfer `amount` of Reward Token to caller
    let stake_token_contract: ContractHash = get_key(STAKE_TOKEN_HASH_KEY_NAME).unwrap_or_revert();
    runtime::call_contract(stake_token_contract, TRANSFER_ENTRY_POINT_NAME, runtime_args!{
        RECIPIENT_RUNTIME_ARG_NAME => staker,
        AMOUNT_RUNTIME_ARG_NAME => staker_reward
    });

}

#[no_mangle]
 fn update_reward() {
    
    // update reward_per_token_stored

    // update last_update_time
}

#[no_mangle]
 fn reward_per_token() {
    
    //
}

fn add_to_dictionary(
    dictionary_uref: URef,
    staker: Address,
    amount: U256,
) -> Result<(), Error> {
    if amount.is_zero() {
        return Ok(());
    }

    let new_staker_balance = {
        let staker_balance = dictionary_read(dictionary_uref, staker);
        staker_balance
            .checked_add(amount)
            .ok_or(Error::Overflow)?
    };

    dictionary_write(dictionary_uref, staker, new_staker_balance);

    Ok(())
}

fn sub_to_dictionary(
    dictionary_uref: URef,
    staker: Address,
    amount: U256,
) -> Result<(), Error> {
    if amount.is_zero() {
        return Ok(());
    }

    let new_staker_balance = {
        let staker_balance = dictionary_read(dictionary_uref, staker);
        staker_balance
            .checked_sub(amount)
            .ok_or(Error::InsufficientBalance)?
    };

    dictionary_write(dictionary_uref, staker, new_staker_balance);

    Ok(())
}

fn totall_supply_add(amount: U256) {
    
    let new_total_supply: U256 = {
        let total_supply: U256 = get_key(TOTAL_SUPPLY_KEY_NAME);
        total_supply
            .checked_add(amount)
            .ok_or(Error::Overflow)?
    };

    set_key(TOTAL_SUPPLY_KEY_NAME, new_total_supply);

}

fn total_supply_sub(amount: U256) {
    
    let new_total_supply: U256 = {
        let total_supply: U256 = get_key(TOTAL_SUPPLY_KEY_NAME);
        total_supply
            .checked_sub(amount)
            .ok_or(Error::InsufficientBalance)?
    };

    set_key(TOTAL_SUPPLY_KEY_NAME, new_total_supply);

}


fn stake_token_deposit() {
    let stake_token_hash: ContractHash = get_key(STAKE_TOKEN_HASH_KEY_NAME);
    runtime::call_contract(stake_token_hash, "deposit", runtime_args!{
        
    });
}

fn stake_token_withdraw() {
    let reward_token_hash: ContractHash = get_key(STAKE_TOKEN_HASH_KEY_NAME);
    runtime::call_contract(stake_token_hash, "transfer", runtime_args!{
        
    });
}

fn reward_token_transfer() {
    let reward_token_hash: ContractHash = get_key(REWARD_TOKEN_HASH_KEY_NAME);
    runtime::call_contract(wcspr_contract_hash, "deposit", runtime_args!{
        
    });
}