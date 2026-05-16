//! Round-trip an event_sequence + event row.

use jekko_store::db::Db;
use jekko_store::sync::{self, EventRow, EventSequenceRow};

#[test]
fn sync_event_create_read() {
    let db = Db::open_in_memory().unwrap();

    let seq = EventSequenceRow {
        aggregate_id: "agg-1".to_string(),
        seq: 5,
        owner_id: Some("owner-a".to_string()),
    };
    sync::upsert_sequence(db.connection(), &seq).expect("upsert seq");
    let got_seq = sync::get_sequence(db.connection(), "agg-1")
        .expect("get seq")
        .expect("present");
    assert_eq!(got_seq, seq);

    let event = EventRow {
        id: "evt-1".to_string(),
        aggregate_id: "agg-1".to_string(),
        seq: 1,
        event_type: "test.created".to_string(),
        data: serde_json::json!({"foo":"bar"}),
    };
    sync::insert_event(db.connection(), &event).expect("insert event");
    let got = sync::get_event(db.connection(), "evt-1")
        .expect("get event")
        .expect("present");
    assert_eq!(got, event);
}

#[test]
fn sync_event_update_via_sequence_upsert() {
    let db = Db::open_in_memory().unwrap();
    sync::upsert_sequence(
        db.connection(),
        &EventSequenceRow {
            aggregate_id: "agg-x".to_string(),
            seq: 1,
            owner_id: None,
        },
    )
    .unwrap();
    sync::upsert_sequence(
        db.connection(),
        &EventSequenceRow {
            aggregate_id: "agg-x".to_string(),
            seq: 42,
            owner_id: Some("o".to_string()),
        },
    )
    .unwrap();
    let got = sync::get_sequence(db.connection(), "agg-x")
        .unwrap()
        .unwrap();
    assert_eq!(got.seq, 42);
    assert_eq!(got.owner_id, Some("o".to_string()));
}

#[test]
fn sync_event_list_in_seq_order() {
    let db = Db::open_in_memory().unwrap();
    sync::upsert_sequence(
        db.connection(),
        &EventSequenceRow {
            aggregate_id: "agg-z".to_string(),
            seq: 0,
            owner_id: None,
        },
    )
    .unwrap();
    for (id, seq) in [("e3", 3), ("e1", 1), ("e2", 2)] {
        sync::insert_event(
            db.connection(),
            &EventRow {
                id: id.to_string(),
                aggregate_id: "agg-z".to_string(),
                seq,
                event_type: "test".to_string(),
                data: serde_json::json!({}),
            },
        )
        .expect("insert");
    }
    let rows = sync::list_events(db.connection(), "agg-z", None).expect("list");
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["e1", "e2", "e3"]
    );
}

#[test]
fn sync_event_list_after_cursor() {
    let db = Db::open_in_memory().unwrap();
    sync::upsert_sequence(
        db.connection(),
        &EventSequenceRow {
            aggregate_id: "agg-y".to_string(),
            seq: 0,
            owner_id: None,
        },
    )
    .unwrap();
    for (id, seq) in [("a", 1), ("b", 2), ("c", 3)] {
        sync::insert_event(
            db.connection(),
            &EventRow {
                id: id.to_string(),
                aggregate_id: "agg-y".to_string(),
                seq,
                event_type: "t".to_string(),
                data: serde_json::json!({}),
            },
        )
        .unwrap();
    }
    let rows = sync::list_events(db.connection(), "agg-y", Some(1)).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
}

#[test]
fn sync_event_delete() {
    let db = Db::open_in_memory().unwrap();
    sync::upsert_sequence(
        db.connection(),
        &EventSequenceRow {
            aggregate_id: "agg-d".to_string(),
            seq: 0,
            owner_id: None,
        },
    )
    .unwrap();
    let event = EventRow {
        id: "evt-d".to_string(),
        aggregate_id: "agg-d".to_string(),
        seq: 1,
        event_type: "t".to_string(),
        data: serde_json::json!({"k":1}),
    };
    sync::insert_event(db.connection(), &event).unwrap();
    let removed = sync::delete_event(db.connection(), "evt-d").expect("delete");
    assert_eq!(removed, 1);
    assert!(sync::get_event(db.connection(), "evt-d").unwrap().is_none());
}
