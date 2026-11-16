use super::*;

#[test]
fn the_builtin_rail_has_four_main_and_two_utility_entries_in_order() {
    let mut registry = ScreenRegistry::default();
    for s in builtin_screens() {
        registry.register(s);
    }
    let ids = |g| registry.group(g).iter().map(|s| s.id).collect::<Vec<_>>();
    assert_eq!(
        ids(ScreenGroup::Main),
        ["home", "drive", "library", "workshop"]
    );
    assert_eq!(ids(ScreenGroup::Utility), ["settings", "help"]);
    assert!(registry.group(ScreenGroup::Main).len() <= 5, "UI-P02");
}

#[test]
fn a_registration_with_an_existing_id_replaces_it() {
    let mut registry = ScreenRegistry::default();
    let mut home = builtin_screens()[0];
    registry.register(home);
    home.order = 99;
    registry.register(home);
    assert_eq!(registry.all().len(), 1);
    assert_eq!(registry.get("home").map(|s| s.order), Some(99));
    assert_eq!(
        registry.by_state(AppState::MainMenu).map(|s| s.id),
        Some("home")
    );
}
