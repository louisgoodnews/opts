//! Mode parsing and the glob patterns of the filters.

use super::*;

#[test]
fn mode_parsing() {
    assert_eq!(Mode::parse("Full"), Some(Mode::Full));
    assert_eq!(Mode::parse("summary"), Some(Mode::Summary));
    assert_eq!(Mode::parse(""), Some(Mode::Off));
    assert_eq!(Mode::parse("loud"), None);
}

#[test]
fn glob_patterns() {
    use filter::glob_match;
    assert!(glob_match("house", "Sceneryobjects/House_01.sco"));
    assert!(glob_match("*.sco", "a/b.SCO"));
    assert!(glob_match(
        "sceneryobjects/*/house*.sco",
        "Sceneryobjects/x/House_01.sco"
    ));
    assert!(!glob_match("*.o3d", "a.sco"));
}
