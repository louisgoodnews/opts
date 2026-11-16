//! Integration tests for texture decoding, format sniffing, and material bridging.

#![deny(unsafe_code)]

use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;
use bevy::render::render_resource::TextureFormat;
use opts_bevy::{
    dds_to_bevy_image, load_bevy_image, o3d_material_to_standard_material, sniff_image_format,
    specular_power_to_roughness, x_material_to_standard_material, ImageFormat, OptsAssetPlugin,
    OptsMaterialPlugin,
};
use opts_core::formats::o3d::Material as O3dMaterial;
use opts_core::formats::x::XMaterial;

mod dds;
mod images;
mod material;
mod synthetic;
