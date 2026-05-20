fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntax_theme(mode: ThemeMode) -> &'static Theme {
    static THEMES: OnceLock<(Theme, Theme)> = OnceLock::new();
    let (dark, light) =
        THEMES.get_or_init(|| (build_theme(ThemeMode::Dark), build_theme(ThemeMode::Light)));
    match mode {
        ThemeMode::Dark => dark,
        ThemeMode::Light => light,
    }
}

fn build_theme(mode: ThemeMode) -> Theme {
    let pal = theme::palette(mode);
    let mut theme = Theme::default();
    theme.name = Some(match mode {
        ThemeMode::Dark => "jekko-dark".to_string(),
        ThemeMode::Light => "jekko-light".to_string(),
    });
    theme.author = Some("Jekko".to_string());
    let mut settings = ThemeSettings::default();
    settings.foreground = Some(syn_color(pal.text));
    settings.background = None;
    theme.settings = settings;
    theme.scopes = vec![
        theme_item(
            "comment, punctuation.definition.comment",
            pal.text_muted,
            Some(FontStyle::ITALIC),
        ),
        theme_item(
            "keyword, storage, storage.modifier, storage.type",
            Color::Rgb(0xd4, 0x72, 0xb6),
            Some(FontStyle::BOLD),
        ),
        theme_item(
            "entity.name.function, support.function, meta.function-call",
            Color::Rgb(0x55, 0xd6, 0xff),
            None,
        ),
        theme_item(
            "string, constant.other.symbol",
            Color::Rgb(0x8a, 0xc8, 0x6a),
            None,
        ),
        theme_item("constant.numeric", Color::Rgb(0xf5, 0xa6, 0x23), None),
        theme_item(
            "entity.name.type, support.type, entity.name.class",
            Color::Rgb(0x55, 0xd6, 0xff),
            None,
        ),
        theme_item(
            "keyword.operator, punctuation, meta.brace, meta.delimiter",
            pal.text_muted,
            None,
        ),
        theme_item(
            "constant.language, variable.language",
            Color::Rgb(0x6a, 0xd6, 0xff),
            None,
        ),
        theme_item(
            "invalid, invalid.illegal",
            Color::Rgb(0xe0, 0x6c, 0x75),
            Some(FontStyle::BOLD),
        ),
    ];
    theme
}

fn theme_item(scope: &str, fg: Color, font_style: Option<FontStyle>) -> ThemeItem {
    ThemeItem {
        scope: match ScopeSelectors::from_str(scope) {
            Ok(selectors) => selectors,
            Err(_) => ScopeSelectors::default(),
        },
        style: StyleModifier {
            foreground: Some(syn_color(fg)),
            background: None,
            font_style: Some(match font_style {
                Some(style) => style,
                None => FontStyle::empty(),
            }),
        },
    }
}

fn syn_color(color: Color) -> SynColor {
    match color {
        Color::Rgb(r, g, b) => SynColor { r, g, b, a: 0xff },
        Color::Reset => SynColor::BLACK,
        Color::Indexed(idx) => SynColor {
            r: idx,
            g: idx,
            b: idx,
            a: 0xff,
        },
        _ => SynColor::WHITE,
    }
}

fn syntax_for_lang<'a>(set: &'a SyntaxSet, lang: &str) -> &'a SyntaxReference {
    set.find_syntax_by_token(lang)
        .or_else(|| set.find_syntax_by_extension(lang))
        .unwrap_or_else(|| set.find_syntax_plain_text())
}

fn is_blank_line(line: &Line<'_>) -> bool {
    line.spans.is_empty() || line.spans.iter().all(|span| span.content.is_empty())
}
