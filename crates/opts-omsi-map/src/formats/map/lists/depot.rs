//! The depot groups of `ailists.cfg` (`[aigroup_depot]`), their fleet lines and the
//! `[aigroup_depot_typgroup_2]` blocks that fill them.

use super::AiLists;

/// One vehicle of a depot's fleet: a line `<number><TAB><registration><TAB><repaint>` after the
/// vehicle file of an `[aigroup_depot_typgroup_2]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetEntry {
    /// Fleet number (`101`).
    pub number: String,
    /// Registration plate (`WIN-SW 101`).
    pub plate: String,
    /// The repaint it wears (`Winsenburg`); empty when the line names none.
    pub repaint: String,
    /// First day (`yyyymmdd`) the vehicle is in service, if the line says.
    pub from: Option<u32>,
    /// Last day (`yyyymmdd`) the vehicle is in service, if the line says.
    pub to: Option<u32>,
}

impl FleetEntry {
    /// Whether the vehicle is in service on `date` (`yyyymmdd`).
    #[must_use]
    pub fn in_service(&self, date: u32) -> bool {
        self.from.is_none_or(|f| date >= f) && self.to.is_none_or(|t| date <= t)
    }
}

/// A depot group of `ailists.cfg` (`[aigroup_depot]`): the vehicles an operator's depot sends
/// out. A tour of the timetable names the group whose vehicles drive it.
#[derive(Debug, Clone, PartialEq)]
pub struct DepotGroup {
    /// Group name (`AVG Solowagen (Kranenburg)`), the `vehicle_group` of a timetable tour.
    pub name: String,
    /// The depot it belongs to (second line), empty when absent.
    pub depot: String,
    /// Vehicle files of its `[aigroup_depot_typgroup_2]` blocks, in file order (with repeats,
    /// which weigh the choice).
    pub vehicles: Vec<String>,
    /// The fleet lines of the type groups: vehicle file and the entry that follows it, in file
    /// order.
    pub fleet: Vec<(String, FleetEntry)>,
}

impl AiLists {
    pub(super) fn handle_depot(parameters: &[String], out: &mut Self) {
        out.depot_groups.push(DepotGroup {
            name: parameters
                .first()
                .map_or_else(String::new, |l| l.trim().to_owned()),
            depot: parameters
                .get(1)
                .map_or_else(String::new, |l| l.trim().to_owned()),
            vehicles: Vec::new(),
            fleet: Vec::new(),
        });
    }

    fn parse_fleet_entry(line: &str) -> Option<FleetEntry> {
        let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
        let number = cols.first().copied().filter(|c| !c.is_empty())?;
        let col = |i: usize| cols.get(i).copied().filter(|c| !c.is_empty());
        let (plate, repaint) = (col(1), col(2));
        let day = |i: usize| col(i).and_then(|c| c.parse::<u32>().ok());
        Some(FleetEntry {
            number: number.to_owned(),
            plate: plate.unwrap_or_default().to_owned(),
            repaint: repaint.unwrap_or_default().to_owned(),
            from: day(3),
            to: day(4),
        })
    }

    pub(super) fn handle_depot_typgroup(parameters: &[String], out: &mut Self) {
        let Some(file) = parameters.first().map(|l| l.trim()) else {
            return;
        };
        let lower = file.to_ascii_lowercase();
        let known = [".bus", ".ovh", ".zug"].iter().any(|e| lower.ends_with(e));
        if known {
            if let Some(g) = out.depot_groups.last_mut() {
                g.vehicles.push(file.to_owned());
                for line in parameters.iter().skip(1) {
                    if let Some(entry) = Self::parse_fleet_entry(line) {
                        g.fleet.push((file.to_owned(), entry));
                    }
                }
            }
            if !out
                .depot_vehicles
                .iter()
                .any(|v| v.eq_ignore_ascii_case(file))
            {
                out.depot_vehicles.push(file.to_owned());
            }
        }
    }
}
