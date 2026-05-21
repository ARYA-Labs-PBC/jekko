#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn motion_sources_default_to_enabled() {
        assert!(motion_enabled_from_sources(None, None, None));
    }

    #[test]
    fn motion_zero_disables_motion() {
        assert!(!motion_enabled_from_sources(Some("0"), None, None));
    }

    #[test]
    fn jekko_reduced_motion_one_disables_motion() {
        assert!(!motion_enabled_from_sources(None, Some("1"), None));
    }

    #[test]
    fn config_reduced_motion_true_disables_motion() {
        assert!(!motion_enabled_from_sources(None, None, Some(true)));
    }

    #[test]
    fn config_false_does_not_override_env_reduced_motion() {
        assert!(!motion_enabled_from_sources(None, Some("1"), Some(false)));
    }

    #[test]
    fn non_disabling_env_values_keep_motion_enabled() {
        assert!(motion_enabled_from_sources(Some("1"), Some("0"), None));
    }

    #[test]
    fn motion_enabled_with_cfg_none_falls_through_to_legacy() {
        let _ = motion_enabled_with_cfg(None);
    }

    #[test]
    fn motion_enabled_with_cfg_reduced_motion_true_disables() {
        let mut cfg = UiConfig::defaults();
        cfg.accessibility.reduced_motion = Some(true);
        assert!(!motion_enabled_with_cfg(Some(&cfg)));
    }

    #[test]
    fn motion_enabled_with_cfg_reduced_motion_false_enables() {
        let mut cfg = UiConfig::defaults();
        cfg.accessibility.reduced_motion = Some(false);
        let prev_motion = std::env::var("MOTION").ok();
        let prev_jekko = std::env::var("JEKKO_REDUCED_MOTION").ok();
        std::env::remove_var("MOTION");
        std::env::remove_var("JEKKO_REDUCED_MOTION");
        let result = motion_enabled_with_cfg(Some(&cfg));
        if let Some(v) = prev_motion {
            std::env::set_var("MOTION", v);
        }
        if let Some(v) = prev_jekko {
            std::env::set_var("JEKKO_REDUCED_MOTION", v);
        }
        assert!(result);
    }

    #[test]
    fn parses_ui_animation_reduced_motion_true() {
        let text = r#"
            [ui]
            other = true

            [ui.animation]
            reduced_motion = true
        "#;
        assert_eq!(parse_ui_toml_reduced_motion(text), Some(true));
    }

    #[test]
    fn parses_ui_animation_reduced_motion_false() {
        let text = r#"
            [ui.animation]
            reduced_motion = false # inline comments are ignored
        "#;
        assert_eq!(parse_ui_toml_reduced_motion(text), Some(false));
    }

    #[test]
    fn ignores_reduced_motion_outside_animation_section() {
        let text = r#"
            reduced_motion = true

            [ui]
            reduced_motion = true
        "#;
        assert_eq!(parse_ui_toml_reduced_motion(text), None);
    }

    #[test]
    fn pulse_glyph_cycles_through_frames() {
        let a = pulse_glyph(Duration::from_millis(0));
        let b = pulse_glyph(Duration::from_millis(170));
        let c = pulse_glyph(Duration::from_millis(330));
        if motion_enabled() {
            assert!(!(a == b && b == c), "expected glyph rotation");
        } else {
            assert_eq!(a, b);
            assert_eq!(b, c);
        }
    }

    #[test]
    fn spinner_glyph_uses_braille() {
        let g = spinner_glyph(Duration::from_millis(0));
        assert!(g
            .chars()
            .any(|c| (0x2800u32..=0x28FFu32).contains(&(c as u32))));
    }

    #[test]
    fn oscillate_color_returns_a_color() {
        let from = Color::Rgb(0, 0, 0);
        let to = Color::Rgb(255, 255, 255);
        let mid = oscillate_color(Duration::from_millis(250), 1.0, from, to);
        match mid {
            Color::Rgb(_, _, _) => {}
            other => panic!("expected Rgb, got {other:?}"),
        }
    }

    #[test]
    fn lerp_color_endpoints() {
        let a = Color::Rgb(10, 20, 30);
        let b = Color::Rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
    }

    #[test]
    fn lerp_color_midpoint() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        match lerp_color(a, b, 0.5) {
            Color::Rgb(r, g, bl) => {
                assert!((99..=101).contains(&r), "r={r}");
                assert!((49..=51).contains(&g), "g={g}");
                assert!((24..=26).contains(&bl), "b={bl}");
            }
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn lerp_color_non_rgb_returns_from() {
        let a = Color::Yellow;
        let b = Color::Rgb(255, 0, 0);
        assert_eq!(lerp_color(a, b, 0.5), Color::Yellow);
    }

    #[test]
    fn elapsed_label_formats() {
        assert_eq!(elapsed_label(Duration::from_secs(3)), "3s");
        assert_eq!(elapsed_label(Duration::from_secs(59)), "59s");
        assert_eq!(elapsed_label(Duration::from_secs(60)), "1m 0s");
        assert_eq!(elapsed_label(Duration::from_secs(125)), "2m 5s");
        assert_eq!(elapsed_label(Duration::from_secs(3661)), "1h 1m");
        assert_eq!(elapsed_label(Duration::from_secs(90061)), "1d 1h");
    }

    #[test]
    fn rotating_verb_returns_empty_for_empty_slice() {
        assert_eq!(
            rotating_verb(Duration::from_secs(0), &[], Duration::from_secs(4)),
            ""
        );
        assert_eq!(
            rotating_verb(Duration::from_secs(120), &[], Duration::from_secs(4)),
            ""
        );
    }

    #[test]
    fn rotating_verb_at_zero_returns_first() {
        let verbs: &[&'static str] = &["alpha", "beta", "gamma"];
        assert_eq!(
            rotating_verb(Duration::from_secs(0), verbs, Duration::from_secs(4)),
            "alpha"
        );
        assert_eq!(
            rotating_verb(Duration::from_secs(3), verbs, Duration::from_secs(4)),
            "alpha"
        );
    }

    #[test]
    fn rotating_verb_after_period_advances_to_second() {
        let verbs: &[&'static str] = &["alpha", "beta", "gamma"];
        assert_eq!(
            rotating_verb(Duration::from_secs(4), verbs, Duration::from_secs(4)),
            "beta"
        );
        assert_eq!(
            rotating_verb(Duration::from_secs(7), verbs, Duration::from_secs(4)),
            "beta"
        );
        assert_eq!(
            rotating_verb(Duration::from_secs(8), verbs, Duration::from_secs(4)),
            "gamma"
        );
    }

    #[test]
    fn rotating_verb_wraps_around() {
        let verbs: &[&'static str] = &["alpha", "beta", "gamma"];
        assert_eq!(
            rotating_verb(Duration::from_secs(12), verbs, Duration::from_secs(4)),
            "alpha"
        );
        assert_eq!(
            rotating_verb(Duration::from_secs(16), verbs, Duration::from_secs(4)),
            "beta"
        );
    }

    #[test]
    fn rotating_verb_with_thinking_verbs_cycles_seven() {
        let period = Duration::from_secs(4);
        for (i, expected) in THINKING_VERBS.iter().enumerate() {
            let elapsed = Duration::from_secs((i as u64) * period.as_secs());
            assert_eq!(
                rotating_verb(elapsed, THINKING_VERBS, period),
                *expected,
                "bucket {i} should map to {:?}",
                expected
            );
        }
        let one_cycle = Duration::from_secs((THINKING_VERBS.len() as u64) * period.as_secs());
        assert_eq!(
            rotating_verb(one_cycle, THINKING_VERBS, period),
            THINKING_VERBS[0]
        );
    }

    #[test]
    fn thinking_verbs_has_seven_entries() {
        assert_eq!(THINKING_VERBS.len(), 7);
        assert_eq!(THINKING_VERBS[0], "Metamorphosing");
        assert_eq!(THINKING_VERBS[6], "Untangling");
    }

    #[test]
    #[should_panic(expected = "non-zero period")]
    fn rotating_verb_panics_on_zero_period() {
        let verbs: &[&'static str] = &["alpha"];
        let _ = rotating_verb(Duration::from_secs(1), verbs, Duration::ZERO);
    }
}
