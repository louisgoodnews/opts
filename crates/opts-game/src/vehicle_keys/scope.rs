//! The vehicles a vehicle key action is for.




/// The vehicles a key action is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Buses, trolleybuses and trams.
    Both,
    /// Buses and trolleybuses.
    Bus,
    /// Trams.
    Tram,
}

impl Scope {
    /// Whether an action of this scope applies to a vehicle (`tram`: it runs on rails).
    #[must_use]
    pub fn applies(self, tram: bool) -> bool {
        match self {
            Self::Both => true,
            Self::Bus => !tram,
            Self::Tram => tram,
        }
    }

    /// Whether two scopes can be in use on the same vehicle, so their keys must differ.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self == Self::Both || other == Self::Both || self == other
    }
}
