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
fn tracks_splits_by_creator() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let other = Address::generate(&s.env);
    let a = Address::generate(&s.env);

    s.client.create_split(
        &creator,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &None,
    );
    s.client.create_split(
        &other,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &None,
    );
    s.client
        .create_split(&creator, &vec![&s.env, a], &vec![&s.env, 10_000], &None);

    assert_eq!(s.client.splits_of(&creator), vec![&s.env, 0, 2]);
    assert_eq!(s.client.splits_of(&other), vec![&s.env, 1]);
    let stranger = Address::generate(&s.env);
    assert_eq!(s.client.splits_of(&stranger), vec![&s.env]);
}

#[test]
fn rejects_too_many_recipients() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let mut recipients = vec![&s.env];
    let mut shares = vec![&s.env];
    for _ in 0..33 {
        recipients.push_back(Address::generate(&s.env));
        shares.push_back(300u32);
    }

    let result = s
        .client
        .try_create_split(&creator, &recipients, &shares, &None);
    assert_eq!(result, Err(Ok(Error::TooManyRecipients)));
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
fn preview_matches_actual_payout() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    let c = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, token_client) = fund_token(&s.env, &payer, 1_000);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone(), c.clone()],
        &vec![&s.env, 3_333, 3_333, 3_334],
        &None,
    );

    let preview = s.client.preview_payout(&id, &1_000);
    assert_eq!(preview, vec![&s.env, 333, 333, 334]);

    s.client.pay(&payer, &id, &token_id, &1_000);
    assert_eq!(token_client.balance(&a), preview.get_unchecked(0));
    assert_eq!(token_client.balance(&b), preview.get_unchecked(1));
    assert_eq!(token_client.balance(&c), preview.get_unchecked(2));
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
fn deposit_credits_split_balance() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, token_client) = fund_token(&s.env, &payer, 1_000);

    let id = s
        .client
        .create_split(&creator, &vec![&s.env, a], &vec![&s.env, 10_000], &None);

    s.client.deposit(&payer, &id, &token_id, &400);

    assert_eq!(s.client.balance(&id, &token_id), 400);
    assert_eq!(token_client.balance(&s.client.address), 400);
    assert_eq!(token_client.balance(&payer), 600);
}

#[test]
fn distribute_pays_recipients_and_clears_balance() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let b = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, token_client) = fund_token(&s.env, &payer, 1_000);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone(), b.clone()],
        &vec![&s.env, 7_500, 2_500],
        &None,
    );

    s.client.deposit(&payer, &id, &token_id, &600);
    s.client.deposit(&payer, &id, &token_id, &400);
    let distributed = s.client.distribute(&id, &token_id);

    assert_eq!(distributed, 1_000);
    assert_eq!(token_client.balance(&a), 750);
    assert_eq!(token_client.balance(&b), 250);
    assert_eq!(token_client.balance(&s.client.address), 0);
    assert_eq!(s.client.balance(&id, &token_id), 0);
}

#[test]
fn balances_per_token_stay_independent() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_x, _) = fund_token(&s.env, &payer, 1_000);
    let (token_y, client_y) = fund_token(&s.env, &payer, 1_000);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &None,
    );

    s.client.deposit(&payer, &id, &token_x, &300);
    s.client.deposit(&payer, &id, &token_y, &700);

    assert_eq!(s.client.balance(&id, &token_x), 300);
    assert_eq!(s.client.balance(&id, &token_y), 700);

    s.client.distribute(&id, &token_y);
    assert_eq!(s.client.balance(&id, &token_x), 300);
    assert_eq!(s.client.balance(&id, &token_y), 0);
    assert_eq!(client_y.balance(&a), 700);
}

#[test]
fn distribute_with_empty_balance_fails() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let a = Address::generate(&s.env);
    let payer = Address::generate(&s.env);
    let (token_id, _) = fund_token(&s.env, &payer, 1_000);

    let id = s
        .client
        .create_split(&creator, &vec![&s.env, a], &vec![&s.env, 10_000], &None);

    let result = s.client.try_distribute(&id, &token_id);
    assert_eq!(result, Err(Ok(Error::NothingToDistribute)));
}

#[test]
fn deposit_to_unknown_split_fails() {
    let s = setup();
    let payer = Address::generate(&s.env);
    let (token_id, _) = fund_token(&s.env, &payer, 1_000);

    let result = s.client.try_deposit(&payer, &42, &token_id, &100);
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
fn control_can_be_transferred_and_renounced() {
    let s = setup();
    let creator = Address::generate(&s.env);
    let controller = Address::generate(&s.env);
    let next = Address::generate(&s.env);
    let a = Address::generate(&s.env);

    let id = s.client.create_split(
        &creator,
        &vec![&s.env, a.clone()],
        &vec![&s.env, 10_000],
        &Some(controller.clone()),
    );

    s.client.transfer_control(&id, &Some(next.clone()));
    assert_eq!(s.client.get_split(&id).controller, Some(next));

    s.client.transfer_control(&id, &None);
    assert_eq!(s.client.get_split(&id).controller, None);

    let update = s
        .client
        .try_update_split(&id, &vec![&s.env, a], &vec![&s.env, 10_000]);
    assert_eq!(update, Err(Ok(Error::SplitImmutable)));

    let transfer = s.client.try_transfer_control(&id, &None);
    assert_eq!(transfer, Err(Ok(Error::SplitImmutable)));
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
