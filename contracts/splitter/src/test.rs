#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Env};

struct Setup {
    env: Env,
    client: SplitterClient<'static>,
}

fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Splitter, ());
    let client = SplitterClient::new(&env, &contract_id);
    Setup { env, client }
}

fn fund_token(env: &Env, payer: &Address, amount: i128) -> (Address, token::Client<'static>) {
    let admin = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(admin);
    let token_id = sac.address();
    token::StellarAssetClient::new(env, &token_id).mint(payer, &amount);
    (token_id.clone(), token::Client::new(env, &token_id))
}

#[test]
fn create_and_get() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone()],
        &vec![&s.env, 6_000, 4_000],
        &None,
    );

    assert_eq!(id, 0);
    assert_eq!(s.client.split_count(), 1);

    let split = s.client.get_split(&id);
    assert_eq!(split.recipients, vec![&s.env, a, b]);
    assert_eq!(split.shares, vec![&s.env, 6_000, 4_000]);
    assert_eq!(split.controller, None);
}

#[test]
fn rejects_invalid_splits() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);

    let no_recipients = s
        .client
        .try_create_split(&creator, &vec![&s.env], &vec![&s.env], &None);
    assert_eq!(no_recipients, Err(Ok(Error::NoRecipients)));

    let mismatch = s.client.try_create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone()],
        &vec![&s.env, 10_000],
        &None,
    );
    assert_eq!(mismatch, Err(Ok(Error::LengthMismatch)));

    let zero_share = s.client.try_create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone()],
        &vec![&s.env, 10_000, 0],
        &None,
    );
    assert_eq!(zero_share, Err(Ok(Error::ZeroShare)));

    let bad_total = s.client.try_create_split(
        &creator,
        &vec![&s.env, a, b],
        &vec![&s.env, 5_000, 4_000],
        &None,
    );
    assert_eq!(bad_total, Err(Ok(Error::BadShareTotal)));
}

#[test]
fn pay_distributes_by_shares() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    let c = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, token_client) = fund_token(&s.env, &payer, 1_000_000);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone(), c.clone()],
        &vec![&s.env, 5_000, 3_000, 2_000],
        &None,
    );

    s.client.pay(&payer, &id, &token_id, &100_000);

    assert_eq!(token_client.balance(&a), 50_000);
    assert_eq!(token_client.balance(&b), 30_000);
    assert_eq!(token_client.balance(&c), 20_000);
    assert_eq!(token_client.balance(&payer), 900_000);
}

#[test]
fn rounding_dust_goes_to_last_recipient() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    let c = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, token_client) = fund_token(&s.env, &payer, 100);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone(), c.clone()],
        &vec![&s.env, 3_333, 3_333, 3_334],
        &None,
    );

    s.client.pay(&payer, &id, &token_id, &100);

    assert_eq!(token_client.balance(&a), 33);
    assert_eq!(token_client.balance(&b), 33);
    assert_eq!(token_client.balance(&c), 34);
    assert_eq!(token_client.balance(&payer), 0);
}

#[test]
fn rejects_non_positive_amounts() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, _) = fund_token(&s.env, &payer, 1_000);

    let id = s
        .client
        .create_split(&creator, &vec![&s.env, a], &vec![&s.env, 10_000], &None);

    let zero = s.client.try_pay(&payer, &id, &token_id, &0);
    assert_eq!(zero, Err(Ok(Error::InvalidAmount)));

    let negative = s.client.try_pay(&payer, &id, &token_id, &-5);
    assert_eq!(negative, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn pay_unknown_split_fails() {
    let s = setup();
    let payer = Address::generate(&s.env);
    let (token_id, _) = fund_token(&s.env, &payer, 1_000);

    let result = s.client.try_pay(&payer, &99, &token_id, &100);
    assert_eq!(result, Err(Ok(Error::SplitNotFound)));
}

#[test]
fn controller_can_update_mutable_split() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let controller = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &Some(controller.clone()),
    );

    s.client.update_split(
        &id,
        &vec![&s.env, a.clone(), b.clone()],
        &vec![&s.env, 7_000, 3_000],
    );

    let split = s.client.get_split(&id);
    assert_eq!(split.recipients, vec![&s.env, a, b]);
    assert_eq!(split.shares, vec![&s.env, 7_000, 3_000]);
}

#[test]
fn immutable_split_cannot_be_updated() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &None,
    );

    let result = s
        .client
        .try_update_split(&id, &vec![&s.env, b], &vec![&s.env, 10_000]);
    assert_eq!(result, Err(Ok(Error::SplitImmutable)));
}
