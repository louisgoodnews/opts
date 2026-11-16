//! Probes (cheap descriptions tested against the filters) and queries over the records.

/// A cheap, borrowed description of a thing, tested against the filters before a record is built.
#[derive(Debug, Clone, Copy)]
pub struct Probe<'a> {
    /// One of `kind`.
    pub kind: &'a str,
    /// Tile the thing belongs to.
    pub tile: Option<(i32, i32)>,
    /// Source file (any form; matched case-insensitively).
    pub file: &'a str,
    /// Position in Bevy world space (x, z are used by the radius filter).
    pub world: [f32; 3],
}

/// A query over the in-memory records.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Only this tile.
    pub tile: Option<(i32, i32)>,
    /// Only this kind.
    pub kind: Option<String>,
    /// File pattern (substring, `*` wildcards, case-insensitive) matched against the record's
    /// file and its mesh files.
    pub file: Option<String>,
    /// Only records with issues.
    pub issues_only: bool,
    /// Within `radius` of `center` (Bevy x, z).
    pub near: Option<([f32; 2], f32)>,
    /// Only records with this decode stage in one of their meshes.
    pub stage: Option<String>,
    /// Most records returned.
    pub limit: usize,
}
