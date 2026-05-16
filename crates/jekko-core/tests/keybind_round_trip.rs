//! Keybind round-trip tests: parsing and rendering a variety of chord strings.
use jekko_core::keybind::{Chord, ChordSet, KeybindsTable};

#[test]
fn parses_ten_representative_chords() {
    let cases = [
        "ctrl+p",
        "shift+enter",
        "alt+up",
        "ctrl+shift+t",
        "<leader>q",
        "<leader>e",
        "escape",
        "tab",
        "shift+tab",
        "super+z",
    ];
    for case in cases {
        let chord = Chord::parse(case).unwrap_or_else(|err| panic!("parse {case}: {err}"));
        // The canonical render must itself re-parse to an equivalent chord.
        let canonical = chord.to_string_canonical();
        let again =
            Chord::parse(&canonical).unwrap_or_else(|err| panic!("reparse {canonical}: {err}"));
        assert_eq!(chord, again, "round-trip failed for {case}");
    }
}

#[test]
fn chord_set_round_trips_comma_separated() {
    let set = ChordSet::parse("pageup,ctrl+alt+b").unwrap();
    let s = set.to_string_canonical();
    let reparsed = ChordSet::parse(&s).unwrap();
    assert_eq!(set, reparsed);
}

#[test]
fn chord_set_none_round_trips() {
    let set = ChordSet::parse("none").unwrap();
    assert!(set.is_empty());
    assert_eq!(set.to_string_canonical(), "none");
}

#[test]
fn defaults_table_resolves_known_actions() {
    let table = KeybindsTable::defaults().expect("defaults parse");

    // ctrl+p -> command_list
    let cmd = table.get("command_list").expect("command_list");
    assert_eq!(cmd.len(), 1);
    assert!(cmd.0[0].ctrl);
    assert_eq!(cmd.0[0].name, "p");

    // shift+tab -> agent_cycle_reverse
    let cycle = table
        .get("agent_cycle_reverse")
        .expect("agent_cycle_reverse");
    assert_eq!(cycle.len(), 1);
    assert!(cycle.0[0].shift);
    assert_eq!(cycle.0[0].name, "tab");

    // input_submit is a comma-separated triple.
    let submit = table.get("input_submit").expect("input_submit");
    assert_eq!(submit.len(), 3);
}

#[test]
fn esc_alias_normalises_to_escape() {
    let chord = Chord::parse("esc").unwrap();
    assert_eq!(chord.name, "escape");
}

#[test]
fn alt_meta_option_are_all_meta() {
    for alias in ["alt+f", "meta+f", "option+f"] {
        let chord = Chord::parse(alias).unwrap();
        assert!(chord.meta, "alias {alias} should set meta");
        assert_eq!(chord.name, "f");
    }
}
