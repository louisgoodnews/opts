//! Writer of `MapTile`: the tile body, object and spline blocks.

use super::*;

impl MapTile {
    /// Serialises the tile's `[version]` and `[object]` records as `tile_X_Y.map` text.
    ///
    /// Version 10 tiles are written without pitch and roll, as that version stores them. In
    /// other versions a missing pitch or roll is written as `0`, so it reads back as `Some(0.0)`.
    ///
    /// # Errors
    /// [`MapWriteError::UnsupportedTileContent`] when the tile has splines, rules, spline
    /// attachments or terrain/water flags; [`MapWriteError::UnwritableLine`] for a string
    /// (path or string variable) that would not read back unchanged.
    pub fn to_text(&self) -> Result<String, MapWriteError> {
        let unsupported = [
            (!self.splines.is_empty(), "[spline]"),
            (!self.rules.is_empty(), "[rule]"),
            (!self.spline_attachments.is_empty(), "[splineAttachement]"),
            (self.has_terrain, "[terrain]"),
            (self.has_water, "[water]"),
            (self.has_variable_terrain, "[variable_terrain]"),
            (
                self.has_variable_terrain_lightmap,
                "[variable_terrainlightmap]",
            ),
        ];
        if let Some((_, what)) = unsupported.iter().find(|(present, _)| *present) {
            return Err(MapWriteError::UnsupportedTileContent(what));
        }
        let mut out = String::new();
        if let Some(v) = self.version {
            block(&mut out, "version", &[v.to_string()]);
        }
        let legacy = self.version == Some(10);
        for obj in &self.objects {
            block(&mut out, "object", &object_lines(obj, legacy)?);
        }
        Ok(out)
    }
}
