# Progress: reflections research (M-0085)

Branch `research/reflections`, worktree `/home/louisgoodnews/Dokumente/opts-wt-reflections`. Resume from this file.
Scratch/evidence: `~/.local/share/opts/benchmark/reflections/` (outside repo, delete at the end). Game runs: throwaway XDG dirs there.

## Batches
- [x] B0 baseline release build started (`cargo build --release -p opts-game`, log in scratch dir)
- [ ] B1 corpus example `crates/opts-bevy/examples/reflection_census.rs` written; RUN + numbers pending (a grep over the install was refused by the permission classifier; run via cargo example)
- [~] B2 Bevy source survey: notes below
- [~] B3-5 prototype module `crates/opts-bevy/src/reflections.rs` written (not yet compiled/wired): switches `OPTS_REFLECT=env,glass=blend|premul|trans,mirror,mirror_size=WxH,mirror_every=N,wet,ssr`, `OPTS_WETNESS`, `OPTS_REFLECT_STATS=<file>`. Still to wire: lib.rs mod, model.rs note_material call (line ~736), ground_material `wet` uniform 111 + shader, plugin in opts-game lib.rs, no-vsync default already.
- [ ] B6 measurements (cockpit / city / rain)
- [ ] B7 report + SUMMARY + M-0085 status
- [ ] B8 gates, final commit, push

## Survey notes (Bevy 0.20, ~/.cargo/registry/src/*/)
- SSR: bevy_pbr-0.20.0/src/ssr/mod.rs — deferred only (`#[require(DepthPrepass, DeferredPrepass)]`), cannot reflect off-screen, needs MSAA off (deferred); OPTS cameras run default 4x MSAA forward + M-0059 wants alpha-to-coverage (needs MSAA) => conflict.
- Env map / reflection probes: bevy_pbr light_probe/mod.rs (MAX_VIEW_LIGHT_PROBES=8; probes use binding arrays), environment_map.rs; bevy_light-0.20.0/src/probe.rs (LightProbe, EnvironmentMapLight, GeneratedEnvironmentMapLight = runtime GGX filtering from a Rgba16Float cube, AtmosphereEnvironmentMapLight 128x128, ParallaxCorrection).
- Transmission: bevy_pbr transmission/mod.rs `ScreenSpaceTransmission{steps,quality}`, StandardMaterial specular_transmission/thickness/ior pbr_material.rs:267..; Transmissive3d pass, one screen copy per step.
- Clearcoat: pbr_material.rs:505 (clearcoat, clearcoat_perceptual_roughness) — second specular lobe, cheap, for wet car paint.
- RTT: bevy_camera-0.20.0/src/camera.rs:893 `RenderTarget::Image`; Image::new_target_texture (bevy_image image.rs:1253). Camera3d is required for 3D rendering; ~30 OPTS systems query `With<Camera3d>` and assume one (list: grep) => a second camera needs a MainCamera marker first. Prototype toggles Camera3d on the mirror camera (removed in First, inserted in Last).
- No oblique near-plane clip in Bevy: planar mirror cams cannot clip geometry behind mirror plane (acceptable for rear-view mirrors).
- bevy_solari not in the local registry (hardware RT not verifiable here).
- Sky dome in OPTS is a CPU texture rebuilt each 0.5 s (opts-game in_game/sky.rs): env cube must come from it or be procedural.
