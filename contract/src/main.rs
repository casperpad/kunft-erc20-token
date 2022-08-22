#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("target arch should be wasm32: compile with '--target wasm32-unknown-unknown'");

extern crate alloc;

mod address;
mod allowances;
mod balances;
pub mod constants;
mod detail;
pub mod entry_points;
mod error;
mod total_supply;

use alloc::{
    format,
    string::{String, ToString},
};

use once_cell::unsync::OnceCell;

use casper_contract::{
    contract_api::{runtime, storage},
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{contracts::NamedKeys, runtime_args, CLValue, Key, RuntimeArgs, URef, U256};

pub use address::Address;
use constants::{
    ADDRESS_RUNTIME_ARG_NAME, ALLOWANCES_KEY_NAME, AMOUNT_RUNTIME_ARG_NAME, BALANCES_KEY_NAME,
    DECIMALS_KEY_NAME, DECIMALS_RUNTIME_ARG_NAME, NAME_KEY_NAME, NAME_RUNTIME_ARG_NAME,
    OWNER_RUNTIME_ARG_NAME, RECIPIENT_RUNTIME_ARG_NAME, SPENDER_RUNTIME_ARG_NAME, SYMBOL_KEY_NAME,
    SYMBOL_RUNTIME_ARG_NAME, TOTAL_SUPPLY_KEY_NAME, TOTAL_SUPPLY_RUNTIME_ARG_NAME,
};
use contract_utils::{AdminControl, ContractContext, OnChainContractStorage};
pub use error::Error;

#[derive(Default)]
pub struct ERC20 {
    balances_uref: OnceCell<URef>,
    allowances_uref: OnceCell<URef>,
    total_supply_uref: OnceCell<URef>,
}

impl ERC20 {
    fn total_supply_uref(&self) -> URef {
        *self
            .total_supply_uref
            .get_or_init(total_supply::total_supply_uref)
    }

    fn read_total_supply(&self) -> U256 {
        total_supply::read_total_supply_from(self.total_supply_uref())
    }

    fn write_total_supply(&self, total_supply: U256) {
        total_supply::write_total_supply_to(self.total_supply_uref(), total_supply)
    }

    fn balances_uref(&self) -> URef {
        *self.balances_uref.get_or_init(balances::get_balances_uref)
    }

    fn read_balance(&self, owner: Address) -> U256 {
        balances::read_balance_from(self.balances_uref(), owner)
    }

    fn write_balance(&mut self, owner: Address, amount: U256) {
        balances::write_balance_to(self.balances_uref(), owner, amount)
    }

    fn allowances_uref(&self) -> URef {
        *self
            .allowances_uref
            .get_or_init(allowances::allowances_uref)
    }

    fn read_allowance(&self, owner: Address, spender: Address) -> U256 {
        allowances::read_allowance_from(self.allowances_uref(), owner, spender)
    }

    fn write_allowance(&mut self, owner: Address, spender: Address, amount: U256) {
        allowances::write_allowance_to(self.allowances_uref(), owner, spender, amount)
    }

    fn transfer_balance(
        &mut self,
        sender: Address,
        recipient: Address,
        amount: U256,
    ) -> Result<(), Error> {
        balances::transfer_balance(self.balances_uref(), sender, recipient, amount)
    }

    pub fn name(&self) -> String {
        detail::read_from(NAME_KEY_NAME)
    }

    pub fn symbol(&self) -> String {
        detail::read_from(SYMBOL_KEY_NAME)
    }

    pub fn decimals(&self) -> u8 {
        detail::read_from(DECIMALS_KEY_NAME)
    }

    pub fn total_supply(&self) -> U256 {
        self.read_total_supply()
    }

    pub fn balance_of(&self, owner: Address) -> U256 {
        self.read_balance(owner)
    }

    pub fn transfer(&mut self, recipient: Address, amount: U256) -> Result<(), Error> {
        let sender = detail::get_immediate_caller_address()?;
        self.transfer_balance(sender, recipient, amount)
    }

    pub fn transfer_from(
        &mut self,
        owner: Address,
        recipient: Address,
        amount: U256,
    ) -> Result<(), Error> {
        let spender = detail::get_immediate_caller_address()?;
        if amount.is_zero() {
            return Ok(());
        }
        let spender_allowance = self.read_allowance(owner, spender);
        let new_spender_allowance = spender_allowance
            .checked_sub(amount)
            .ok_or(Error::InsufficientAllowance)?;
        self.transfer_balance(owner, recipient, amount)?;
        self.write_allowance(owner, spender, new_spender_allowance);
        Ok(())
    }

    pub fn approve(&mut self, spender: Address, amount: U256) -> Result<(), Error> {
        let owner = detail::get_immediate_caller_address()?;
        self.write_allowance(owner, spender, amount);
        Ok(())
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.read_allowance(owner, spender)
    }

    pub fn mint(&mut self, owner: Address, amount: U256) -> Result<(), Error> {
        let new_balance = {
            let balance = self.read_balance(owner);
            balance.checked_add(amount).ok_or(Error::Overflow)?
        };
        let new_total_supply = {
            let total_supply: U256 = self.read_total_supply();
            total_supply.checked_add(amount).ok_or(Error::Overflow)?
        };
        self.write_balance(owner, new_balance);
        self.write_total_supply(new_total_supply);
        Ok(())
    }
    pub fn burn(&mut self, owner: Address, amount: U256) -> Result<(), Error> {
        let new_balance = {
            let balance = self.read_balance(owner);
            balance
                .checked_sub(amount)
                .ok_or(Error::InsufficientBalance)?
        };
        let new_total_supply = {
            let total_supply = self.read_total_supply();
            total_supply.checked_sub(amount).ok_or(Error::Overflow)?
        };
        self.write_balance(owner, new_balance);
        self.write_total_supply(new_total_supply);
        Ok(())
    }
}

#[derive(Default)]
struct SwprToken(OnChainContractStorage);

impl ContractContext<OnChainContractStorage> for SwprToken {
    fn storage(&self) -> &OnChainContractStorage {
        &self.0
    }
}
impl AdminControl<OnChainContractStorage> for SwprToken {}

impl SwprToken {
    fn constructor(&mut self) {
        AdminControl::init(self);
    }
}

#[no_mangle]
pub extern "C" fn constructor() {
    SwprToken::default().constructor();
    let default_admin = runtime::get_caller();
    SwprToken::default().add_admin_without_checked(Key::from(default_admin));
}

#[no_mangle]
pub extern "C" fn name() {
    let name = ERC20::default().name();
    runtime::ret(CLValue::from_t(name).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn symbol() {
    let symbol = ERC20::default().symbol();
    runtime::ret(CLValue::from_t(symbol).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn decimals() {
    let decimals = ERC20::default().decimals();
    runtime::ret(CLValue::from_t(decimals).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn total_supply() {
    let total_supply = ERC20::default().total_supply();
    runtime::ret(CLValue::from_t(total_supply).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn balance_of() {
    let address: Address = runtime::get_named_arg(ADDRESS_RUNTIME_ARG_NAME);
    let balance = ERC20::default().balance_of(address);
    runtime::ret(CLValue::from_t(balance).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn transfer() {
    let recipient: Address = runtime::get_named_arg(RECIPIENT_RUNTIME_ARG_NAME);
    let amount: U256 = runtime::get_named_arg(AMOUNT_RUNTIME_ARG_NAME);

    ERC20::default()
        .transfer(recipient, amount)
        .unwrap_or_revert();
}

#[no_mangle]
pub extern "C" fn approve() {
    let spender: Address = runtime::get_named_arg(SPENDER_RUNTIME_ARG_NAME);
    let amount: U256 = runtime::get_named_arg(AMOUNT_RUNTIME_ARG_NAME);

    ERC20::default().approve(spender, amount).unwrap_or_revert();
}

#[no_mangle]
pub extern "C" fn allowance() {
    let owner: Address = runtime::get_named_arg(OWNER_RUNTIME_ARG_NAME);
    let spender: Address = runtime::get_named_arg(SPENDER_RUNTIME_ARG_NAME);
    let val = ERC20::default().allowance(owner, spender);
    runtime::ret(CLValue::from_t(val).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn transfer_from() {
    let owner: Address = runtime::get_named_arg(OWNER_RUNTIME_ARG_NAME);
    let recipient: Address = runtime::get_named_arg(RECIPIENT_RUNTIME_ARG_NAME);
    let amount: U256 = runtime::get_named_arg(AMOUNT_RUNTIME_ARG_NAME);
    ERC20::default()
        .transfer_from(owner, recipient, amount)
        .unwrap_or_revert();
}

#[no_mangle]
pub extern "C" fn mint() {
    SwprToken::default().assert_caller_is_admin();

    let owner: Address = runtime::get_named_arg(OWNER_RUNTIME_ARG_NAME);
    let amount: U256 = runtime::get_named_arg(AMOUNT_RUNTIME_ARG_NAME);
    ERC20::default().mint(owner, amount).unwrap_or_revert();
}

#[no_mangle]
pub extern "C" fn burn() {
    SwprToken::default().assert_caller_is_admin();

    let owner: Address = runtime::get_named_arg(OWNER_RUNTIME_ARG_NAME);
    let amount: U256 = runtime::get_named_arg(AMOUNT_RUNTIME_ARG_NAME);
    ERC20::default().burn(owner, amount).unwrap_or_revert();
}

#[no_mangle]
pub extern "C" fn add_admin() {
    let new_admin: Address = runtime::get_named_arg("admin");
    SwprToken::default().add_admin(new_admin.into());
}

#[no_mangle]
pub extern "C" fn remove_admin() {
    let admin: Address = runtime::get_named_arg("admin");
    SwprToken::default().disable_admin(admin.into());
}

#[no_mangle]
fn call() {
    let name: String = runtime::get_named_arg(NAME_RUNTIME_ARG_NAME);
    let symbol: String = runtime::get_named_arg(SYMBOL_RUNTIME_ARG_NAME);
    let decimals: u8 = runtime::get_named_arg(DECIMALS_RUNTIME_ARG_NAME);
    let total_supply = runtime::get_named_arg(TOTAL_SUPPLY_RUNTIME_ARG_NAME);

    let balances_uref = storage::new_dictionary(BALANCES_KEY_NAME).unwrap_or_revert();
    let allowances_uref = storage::new_dictionary(ALLOWANCES_KEY_NAME).unwrap_or_revert();
    // We need to hold on a RW access rights because tokens can be minted or burned.
    let total_supply_uref = storage::new_uref(total_supply).into_read_write();

    let mut named_keys = NamedKeys::new();

    let name_key = {
        let name_uref = storage::new_uref(name.clone()).into_read();
        Key::from(name_uref)
    };

    let symbol_key = {
        let symbol_uref = storage::new_uref(symbol).into_read();
        Key::from(symbol_uref)
    };

    let decimals_key = {
        let decimals_uref = storage::new_uref(decimals).into_read();
        Key::from(decimals_uref)
    };

    let total_supply_key = Key::from(total_supply_uref);

    let balances_dictionary_key = {
        // Sets up initial balance for the caller - either an account, or a contract.
        let caller = detail::get_caller_address().unwrap_or_revert();
        balances::write_balance_to(balances_uref, caller, total_supply);

        runtime::remove_key(BALANCES_KEY_NAME);

        Key::from(balances_uref)
    };

    let allowances_dictionary_key = {
        runtime::remove_key(ALLOWANCES_KEY_NAME);

        Key::from(allowances_uref)
    };

    named_keys.insert(NAME_KEY_NAME.to_string(), name_key);
    named_keys.insert(SYMBOL_KEY_NAME.to_string(), symbol_key);
    named_keys.insert(DECIMALS_KEY_NAME.to_string(), decimals_key);
    named_keys.insert(BALANCES_KEY_NAME.to_string(), balances_dictionary_key);
    named_keys.insert(ALLOWANCES_KEY_NAME.to_string(), allowances_dictionary_key);
    named_keys.insert(TOTAL_SUPPLY_KEY_NAME.to_string(), total_supply_key);

    let (contract_hash, _version) = storage::new_locked_contract(
        entry_points::default(),
        Some(named_keys),
        Some(format!("{}_contract_package_hash", name)),
        None,
    );

    runtime::put_key(&format!("{}_contract_hash", name), contract_hash.into());

    // Hash of the installed contract will be reachable through named keys.

    let _: () = runtime::call_contract(contract_hash, "constructor", runtime_args! {});
}
