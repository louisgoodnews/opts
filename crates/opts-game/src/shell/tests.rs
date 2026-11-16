use super::*;

#[test]
fn without_a_shell_the_screen_root_has_no_parent() {
    let mut world = World::new();
    let root = {
        let mut commands = world.commands();
        spawn_screen_root(&mut commands, &ShellContent::default(), "test")
    };
    world.flush();
    assert!(world.get::<ChildOf>(root).is_none());
    assert!(world.get::<ScreenRoot>(root).is_some());
}

#[test]
fn with_a_shell_the_screen_root_lives_in_the_content_area() {
    let mut world = World::new();
    let area = world.spawn(Node::default()).id();
    let root = {
        let mut commands = world.commands();
        spawn_screen_root(&mut commands, &ShellContent(Some(area)), "test")
    };
    world.flush();
    assert_eq!(world.get::<ChildOf>(root).map(ChildOf::parent), Some(area));
}

#[test]
fn every_shell_key_exists_in_english_and_german_and_nothing_else_does() {
    let mut loc = opts_gui::l10n::Localization::new("en");
    extend_localization_into(&mut loc);
    for key in keys::ALL {
        assert!(loc.has("en", key), "shell-en.ftl lacks `{key}`");
        assert!(loc.has("de", key), "shell-de.ftl lacks `{key}`");
    }
    for table in [
        include_str!("l10n/shell-en.ftl"),
        include_str!("l10n/shell-de.ftl"),
    ] {
        for line in table
            .lines()
            .filter(|l| l.contains(" = ") && !l.starts_with(['#', ' ']))
        {
            let key = line.split(" = ").next().unwrap_or_default();
            assert!(
                keys::ALL.contains(&key)
                    || opts_bevy::reporting::keys::ALL
                        .iter()
                        .any(|(k, _)| *k == key),
                "`{key}` is in a table but not in keys::ALL"
            );
        }
    }
}
