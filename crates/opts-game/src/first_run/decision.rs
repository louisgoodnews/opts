//! First-run decisions as pure functions: what to do with the installation found at start-up, and
//! how to judge the folder a user picked.

use std::path::PathBuf;

use crate::omsi_root::root_from_selection;

/// What the start-up check knows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupFacts {
    /// `--omsi-path`, or the root above `--map-path`.
    pub explicit: Option<PathBuf>,
    /// `$OMSI_PATH`.
    pub env: Option<PathBuf>,
    /// The folder the user confirmed earlier (settings), if it is still an installation.
    pub saved: Option<PathBuf>,
    /// An installation in a default Steam location that was never confirmed.
    pub candidate: Option<PathBuf>,
    /// Whether a window is there to ask in (not `--headless`, not `--autostart`).
    pub interactive: bool,
}

/// What start-up does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupDecision {
    /// Use this folder. `remember` says whether it is saved to the settings.
    Adopt {
        /// The installation root.
        root: PathBuf,
        /// Save it (a folder the user did not give explicitly and that is not saved yet).
        remember: bool,
    },
    /// Ask the user to confirm this folder (found in a default location).
    Confirm(PathBuf),
    /// Ask the user to choose a folder.
    Choose,
    /// Nothing found and nobody to ask (headless): the catalogue stays empty.
    Missing,
}

/// The decision for `facts`: an explicit choice (command line, environment, saved) is used as it
/// is; a folder found by searching is **offered for confirmation**, never adopted silently
/// (`docs/src/roadmap/phase-9/flow.md` section 3.1). Without a window the search result is used,
/// as before, because nobody could be asked.
pub fn decide(facts: &StartupFacts) -> StartupDecision {
    if let Some(root) = facts.explicit.clone().or_else(|| facts.env.clone()) {
        return StartupDecision::Adopt {
            root,
            remember: false,
        };
    }
    if let Some(root) = facts.saved.clone() {
        return StartupDecision::Adopt {
            root,
            remember: false,
        };
    }
    match (&facts.candidate, facts.interactive) {
        (Some(root), true) => StartupDecision::Confirm(root.clone()),
        (Some(root), false) => StartupDecision::Adopt {
            root: root.clone(),
            remember: true,
        },
        (None, true) => StartupDecision::Choose,
        (None, false) => StartupDecision::Missing,
    }
}

/// Why a picked folder was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChooseError {
    /// The dialog was closed without a folder.
    Cancelled,
    /// The folder is not an installation: it needs `maps` and `Vehicles`.
    NotInstallation(PathBuf),
}

/// Judges the dialog's answer: the installation root (a picked `maps/Grundorf` resolves to the
/// installation above it) or the reason it is not one.
pub fn check_choice(answer: Option<PathBuf>) -> Result<PathBuf, ChooseError> {
    let picked = answer.ok_or(ChooseError::Cancelled)?;
    root_from_selection(&picked).ok_or(ChooseError::NotInstallation(picked))
}

/// Whether the welcome screen is due: the content is read, there is no history and it was never
/// shown. An upgrade with history never sees it.
pub fn welcome_due(welcome_done: bool, has_history: bool, content_ready: bool) -> bool {
    !welcome_done && !has_history && content_ready
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn explicit_choices_are_used_without_asking_and_not_saved() {
        let facts = StartupFacts {
            explicit: Some(p("/a")),
            candidate: Some(p("/steam")),
            interactive: true,
            ..Default::default()
        };
        assert_eq!(
            decide(&facts),
            StartupDecision::Adopt {
                root: p("/a"),
                remember: false
            }
        );
        let env = StartupFacts {
            env: Some(p("/e")),
            saved: Some(p("/s")),
            interactive: true,
            ..Default::default()
        };
        assert!(matches!(decide(&env), StartupDecision::Adopt { root, .. } if root == p("/e")));
    }

    #[test]
    fn a_confirmed_saved_folder_is_used_without_asking_again() {
        let facts = StartupFacts {
            saved: Some(p("/s")),
            candidate: Some(p("/steam")),
            interactive: true,
            ..Default::default()
        };
        assert!(
            matches!(decide(&facts), StartupDecision::Adopt { root, remember: false } if root == p("/s"))
        );
    }

    #[test]
    fn a_found_folder_is_offered_not_adopted_when_a_window_can_ask() {
        let facts = StartupFacts {
            candidate: Some(p("/steam")),
            interactive: true,
            ..Default::default()
        };
        assert_eq!(decide(&facts), StartupDecision::Confirm(p("/steam")));
    }

    #[test]
    fn without_a_window_the_search_result_is_used_and_remembered() {
        let facts = StartupFacts {
            candidate: Some(p("/steam")),
            ..Default::default()
        };
        assert_eq!(
            decide(&facts),
            StartupDecision::Adopt {
                root: p("/steam"),
                remember: true
            }
        );
        assert_eq!(decide(&StartupFacts::default()), StartupDecision::Missing);
    }

    #[test]
    fn nothing_found_asks_the_user_to_choose() {
        let facts = StartupFacts {
            interactive: true,
            ..Default::default()
        };
        assert_eq!(decide(&facts), StartupDecision::Choose);
    }

    #[test]
    fn answers_are_judged_with_the_reason() {
        assert_eq!(check_choice(None), Err(ChooseError::Cancelled));
        let tmp = std::env::temp_dir();
        assert_eq!(
            check_choice(Some(tmp.clone())),
            Err(ChooseError::NotInstallation(tmp))
        );
        let root = std::env::temp_dir().join(format!("opts-firstrun-{}", std::process::id()));
        std::fs::create_dir_all(root.join("maps/M")).expect("mkdir");
        std::fs::create_dir_all(root.join("Vehicles")).expect("mkdir");
        assert_eq!(check_choice(Some(root.join("maps/M"))), Ok(root.clone()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_welcome_shows_once_for_a_fresh_start_with_content() {
        assert!(welcome_due(false, false, true));
        assert!(!welcome_due(true, false, true), "already shown");
        assert!(!welcome_due(false, true, true), "an upgrade with history");
        assert!(
            !welcome_due(false, false, false),
            "content still being read"
        );
    }
}
