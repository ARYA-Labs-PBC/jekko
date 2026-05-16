use std::collections::BTreeMap;

use super::ActionName;

/// Default keybinds table. Keys are action names (matching the TS
/// `KeybindsSchema` fields); values are the default binding strings before
/// parsing.
///
/// Returns a [`BTreeMap`] so iteration order is deterministic.
pub fn default_bindings() -> BTreeMap<ActionName, &'static str> {
    let mut m = BTreeMap::<ActionName, &'static str>::new();
    m.insert("leader", "ctrl+x");
    m.insert("app_exit", "ctrl+c,ctrl+d,<leader>q");
    m.insert("editor_open", "<leader>e");
    m.insert("theme_list", "<leader>t");
    m.insert("sidebar_toggle", "<leader>b");
    m.insert("scrollbar_toggle", "none");
    m.insert("username_toggle", "none");
    m.insert("status_view", "<leader>s");
    m.insert("session_export", "<leader>x");
    m.insert("session_new", "<leader>n");
    m.insert("session_list", "<leader>l");
    m.insert("session_timeline", "<leader>g");
    m.insert("session_fork", "none");
    m.insert("session_rename", "ctrl+r");
    m.insert("session_delete", "ctrl+d");
    m.insert("stash_delete", "ctrl+d");
    m.insert("model_provider_list", "ctrl+a");
    m.insert("model_favorite_toggle", "ctrl+f");
    m.insert("session_share", "none");
    m.insert("session_unshare", "none");
    m.insert("session_interrupt", "escape");
    m.insert("session_compact", "<leader>c");
    m.insert("messages_page_up", "pageup,ctrl+alt+b");
    m.insert("messages_page_down", "pagedown,ctrl+alt+f");
    m.insert("messages_line_up", "ctrl+alt+y");
    m.insert("messages_line_down", "ctrl+alt+e");
    m.insert("messages_half_page_up", "ctrl+alt+u");
    m.insert("messages_half_page_down", "ctrl+alt+d");
    m.insert("messages_first", "ctrl+g,home");
    m.insert("messages_last", "ctrl+alt+g,end");
    m.insert("messages_next", "none");
    m.insert("messages_previous", "none");
    m.insert("messages_last_user", "none");
    m.insert("messages_copy", "<leader>y");
    m.insert("messages_undo", "<leader>u");
    m.insert("messages_redo", "<leader>r");
    m.insert("messages_toggle_conceal", "<leader>h");
    m.insert("tool_details", "none");
    m.insert("model_list", "<leader>m");
    m.insert("model_cycle_recent", "f2");
    m.insert("model_cycle_recent_reverse", "shift+f2");
    m.insert("model_cycle_favorite", "none");
    m.insert("model_cycle_favorite_reverse", "none");
    m.insert("command_list", "ctrl+p");
    m.insert("agent_list", "<leader>a");
    m.insert("agent_cycle", "tab");
    m.insert("agent_cycle_reverse", "shift+tab");
    m.insert("variant_cycle", "ctrl+t");
    m.insert("variant_list", "none");
    m.insert("input_clear", "ctrl+c");
    m.insert("input_paste", "ctrl+v");
    m.insert("input_submit", "return,enter,linefeed");
    m.insert(
        "input_newline",
        "shift+return,shift+enter,shift+linefeed,ctrl+return,ctrl+enter,ctrl+linefeed,alt+return,alt+enter,alt+linefeed",
    );
    m.insert("input_move_left", "left,ctrl+b");
    m.insert("input_move_right", "right,ctrl+f");
    m.insert("input_move_up", "up");
    m.insert("input_move_down", "down");
    m.insert("input_select_left", "shift+left");
    m.insert("input_select_right", "shift+right");
    m.insert("input_select_up", "shift+up");
    m.insert("input_select_down", "shift+down");
    m.insert("input_line_home", "ctrl+a");
    m.insert("input_line_end", "ctrl+e");
    m.insert("input_select_line_home", "ctrl+shift+a");
    m.insert("input_select_line_end", "ctrl+shift+e");
    m.insert("input_visual_line_home", "alt+a");
    m.insert("input_visual_line_end", "alt+e");
    m.insert("input_select_visual_line_home", "alt+shift+a");
    m.insert("input_select_visual_line_end", "alt+shift+e");
    m.insert("input_buffer_home", "home");
    m.insert("input_buffer_end", "end");
    m.insert("input_select_buffer_home", "shift+home");
    m.insert("input_select_buffer_end", "shift+end");
    m.insert("input_delete_line", "ctrl+shift+d");
    m.insert("input_delete_to_line_end", "ctrl+k");
    m.insert("input_delete_to_line_start", "ctrl+u");
    m.insert("input_backspace", "backspace,shift+backspace");
    m.insert("input_delete", "ctrl+d,delete,shift+delete");
    m.insert("input_undo", "ctrl+-,super+z");
    m.insert("input_redo", "ctrl+.,super+shift+z");
    m.insert("input_word_forward", "alt+f,alt+right,ctrl+right");
    m.insert("input_word_backward", "alt+b,alt+left,ctrl+left");
    m.insert("input_select_word_forward", "alt+shift+f,alt+shift+right");
    m.insert("input_select_word_backward", "alt+shift+b,alt+shift+left");
    m.insert("input_delete_word_forward", "alt+d,alt+delete,ctrl+delete");
    m.insert(
        "input_delete_word_backward",
        "ctrl+w,ctrl+backspace,alt+backspace",
    );
    m.insert("history_previous", "up");
    m.insert("history_next", "down");
    m.insert("session_child_first", "<leader>down");
    m.insert("session_child_cycle", "right");
    m.insert("session_child_cycle_reverse", "left");
    m.insert("session_parent", "up");
    m.insert("terminal_suspend", "ctrl+z");
    m.insert("terminal_title_toggle", "none");
    m.insert("tips_toggle", "<leader>h");
    m.insert("plugin_manager", "none");
    m.insert("display_thinking", "none");
    m.insert("engage", "return");
    m.insert("shell.tab.cycle", "tab");
    m.insert("shell.tab.cycleBack", "shift+tab");
    m.insert("shell.tab.set", "1,2,3");
    m.insert("shell.left.toggle", "ctrl+b");
    m.insert("theme.mode.toggle", "ctrl+shift+t");
    m.insert("feed.scroll.pageUp", "pageup");
    m.insert("feed.scroll.pageDown", "pagedown");
    m.insert("feed.scroll.top", "g");
    m.insert("feed.scroll.bottom", "shift+g");
    m.insert("feed.yank", "y");
    m.insert("feed.reasoning.toggle", "r");
    m.insert("session.new", "ctrl+n");
    m.insert("session.resume", "ctrl+r");
    m.insert("help.show", "?");
    m
}
