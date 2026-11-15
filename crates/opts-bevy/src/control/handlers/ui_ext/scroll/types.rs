//! The scrollable kinds and the scrollable record that the handlers of the `scroll` command
//! work on.

use super::*;

/// What kind of view a [`Scrollable`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollKind {
    /// A node with `overflow: scroll` and a [`ScrollPosition`].
    Container,
    /// A windowed master list (only some rows exist as entities).
    List(Entity),
}

/// One scrollable view as the protocol sees it.
#[derive(Debug, Clone)]
pub struct Scrollable {
    /// Id for `ui.scroll target`.
    pub id: String,
    /// The node.
    pub entity: Entity,
    /// Kind.
    pub kind: ScrollKind,
    /// Rectangle in logical pixels.
    pub rect: Option<[f32; 4]>,
    /// Current offset `(x, y)` in logical pixels.
    pub offset: Vec2,
    /// Visible window size.
    pub viewport: Vec2,
    /// Content size.
    pub content: Vec2,
    /// Displayed (not hidden by an ancestor).
    pub visible: bool,
}

impl Scrollable {
    /// The largest offset.
    #[must_use]
    pub fn max(&self) -> Vec2 {
        (self.content - self.viewport).max(Vec2::ZERO)
    }
}
