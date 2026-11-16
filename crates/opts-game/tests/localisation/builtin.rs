//! The core library lists match the tables the game loads.

use super::*;

/// `opts-tools l10n` and the pack checks read the shipped tables through
/// `opts_l10n::builtin`; the game loads them through its own `include_str!` calls. A new
/// `.ftl` file that is missing from the core list would make the tool disagree with the game.
#[test]
fn builtin_lists_match_the_loaded_tables() {
    let mut loc = whole_application();
    opts_game::hud::add_messages(&mut loc);
    for (lang, table) in [
        ("en", opts_l10n::builtin::table_of("en").expect("en")),
        ("de", opts_l10n::builtin::table_of("de").expect("de")),
    ] {
        let mut listed: Vec<&str> = table.messages.keys().map(String::as_str).collect();
        listed.sort_unstable();
        assert_eq!(
            listed,
            loc.keys_of(lang),
            "{lang}: add the new table to opts-core/src/l10n/builtin.rs"
        );
    }
}
