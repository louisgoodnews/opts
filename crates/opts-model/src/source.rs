//! Where the engine gets content from, without knowing which format it came in.

use crate::vehicle::VehicleDefinition;

/// Why content could not be provided.
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    /// Nothing with this name.
    #[error("no such content: {0}")]
    NotFound(String),
    /// It exists but could not be read.
    #[error("could not read {name}: {reason}")]
    Unreadable {
        /// What was asked for.
        name: String,
        /// Why it failed.
        reason: String,
    },
}

/// A place content can come from: native files today, the OMSI installation through the importer.
/// The engine crates ask this trait; the game (the composition root) chooses the implementation.
/// Grows with the types of the model (maps, scenery, meshes).
pub trait ContentSource {
    /// The vehicle with this id (the `.bus` stem or the native file's name).
    ///
    /// # Errors
    /// The vehicle is not there or cannot be read.
    fn vehicle(&self, id: &str) -> Result<VehicleDefinition, ContentError>;
}
