use std::str::FromStr;
use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, Style as SynStyle, StyleModifier, Theme,
    ThemeItem, ThemeSettings,
};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::theme;
use jekko_core::theme::ThemeMode;

const MAX_CODE_BLOCK_BYTES: usize = 40 * 1024;

include!("syntax/renderer.rs");
include!("syntax/markdown.rs");
include!("syntax/code.rs");
include!("syntax/theme.rs");

#[cfg(test)]
include!("syntax/tests.rs");
