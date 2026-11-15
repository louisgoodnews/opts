//! The schema of the input table: the producer, the variable kind and one row.

/// Where the engine takes an input from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The vehicle's physics (speeds, accelerations, wheels, odometer).
    Physics,
    /// The driver or the traffic planner (pedals, AI flags, target).
    Planner,
    /// The simulation clock (time of day, date, sun).
    Clock,
    /// Weather (temperature, humidity, precipitation).
    Weather,
    /// Environment of the vehicle (brightness, road condition, dirt, wear).
    Environment,
    /// The passenger system (counts, door requests).
    Passengers,
    /// Timetable state (schedule active).
    Timetable,
    /// Collision events.
    Collision,
    /// The mouse while a hot-spot is held (fed by the trigger system, not by [`feed`]).
    Mouse,
    /// Rail coupling state.
    Rail,
    /// Traffic rules (priority).
    Traffic,
    /// A fixed value no system produces yet.
    Constant,
}

/// Whether the name is a vehicle float variable (`(L.L.x)`) or a system value (`(L.S.x)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `(L.L.name)`.
    Float,
    /// `(L.S.name)`.
    System,
}

/// One input of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRow {
    /// Lower-case name.
    pub name: &'static str,
    /// Unit or value range.
    pub unit: &'static str,
    /// Producer.
    pub source: Source,
    /// Variable kind.
    pub kind: Kind,
    /// Scripts may store to this name too (AI scripts write the pedals, trailers write wheel
    /// rotations): the host writes it only when the set does not.
    pub shared: bool,
    /// A real value reaches it today; otherwise the feed writes the accepted zero and the
    /// integration report lists it as *zero-fed*.
    pub live: bool,
}
