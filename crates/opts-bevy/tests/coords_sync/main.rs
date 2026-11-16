//! Integration tests for coordinate frame conversion functions, components,
//! and Bevy ECS synchronization systems.

use bevy::app::App;
use bevy::math::Vec3;
use bevy::transform::components::Transform;
use bevy::MinimalPlugins;

use opts_bevy::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    OmsiCoords, OmsiTransform, OptsCoordsPlugin,
};

mod angles;
mod mapping;
mod sync;
