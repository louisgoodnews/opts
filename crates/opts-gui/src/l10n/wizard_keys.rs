//! Message keys of the master-detail pattern, the *Plan a drive* wizard and the loading screen.
//!
//! The strings live in `wizard_en.ftl` and `wizard_de.ftl`, merged into the `en` and `de` tables
//! by [`Localization`](super::Localization). Key tables may hold string literals.

/// Keys of the master-detail pattern.
pub mod md {
    /// Label (frame name) of the search field.
    pub const SEARCH_LABEL: &str = "md-search-label";
    /// Placeholder of the search field when the screen names no own key (variable `count`).
    pub const SEARCH: &str = "md-search";
    /// Header of the recent entries.
    pub const SECTION_RECENT: &str = "md-section-recent";
    /// Header of the starred entries.
    pub const SECTION_FAVOURITES: &str = "md-section-favourites";
    /// Header of the full list.
    pub const SECTION_ALL: &str = "md-section-all";
    /// Nothing matches the search (variable `query`).
    pub const NO_RESULTS: &str = "md-no-results";
    /// Button that removes search text and chips.
    pub const CLEAR_FILTER: &str = "md-clear-filter";
    /// Tooltip of an empty star.
    pub const STAR_ADD: &str = "md-star-add";
    /// Tooltip of a filled star.
    pub const STAR_REMOVE: &str = "md-star-remove";
}

/// Keys of the loading screen.
pub mod load;

mod all;
pub use all::ALL;

/// Keys of the *Plan a drive* wizard.
pub mod wiz;
