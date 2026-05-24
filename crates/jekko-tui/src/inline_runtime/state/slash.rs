// ── Slash commands ───────────────────────────────────────────────────────────
//
// Catalog + action types live in `crate::slash::*` (COWBOY I1/I2). This
// runtime keeps only the popup state machine + visibility gating for the
// compatibility `/panels` marker.

#[derive(Default)]
struct SlashState {
    active: bool,
    query: String,
    cursor: usize,
    submenu: Option<SlashSubmenuState>,
    // WHY: store filtered ids as owned strings rather than catalog indices.
    // Indices would conflate builtins + user-defined entries and break when
    // the user-defined set changes; ids are stable across filters.
    filtered: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SlashSubmenuState {
    parent_id: String,
    cursor: usize,
}

impl SlashState {
    fn refresh_filter(&mut self, catalog: &SlashCatalog) {
        let q = self.query.to_lowercase();
        self.filtered.clear();
        for cmd in catalog.all() {
            if !slash_command_visible(cmd) {
                continue;
            }
            if q.is_empty() || cmd.id().starts_with(&q) {
                self.filtered.push(cmd.id().to_string());
            }
        }
        if self.cursor >= self.filtered.len() {
            self.cursor = self.filtered.len().saturating_sub(1);
        }
    }

    fn current_command<'a>(&self, catalog: &'a SlashCatalog) -> Option<&'a SlashCommand> {
        if self.submenu.is_some() {
            return None;
        }
        self.filtered
            .get(self.cursor)
            .and_then(|id| catalog.find(id))
    }

    fn open_submenu(&mut self, catalog: &SlashCatalog, parent_id: &str) -> bool {
        let Some(submenu) = catalog.submenu_for(parent_id) else {
            return false;
        };
        if submenu.items.is_empty() {
            return false;
        }
        self.query = parent_id.to_string();
        self.refresh_filter(catalog);
        self.submenu = Some(SlashSubmenuState {
            parent_id: parent_id.to_string(),
            cursor: 0,
        });
        true
    }

    fn pop_submenu(&mut self) -> bool {
        self.submenu.take().is_some()
    }

    fn selected_subcommand(
        &self,
        catalog: &SlashCatalog,
    ) -> Option<(&'static str, &'static str, &'static SlashSubcommand)> {
        let state = self.submenu.as_ref()?;
        let submenu = catalog.submenu_for(&state.parent_id)?;
        let item = submenu.item(state.cursor)?;
        Some((submenu.parent_id, submenu.shell_base, item))
    }

    fn selection_len(&self, catalog: &SlashCatalog) -> usize {
        if let Some(state) = &self.submenu {
            return match catalog.submenu_for(&state.parent_id) {
                Some(submenu) => submenu.items.len(),
                None => 0,
            };
        }
        self.filtered.len()
    }

    fn move_prev(&mut self) {
        if let Some(submenu) = self.submenu.as_mut() {
            if submenu.cursor > 0 {
                submenu.cursor -= 1;
            }
        } else if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_next(&mut self, catalog: &SlashCatalog) {
        let max = self.selection_len(catalog).saturating_sub(1);
        if let Some(submenu) = self.submenu.as_mut() {
            if submenu.cursor < max {
                submenu.cursor += 1;
            }
        } else if self.cursor < max {
            self.cursor += 1;
        }
    }
}

fn slash_command_visible(cmd: &SlashCommand) -> bool {
    if cmd.id() != "panels" {
        return true;
    }
    std::env::var(LEGACY_PANELS_ENV)
        .ok()
        .map(|v| matches!(v.trim(), "1" | "true" | "on"))
        .unwrap_or(false)
}

fn slash_command_visible_count(catalog: &SlashCatalog) -> usize {
    catalog
        .all()
        .filter(|cmd| slash_command_visible(cmd))
        .count()
}
