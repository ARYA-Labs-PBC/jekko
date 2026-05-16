//! Session-id newtype parsing tests.
use std::str::FromStr;

use jekko_core::project::ProjectId;
use jekko_core::session::{
    AccountId, EventId, MessageId, PartId, PermissionId, ServiceId, SessionId, WorkspaceId,
};

#[test]
fn session_id_round_trip() {
    let id: SessionId = "session_01HXXX".parse().unwrap();
    assert_eq!(id.as_str(), "session_01HXXX");
    assert_eq!(id.to_string(), "session_01HXXX");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"session_01HXXX\"");
    let back: SessionId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn empty_session_id_rejected() {
    let err = SessionId::from_str("").unwrap_err();
    assert_eq!(err.kind, SessionId::KIND);
}

#[test]
fn all_id_kinds_round_trip() {
    let pairs: &[(&str, &str)] = &[
        (SessionId::KIND, "session_x"),
        (MessageId::KIND, "msg_x"),
        (PartId::KIND, "part_x"),
        (PermissionId::KIND, "perm_x"),
        (EventId::KIND, "evt_x"),
        (WorkspaceId::KIND, "ws_x"),
        (AccountId::KIND, "acc_x"),
        (ServiceId::KIND, "svc_x"),
    ];

    for (kind, value) in pairs {
        match *kind {
            "session" => {
                let id: SessionId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "message" => {
                let id: MessageId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "part" => {
                let id: PartId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "permission" => {
                let id: PermissionId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "event" => {
                let id: EventId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "workspace" => {
                let id: WorkspaceId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "account" => {
                let id: AccountId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            "service" => {
                let id: ServiceId = value.parse().unwrap();
                assert_eq!(id.as_str(), *value);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn project_id_global_constant() {
    let p = ProjectId::global();
    assert_eq!(p.as_str(), ProjectId::GLOBAL);
    assert_eq!(p.to_string(), "global");
}
