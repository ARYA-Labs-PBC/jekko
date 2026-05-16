use super::*;

#[test]
fn parse_simple_chord() {
    let chord: Chord = "ctrl+p".parse().unwrap();
    assert!(chord.ctrl);
    assert_eq!(chord.name, "p");
}

#[test]
fn parse_leader_chord() {
    let chord: Chord = "<leader>q".parse().unwrap();
    assert!(chord.leader);
    assert_eq!(chord.name, "q");
}

#[test]
fn parse_alt_alias() {
    let chord: Chord = "option+f".parse().unwrap();
    assert!(chord.meta);
    assert_eq!(chord.name, "f");
}

#[test]
fn parse_esc_alias() {
    let chord: Chord = "esc".parse().unwrap();
    assert_eq!(chord.name, "escape");
}

#[test]
fn chord_set_none() {
    let set: ChordSet = "none".parse().unwrap();
    assert!(set.is_empty());
}

#[test]
fn chord_set_canonical() {
    let set: ChordSet = "pageup,ctrl+alt+b".parse().unwrap();
    assert_eq!(set.len(), 2);
    assert_eq!(set.to_string_canonical(), "pageup,ctrl+alt+b");
}

#[test]
fn default_table_resolves() {
    let table = KeybindsTable::defaults().expect("defaults parse");
    let new_session = table.get("session.new").unwrap();
    assert_eq!(new_session.len(), 1);
    assert!(new_session.0[0].ctrl);
    assert_eq!(new_session.0[0].name, "n");
}
