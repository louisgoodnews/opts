//! What Home shows as a function of the installation and the catalogue (pure, testable).

/// The catalogue as Home sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogueStatus {
    /// No catalogue (no OMSI folder, or it could not be opened).
    Unavailable,
    /// A scan is running.
    Reading,
    /// Scanned: how many maps and vehicles it holds.
    Ready {
        /// Maps.
        maps: usize,
        /// Playable vehicles.
        vehicles: usize,
    },
}

/// Everything Home depends on besides the drive history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeInputs {
    /// Whether an OMSI installation is known.
    pub omsi_found: bool,
    /// The catalogue.
    pub catalogue: CatalogueStatus,
    /// Whether a scan is running (the chip says so even when the lists already hold content).
    pub refreshing: bool,
}

/// A problem Home reports in a banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeBanner {
    /// No OMSI installation: action *Choose folder*.
    NoOmsi,
    /// The catalogue is ready but holds no maps or no vehicles.
    EmptyCatalogue,
}

/// Why starting a drive is not possible right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveBlock {
    /// Nothing blocks it.
    None,
    /// No OMSI folder.
    NoOmsi,
    /// The catalogue is still being read for the first time.
    Reading,
    /// The catalogue has no maps or no vehicles.
    Empty,
}

impl HomeInputs {
    /// The banner to show, if any.
    pub fn banner(&self) -> Option<HomeBanner> {
        if !self.omsi_found {
            return Some(HomeBanner::NoOmsi);
        }
        match self.catalogue {
            CatalogueStatus::Ready { maps, vehicles } if maps == 0 || vehicles == 0 => {
                Some(HomeBanner::EmptyCatalogue)
            }
            _ => None,
        }
    }

    /// What blocks the drive buttons.
    pub fn drive_block(&self) -> DriveBlock {
        if !self.omsi_found {
            return DriveBlock::NoOmsi;
        }
        match self.catalogue {
            CatalogueStatus::Unavailable => DriveBlock::NoOmsi,
            CatalogueStatus::Reading => DriveBlock::Reading,
            CatalogueStatus::Ready { maps, vehicles } if maps == 0 || vehicles == 0 => {
                DriveBlock::Empty
            }
            CatalogueStatus::Ready { .. } => DriveBlock::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(omsi_found: bool, catalogue: CatalogueStatus) -> HomeInputs {
        HomeInputs {
            omsi_found,
            catalogue,
            refreshing: false,
        }
    }

    #[test]
    fn without_omsi_there_is_a_banner_and_no_drive() {
        let i = inputs(false, CatalogueStatus::Unavailable);
        assert_eq!(i.banner(), Some(HomeBanner::NoOmsi));
        assert_eq!(i.drive_block(), DriveBlock::NoOmsi);
    }

    #[test]
    fn an_empty_catalogue_is_reported_but_a_reading_one_is_not_an_error() {
        let empty = inputs(
            true,
            CatalogueStatus::Ready {
                maps: 0,
                vehicles: 3,
            },
        );
        assert_eq!(empty.banner(), Some(HomeBanner::EmptyCatalogue));
        assert_eq!(empty.drive_block(), DriveBlock::Empty);
        let reading = inputs(true, CatalogueStatus::Reading);
        assert_eq!(reading.banner(), None);
        assert_eq!(reading.drive_block(), DriveBlock::Reading);
    }

    #[test]
    fn a_filled_catalogue_blocks_nothing() {
        let i = inputs(
            true,
            CatalogueStatus::Ready {
                maps: 4,
                vehicles: 9,
            },
        );
        assert_eq!(i.banner(), None);
        assert_eq!(i.drive_block(), DriveBlock::None);
    }
}
