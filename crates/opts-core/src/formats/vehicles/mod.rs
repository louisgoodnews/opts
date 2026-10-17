//! Vehicle configuration and operational network formats (`.bus`, `paths.cfg`, `model.cfg`, `.hof`).

pub mod ai_profile;
pub mod bus;
pub mod error;
pub mod hof;
pub mod model;
pub mod paths;
pub mod script_texture;

pub use ai_profile::{AiKind, AiProfile, VehicleDomain};
pub use bus::{
    parse_bus, BusAxle, BusCamera, BusManifest, CouplingLimits, FriendlyName, RegistrationMode,
    VehicleExtras,
};
pub use error::{BusParseError, PathsParseError};
pub use hof::{HofBusStop, HofFile, HofParseError, HofRoute, HofTerminus};
pub use model::{
    parse_model, MaterialModifier, ModelAnim, ModelConfig, ModelLod, ModelMaterial, ModelMesh,
    ModelParseError, ViewpointFlags,
};
pub use paths::{parse_paths, DoorBinding, PassengerPaths, PathLink, PathPoint, StepSoundPack};
pub use script_texture::{CpuScriptTexture, ScriptTextureCanvas};
