#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(body: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let saved = save_env();
        clear_env();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        restore_env(saved);
        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    fn save_env() -> [(String, Option<String>); 4] {
        [
            ("JEKKO_ASCII".into(), std::env::var("JEKKO_ASCII").ok()),
            ("LC_ALL".into(), std::env::var("LC_ALL").ok()),
            ("LC_CTYPE".into(), std::env::var("LC_CTYPE").ok()),
            ("LANG".into(), std::env::var("LANG").ok()),
        ]
    }

    fn clear_env() {
        for name in ["JEKKO_ASCII", "LC_ALL", "LC_CTYPE", "LANG"] {
            std::env::remove_var(name);
        }
    }

    fn restore_env(saved: [(String, Option<String>); 4]) {
        for (name, value) in saved {
            match value {
                Some(v) => std::env::set_var(&name, v),
                None => std::env::remove_var(&name),
            }
        }
    }

    #[test]
    fn default_is_unicode() {
        with_clean_env(|| {
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn jekko_ascii_env_sets_ascii() {
        with_clean_env(|| {
            std::env::set_var("JEKKO_ASCII", "1");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn lc_all_c_sets_ascii() {
        with_clean_env(|| {
            std::env::set_var("LC_ALL", "C");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn lang_c_sets_ascii_only_when_lc_all_unset() {
        with_clean_env(|| {
            std::env::set_var("LANG", "C");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn lc_all_utf8_overrides_lang_c() {
        with_clean_env(|| {
            std::env::set_var("LC_ALL", "en_US.UTF-8");
            std::env::set_var("LANG", "C");
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn empty_jekko_ascii_does_not_force_ascii() {
        with_clean_env(|| {
            std::env::set_var("JEKKO_ASCII", "");
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn jekko_ascii_zero_does_not_force_ascii() {
        with_clean_env(|| {
            std::env::set_var("JEKKO_ASCII", "0");
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn jekko_ascii_false_does_not_force_ascii() {
        with_clean_env(|| {
            std::env::set_var("JEKKO_ASCII", "false");
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn lc_ctype_c_sets_ascii_when_lc_all_unset() {
        with_clean_env(|| {
            std::env::set_var("LC_CTYPE", "C");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn lc_ctype_posix_sets_ascii() {
        with_clean_env(|| {
            std::env::set_var("LC_CTYPE", "POSIX");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn lc_all_utf8_keeps_unicode() {
        with_clean_env(|| {
            std::env::set_var("LC_ALL", "en_US.UTF-8");
            assert_eq!(compute_mode(), GlyphMode::Unicode);
        });
    }

    #[test]
    fn case_insensitive_posix_match() {
        with_clean_env(|| {
            std::env::set_var("LANG", "posix");
            assert_eq!(compute_mode(), GlyphMode::Ascii);
        });
    }

    #[test]
    fn current_matches_mode() {
        let table = current();
        let unicode_match = table.bullet_success == UNICODE.bullet_success
            && table.composer_prefix == UNICODE.composer_prefix
            && table.agent_done == UNICODE.agent_done;
        let ascii_match = table.bullet_success == ASCII.bullet_success
            && table.composer_prefix == ASCII.composer_prefix
            && table.agent_done == ASCII.agent_done;
        assert!(
            unicode_match || ascii_match,
            "current() returned a table that matches neither UNICODE nor ASCII"
        );
    }

    #[test]
    fn glyph_constants_match_unicode_vs_ascii() {
        assert_eq!(UNICODE.bullet_success, "●");
        assert_eq!(UNICODE.bullet_pending, "○");
        assert_eq!(UNICODE.bullet_cancelled, "◌");
        assert_eq!(UNICODE.bullet_failed, "×");
        assert_eq!(UNICODE.tree_branch, "└");
        assert_eq!(UNICODE.composer_prefix, "›");
        assert_eq!(UNICODE.divider, "─");
        assert_eq!(UNICODE.ellipsis, "…");
        assert_eq!(UNICODE.banner_prefix, "▸▸");
        assert_eq!(UNICODE.working_pulse, "◦");
        assert_eq!(UNICODE.card_margin, "│");
        assert_eq!(UNICODE.boot_marker, "⚡");
        assert_eq!(UNICODE.arrow_up, "↑");
        assert_eq!(UNICODE.arrow_down, "↓");
        assert_eq!(UNICODE.agent_done, "✓");
        assert_eq!(UNICODE.info_marker, "ⓘ");
        assert_eq!(UNICODE.warning_marker, "▲");
        assert_eq!(UNICODE.error_marker, "✕");
        assert_eq!(UNICODE.running_bullet, "◉");
        assert_eq!(UNICODE.welcome_marker, "✻");
        assert_eq!(UNICODE.spinner_placeholder, "⋯");
        assert_eq!(UNICODE.cursor_block, "█");
        assert_eq!(UNICODE.separator_dot, "·");

        assert_eq!(ASCII.bullet_success, "*");
        assert_eq!(ASCII.bullet_pending, "o");
        assert_eq!(ASCII.bullet_cancelled, ".");
        assert_eq!(ASCII.bullet_failed, "x");
        assert_eq!(ASCII.tree_branch, "\\");
        assert_eq!(ASCII.composer_prefix, ">");
        assert_eq!(ASCII.divider, "-");
        assert_eq!(ASCII.ellipsis, "...");
        assert_eq!(ASCII.banner_prefix, ">>");
        assert_eq!(ASCII.working_pulse, "o");
        assert_eq!(ASCII.card_margin, "|");
        assert_eq!(ASCII.boot_marker, "!");
        assert_eq!(ASCII.arrow_up, "^");
        assert_eq!(ASCII.arrow_down, "v");
        assert_eq!(ASCII.agent_done, "[v]");
        assert_eq!(ASCII.info_marker, "(i)");
        assert_eq!(ASCII.warning_marker, "!");
        assert_eq!(ASCII.error_marker, "x");
        assert_eq!(ASCII.running_bullet, "o");
        assert_eq!(ASCII.welcome_marker, "*");
        assert_eq!(ASCII.spinner_placeholder, "...");
        assert_eq!(ASCII.cursor_block, "#");
        assert_eq!(ASCII.separator_dot, "-");
    }

    #[test]
    fn info_marker_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.info_marker, "ⓘ");
        assert_eq!(ASCII.info_marker, "(i)");
    }

    #[test]
    fn warning_marker_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.warning_marker, "▲");
        assert_eq!(ASCII.warning_marker, "!");
    }

    #[test]
    fn error_marker_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.error_marker, "✕");
        assert_eq!(ASCII.error_marker, "x");
    }

    #[test]
    fn running_bullet_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.running_bullet, "◉");
        assert_eq!(ASCII.running_bullet, "o");
    }

    #[test]
    fn welcome_marker_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.welcome_marker, "✻");
        assert_eq!(ASCII.welcome_marker, "*");
    }

    #[test]
    fn spinner_placeholder_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.spinner_placeholder, "⋯");
        assert_eq!(ASCII.spinner_placeholder, "...");
    }

    #[test]
    fn cursor_block_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.cursor_block, "█");
        assert_eq!(ASCII.cursor_block, "#");
    }

    #[test]
    fn separator_dot_unicode_and_ascii_match_spec() {
        assert_eq!(UNICODE.separator_dot, "·");
        assert_eq!(ASCII.separator_dot, "-");
    }

    #[test]
    fn cancelled_bullet_already_exists_via_bullet_cancelled() {
        assert_eq!(UNICODE.bullet_cancelled, "◌");
        assert_eq!(ASCII.bullet_cancelled, ".");
    }

    #[test]
    fn env_truthy_handles_common_inputs() {
        with_clean_env(|| {
            std::env::set_var("JEKKO_ASCII", "1");
            assert!(env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "true");
            assert!(env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "yes");
            assert!(env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "0");
            assert!(!env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "false");
            assert!(!env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "False");
            assert!(!env_truthy("JEKKO_ASCII"));

            std::env::set_var("JEKKO_ASCII", "");
            assert!(!env_truthy("JEKKO_ASCII"));

            std::env::remove_var("JEKKO_ASCII");
            assert!(!env_truthy("JEKKO_ASCII"));
        });
    }
}
