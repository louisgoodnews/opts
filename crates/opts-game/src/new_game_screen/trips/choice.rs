//! The runs an offer is made of: the chosen trip and the offered run, with their labels.

use super::*;

/// The trip chosen for the drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripChoice {
    /// Line file stem.
    pub line_file: String,
    /// Tour index in the line file.
    pub tour: usize,
    /// Run index in the tour.
    pub run: usize,
    /// Station id of the first stop (the bus stop object's ID code in the map).
    pub station_id: u32,
    /// Name of the first stop.
    pub stop_name: String,
    /// Line text.
    pub line: String,
    /// Destination text.
    pub destination: String,
    /// Departure from the first stop (s after midnight).
    pub depart_s: u32,
}

impl TripChoice {
    /// The list id: `line_file:tour:run`.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}:{}:{}", self.line_file, self.tour, self.run)
    }

    /// `HH:MM` of the departure.
    #[must_use]
    pub fn depart_text(&self) -> String {
        let m = (self.depart_s / 60) % (24 * 60);
        format!("{:02}:{:02}", m / 60, m % 60)
    }

    /// `92 → Stadtgrenze`; a trip without a line text (a depot run) is its destination alone.
    #[must_use]
    pub fn title(&self) -> String {
        if self.line.is_empty() {
            self.destination.clone()
        } else {
            format!("{} → {}", self.line, self.destination)
        }
    }
}

/// A run the step offers.
#[derive(Debug, Clone, PartialEq)]
pub struct TripOption {
    /// What choosing it sets.
    pub choice: TripChoice,
    /// The tour's name.
    pub tour_name: String,
    /// Name of the last stop.
    pub last_stop: String,
    /// Number of stops.
    pub stops: usize,
}
