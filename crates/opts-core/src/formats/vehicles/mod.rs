//! Vehicle configuration and operational network formats (`.bus`, `paths.cfg`).

pub mod bus;
pub mod error;
pub mod paths;

pub use bus::{parse_bus, BusAxle, BusManifest, FriendlyName, RegistrationMode};
pub use error::{BusParseError, PathsParseError};
pub use paths::{parse_paths, DoorBinding, PassengerPaths, PathLink, PathPoint, StepSoundPack};
