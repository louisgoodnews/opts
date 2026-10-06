//! Parser and data model for OMSI `.org` registration and fleet number lists.
//!
//! `.org` files represent pools of identifiers from which vehicles draw fleet numbers
//! or license plates. Repeated entries indicate frequency weighting.
//!
//! See specification in `docs/src/formats/text/org.md`.

use std::collections::HashMap;

pub mod error;
pub use error::OrgParseError;

use crate::formats::text::decode_bytes;

/// A parsed `.org` pool of vehicle identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationPool {
    /// Ordered list of all entries (including duplicates).
    entries: Vec<String>,
    /// Frequency count for each unique identifier.
    weights: HashMap<String, usize>,
}

impl RegistrationPool {
    /// Parses a `.org` list from a text string.
    ///
    /// # Errors
    /// Returns [`OrgParseError::EmptyPool`] if the text yields zero valid entries.
    pub fn parse(text: &str) -> Result<Self, OrgParseError> {
        let mut entries = Vec::new();
        let mut weights = HashMap::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            entries.push(trimmed.to_string());
            *weights.entry(trimmed.to_string()).or_insert(0) += 1;
        }

        if entries.is_empty() {
            return Err(OrgParseError::EmptyPool);
        }

        Ok(Self { entries, weights })
    }

    /// Parses a `.org` list from raw bytes, supporting UTF-8 and Windows-1252.
    ///
    /// # Errors
    /// Returns [`OrgParseError`] on decoding failure or if the pool is empty.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, OrgParseError> {
        let text = decode_bytes(bytes);
        Self::parse(&text)
    }

    /// Returns a slice of all raw entries in the pool, in declared order.
    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Returns the total number of entries in the pool (including duplicates).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the number of distinct unique identifiers in the pool.
    #[must_use]
    pub fn unique_count(&self) -> usize {
        self.weights.len()
    }

    /// Returns the occurrence count (weight) of a given identifier.
    #[must_use]
    pub fn weight(&self, entry: &str) -> usize {
        self.weights.get(entry).copied().unwrap_or(0)
    }

    /// Returns the entry at the given linear index, or `None` if out of bounds.
    #[must_use]
    pub fn select_by_index(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(String::as_str)
    }

    /// Selects an entry weighted by frequency using an index in `0..total_count()`.
    ///
    /// Because the raw entry list preserves duplicates with uniform index spacing,
    /// selecting uniformly over `0..total_count()` yields exact frequency weighting.
    #[must_use]
    pub fn select_weighted(&self, weight_index: usize) -> Option<&str> {
        self.select_by_index(weight_index)
    }
}

/// Convenience function to parse `.org` content from a string slice.
pub fn parse_org(text: &str) -> Result<RegistrationPool, OrgParseError> {
    RegistrationPool::parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_pool() {
        let sample = "B-KL 7093\r\nB-PU 8018\nB-KH 3463\r\n";
        let pool = parse_org(sample).unwrap();
        assert_eq!(pool.total_count(), 3);
        assert_eq!(pool.unique_count(), 3);
        assert_eq!(pool.select_by_index(0), Some("B-KL 7093"));
        assert_eq!(pool.select_by_index(1), Some("B-PU 8018"));
        assert_eq!(pool.select_by_index(2), Some("B-KH 3463"));
        assert_eq!(pool.select_by_index(3), None);
    }

    #[test]
    fn test_parse_weighted_pool() {
        let sample = "A\nA\nA\nB\n";
        let pool = parse_org(sample).unwrap();
        assert_eq!(pool.total_count(), 4);
        assert_eq!(pool.unique_count(), 2);
        assert_eq!(pool.weight("A"), 3);
        assert_eq!(pool.weight("B"), 1);
        assert_eq!(pool.weight("C"), 0);
    }

    #[test]
    fn test_empty_pool_rejected() {
        let empty = "   \r\n\n  \t  \r\n";
        assert_eq!(parse_org(empty), Err(OrgParseError::EmptyPool));
    }

    #[test]
    fn test_parse_cp1252_bytes() {
        // "München" in CP1252: 'ü' is 0xFC
        let cp1252_bytes = vec![0x4D, 0xFC, 0x6E, 0x63, 0x68, 0x65, 0x6E, 0x0A];
        let pool = RegistrationPool::parse_bytes(&cp1252_bytes).unwrap();
        assert_eq!(pool.select_by_index(0), Some("München"));
    }
}
