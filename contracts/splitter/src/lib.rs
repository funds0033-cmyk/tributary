#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contractmeta, contracttype, token,
    Address, Env, Vec,
};

contractmeta!(key = "name", val = "tributary-splitter");
contractmeta!(
    key = "source",
    val = "https://github.com/tributary-protocol/tributary"
);

pub const TOTAL_SHARES: u32 = 10_000;
pub const MAX_RECIPIENTS: u32 = 32;

const DAY_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = 30 * DAY_LEDGERS;
const TTL_EXTEND_TO: u32 = 120 * DAY_LEDGERS;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NoRecipients = 1,
    LengthMismatch = 2,
    ZeroShare = 3,
    BadShareTotal = 4,
    SplitNotFound = 5,
    SplitImmutable = 6,
    InvalidAmount = 7,
    NothingToDistribute = 8,
    TooManyRecipients = 9,
}

#[contracttype]
#[derive(Clone)]
pub struct Split {
    pub recipients: Vec<Address>,
    pub shares: Vec<u32>,
    pub controller: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Count,
    Split(u64),
    Balance(u64, Address),
    Created(Address),
}

#[contractevent]
#[derive(Clone)]
pub struct SplitCreated {
    #[topic]
    pub id: u64,
    pub creator: Address,
}

#[contractevent]
#[derive(Clone)]
pub struct SplitPaid {
    #[topic]
    pub id: u64,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone)]
pub struct SplitUpdated {
    #[topic]
    pub id: u64,
}

#[contractevent]
#[derive(Clone)]
pub struct ControlTransferred {
    #[topic]
    pub id: u64,
    pub new_controller: Option<Address>,
}

#[contractevent]
#[derive(Clone)]
pub struct Deposited {
    #[topic]
    pub id: u64,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone)]
pub struct Distributed {
    #[topic]
    pub id: u64,
    pub token: Address,
    pub amount: i128,
}

#[contract]
pub struct Splitter;

#[contractimpl]
impl Splitter {
    /// Registers a new split and returns its id. Shares are basis points
    /// and must sum to exactly 10_000. Passing a controller makes the
    /// split mutable by that address; passing None locks it forever.
    pub fn create_split(
        env: Env,
        creator: Address,
        recipients: Vec<Address>,
        shares: Vec<u32>,
        controller: Option<Address>,
    ) -> Result<u64, Error> {
        creator.require_auth();
        validate(&recipients, &shares)?;

        let id: u64 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
        let split = Split {
            recipients,
            shares,
            controller,
        };
        env.storage().persistent().set(&DataKey::Split(id), &split);
        env.storage().instance().set(&DataKey::Count, &(id + 1));
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);

        let index_key = DataKey::Created(creator.clone());
        let mut created: Vec<u64> = env
            .storage()
            .persistent()
            .get(&index_key)
            .unwrap_or_else(|| Vec::new(&env));
        created.push_back(id);
        env.storage().persistent().set(&index_key, &created);

        SplitCreated { id, creator }.publish(&env);
        Ok(id)
    }

    /// Moves `amount` of `token` from the payer to every recipient of the
    /// split in one call. Rounding dust goes to the last recipient.
    pub fn pay(
        env: Env,
        from: Address,
        id: u64,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        from.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let split = load(&env, id)?;
        payout(&env, &split, &from, &token, amount);
        SplitPaid { id, token, amount }.publish(&env);
        Ok(())
    }

    /// Replaces the recipients and shares of a mutable split.
    pub fn update_split(
        env: Env,
        id: u64,
        recipients: Vec<Address>,
        shares: Vec<u32>,
    ) -> Result<(), Error> {
        let mut split = load(&env, id)?;
        let controller = split.controller.clone().ok_or(Error::SplitImmutable)?;
        controller.require_auth();
        validate(&recipients, &shares)?;
        split.recipients = recipients;
        split.shares = shares;
        env.storage().persistent().set(&DataKey::Split(id), &split);
        SplitUpdated { id }.publish(&env);
        Ok(())
    }

    /// Hands control of a mutable split to another address, or locks it
    /// forever when the new controller is None.
    pub fn transfer_control(
        env: Env,
        id: u64,
        new_controller: Option<Address>,
    ) -> Result<(), Error> {
        let mut split = load(&env, id)?;
        let controller = split.controller.clone().ok_or(Error::SplitImmutable)?;
        controller.require_auth();
        split.controller = new_controller.clone();
        env.storage().persistent().set(&DataKey::Split(id), &split);
        ControlTransferred { id, new_controller }.publish(&env);
        Ok(())
    }

    /// Moves funds into the contract and credits them to the split without
    /// paying anyone yet. Useful when money arrives before a distribution
    /// should happen.
    pub fn deposit(
        env: Env,
        from: Address,
        id: u64,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        from.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        load(&env, id)?;
        let vault = env.current_contract_address();
        token::Client::new(&env, &token).transfer(&from, &vault, &amount);
        let key = DataKey::Balance(id, token.clone());
        let held: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(held + amount));
        Deposited { id, token, amount }.publish(&env);
        Ok(())
    }

    /// Pays out everything credited to the split for the given token.
    /// Anyone can call this; the routing table decides where funds go.
    pub fn distribute(env: Env, id: u64, token: Address) -> Result<i128, Error> {
        let split = load(&env, id)?;
        let key = DataKey::Balance(id, token.clone());
        let amount: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if amount <= 0 {
            return Err(Error::NothingToDistribute);
        }
        env.storage().persistent().remove(&key);
        payout(
            &env,
            &split,
            &env.current_contract_address(),
            &token,
            amount,
        );
        Distributed { id, token, amount }.publish(&env);
        Ok(amount)
    }

    /// Returns the exact per-recipient amounts a payment of `amount` would
    /// produce, without moving any funds.
    pub fn preview_payout(env: Env, id: u64, amount: i128) -> Result<Vec<i128>, Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let split = load(&env, id)?;
        Ok(amounts(&env, &split, amount))
    }

    pub fn balance(env: Env, id: u64, token: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id, token))
            .unwrap_or(0)
    }

    pub fn get_split(env: Env, id: u64) -> Result<Split, Error> {
        load(&env, id)
    }

    pub fn splits_of(env: Env, creator: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::Created(creator))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn split_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }
}

fn validate(recipients: &Vec<Address>, shares: &Vec<u32>) -> Result<(), Error> {
    if recipients.is_empty() {
        return Err(Error::NoRecipients);
    }
    if recipients.len() > MAX_RECIPIENTS {
        return Err(Error::TooManyRecipients);
    }
    if recipients.len() != shares.len() {
        return Err(Error::LengthMismatch);
    }
    let mut total: u32 = 0;
    for share in shares.iter() {
        if share == 0 {
            return Err(Error::ZeroShare);
        }
        total = total.checked_add(share).ok_or(Error::BadShareTotal)?;
    }
    if total != TOTAL_SHARES {
        return Err(Error::BadShareTotal);
    }
    Ok(())
}

fn amounts(env: &Env, split: &Split, amount: i128) -> Vec<i128> {
    let mut out = Vec::new(env);
    let last = split.recipients.len() - 1;
    let mut assigned: i128 = 0;
    for i in 0..split.recipients.len() {
        let part = if i == last {
            amount - assigned
        } else {
            amount * split.shares.get_unchecked(i) as i128 / TOTAL_SHARES as i128
        };
        out.push_back(part);
        assigned += part;
    }
    out
}

fn payout(env: &Env, split: &Split, from: &Address, token: &Address, amount: i128) {
    let client = token::Client::new(env, token);
    let parts = amounts(env, split, amount);
    for i in 0..split.recipients.len() {
        let part = parts.get_unchecked(i);
        if part > 0 {
            client.transfer(from, &split.recipients.get_unchecked(i), &part);
        }
    }
}

fn load(env: &Env, id: u64) -> Result<Split, Error> {
    let key = DataKey::Split(id);
    let split = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::SplitNotFound)?;
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    Ok(split)
}

#[cfg(test)]
mod test;
