//! Editing operations of the formation editor, on the model itself so they can be tested without
//! a UI. Every operation keeps the formation well-formed; indices out of range change nothing.

use super::{FormationSegment, Orientation, TrainFormation, VehicleRef};

impl TrainFormation {
    /// Appends a car (running forward) and returns its index.
    pub fn add_file(&mut self, file: &str) -> usize {
        self.segments.push(FormationSegment {
            vehicle: VehicleRef::File(file.to_owned()),
            orientation: Orientation::Forward,
            powered: None,
        });
        self.segments.len() - 1
    }

    /// Removes car `index`; false when there is none.
    pub fn remove_car(&mut self, index: usize) -> bool {
        if index < self.segments.len() {
            self.segments.remove(index);
            true
        } else {
            false
        }
    }

    /// Moves car `index` by `delta` places (negative: towards the front) and returns its new
    /// index; `None` when it would leave the train or `index` is out of range.
    pub fn move_car(&mut self, index: usize, delta: isize) -> Option<usize> {
        let to = index.checked_add_signed(delta)?;
        if index >= self.segments.len() || to >= self.segments.len() {
            return None;
        }
        let car = self.segments.remove(index);
        self.segments.insert(to, car);
        Some(to)
    }

    /// Turns car `index` around; false when there is none.
    pub fn turn_car(&mut self, index: usize) -> bool {
        match self.segments.get_mut(index) {
            Some(s) => {
                s.orientation = s.orientation.flipped();
                true
            }
            None => false,
        }
    }

    /// Cycles the `powered` override of car `index`: automatic, pulls, hauled, automatic.
    /// Returns the new value; `None` when there is no such car.
    pub fn cycle_powered(&mut self, index: usize) -> Option<Option<bool>> {
        let s = self.segments.get_mut(index)?;
        s.powered = match s.powered {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
        Some(s.powered)
    }

    /// A name for an unnamed formation: the first car's file name and the car count
    /// (`BR218_OR + 3`), or `Formation` when it is empty.
    #[must_use]
    pub fn suggested_name(&self) -> String {
        let Some(first) = self.segments.first() else {
            return "Formation".to_owned();
        };
        let text = first.vehicle.as_str();
        let stem = text
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(text)
            .rsplit_once('.')
            .map_or_else(
                || text.rsplit(['\\', '/']).next().unwrap_or(text),
                |(s, _)| s,
            );
        match self.segments.len() {
            1 => stem.to_owned(),
            n => format!("{stem} + {}", n - 1),
        }
    }
}
