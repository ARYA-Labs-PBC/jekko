//! Round-trip account, account_state, control_account rows.

use jekko_store::account::{self, AccountRow, AccountStateRow, ControlAccountRow};
use jekko_store::db::Db;

fn sample(id: &str) -> AccountRow {
    AccountRow {
        id: id.to_string(),
        email: "user@example.com".to_string(),
        url: "https://api.example.com".to_string(),
        access_token: "x".to_string(),
        refresh_token: "tok-refresh".to_string(),
        token_expiry: Some(2_000_000),
        time_created: 1_000_000,
        time_updated: 1_000_000,
    }
}

#[test]
fn account_create_read() {
    let db = Db::open_in_memory().unwrap();
    let row = sample("acct-1");
    account::upsert(db.connection(), &row).expect("upsert");
    let got = account::get(db.connection(), "acct-1")
        .expect("query")
        .expect("present");
    assert_eq!(got, row);
}

#[test]
fn account_update_via_upsert() {
    let db = Db::open_in_memory().unwrap();
    let mut row = sample("acct-2");
    account::upsert(db.connection(), &row).expect("upsert");

    row.email = "new@example.com".to_string();
    row.access_token = "fresh".to_string();
    row.time_updated = 9_999_999;
    account::upsert(db.connection(), &row).expect("update");

    let got = account::get(db.connection(), "acct-2")
        .expect("query")
        .expect("present");
    assert_eq!(got.email, "new@example.com");
    assert_eq!(got.access_token, "fresh");
    assert_eq!(got.time_updated, 9_999_999);
}

#[test]
fn account_delete() {
    let db = Db::open_in_memory().unwrap();
    let row = sample("acct-3");
    account::upsert(db.connection(), &row).expect("upsert");
    let n = account::delete(db.connection(), "acct-3").expect("delete");
    assert_eq!(n, 1);
    assert!(account::get(db.connection(), "acct-3")
        .expect("query")
        .is_none());
}

#[test]
fn account_state_round_trip() {
    let db = Db::open_in_memory().unwrap();
    let initial = account::read_state(db.connection())
        .expect("read")
        .expect("seeded state");
    assert_eq!(
        initial,
        AccountStateRow {
            id: 1,
            active_account_id: None,
            active_org_id: None,
        }
    );

    // The FK references `account.id`, so we must seed an account first.
    account::upsert(db.connection(), &sample("acct-x")).expect("seed account");

    let state = AccountStateRow {
        id: 1,
        active_account_id: Some("acct-x".to_string()),
        active_org_id: Some("org-y".to_string()),
    };
    account::write_state(db.connection(), &state).expect("write");
    let got = account::read_state(db.connection())
        .expect("read")
        .expect("present");
    assert_eq!(got, state);

    let updated = AccountStateRow {
        id: 1,
        active_account_id: None,
        active_org_id: None,
    };
    account::write_state(db.connection(), &updated).expect("clear");
    let got = account::read_state(db.connection())
        .expect("read")
        .expect("present");
    assert_eq!(got, updated);
}

#[test]
fn control_account_round_trip() {
    let db = Db::open_in_memory().unwrap();
    let row = ControlAccountRow {
        email: "ctl@example.com".to_string(),
        url: "https://ctl.example.com".to_string(),
        access_token: "a".to_string(),
        refresh_token: "r".to_string(),
        token_expiry: None,
        active: true,
        time_created: 1,
        time_updated: 1,
    };
    account::upsert_control(db.connection(), &row).expect("upsert");
    let got = account::get_control(db.connection(), &row.email, &row.url)
        .expect("query")
        .expect("present");
    assert_eq!(got, row);
}
