//! The row's content: the switch, the memory and time estimate, and the warning with its way back.

use super::*;

/// The switch, the estimate and, when it does not fit, the warning with its way out.
pub(super) fn content(
    commands: &mut Commands,
    loc: &Localization,
    on: bool,
    tiles: u32,
    profile: Option<&opts_hardware::HardwareProfile>,
) -> Vec<Entity> {
    let switch = SwitchBuilder::new(loc.t("wiz-load-all-title"))
        .with_value(on)
        .spawn(commands);
    commands
        .entity(switch)
        .insert((LoadAllSwitch, UiId::new("wizard.review.load_all")));
    let mut out = vec![switch];
    if tiles == 0 {
        out.push(note(commands, loc.t("wiz-load-all-unknown-map")));
        return out;
    }
    let Some(profile) = profile else {
        out.push(note(commands, loc.t("wiz-load-all-no-profile")));
        return out;
    };
    let estimate = estimate_load_all(profile, tiles, &MapStats::default());
    let args = [
        ("tiles".to_string(), (tiles as i32).into()),
        (
            "memory".to_string(),
            memory_text(estimate.memory_mib).into(),
        ),
        ("time".to_string(), time_text(estimate.seconds).into()),
    ];
    out.push(note(commands, loc.t_args("wiz-load-all-note", &args)));
    if !estimate.storage_measured {
        out.push(note(commands, loc.t("wiz-load-all-slow-disk")));
    }
    if estimate.verdict == MemoryVerdict::TooLarge {
        out.push(warning(commands, loc, estimate.memory_mib, profile));
    }
    out
}

fn note(commands: &mut Commands, text: String) -> Entity {
    commands
        .spawn(plain(
            text,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id()
}

/// Red text and the button back to the radius; shown whether or not the switch is on, so the
/// user learns it before switching.
fn warning(
    commands: &mut Commands,
    loc: &Localization,
    memory_mib: u64,
    profile: &opts_hardware::HardwareProfile,
) -> Entity {
    let safe = safe_resident_mib(profile).unwrap_or(0);
    let args = [
        ("memory".to_string(), memory_text(memory_mib).into()),
        ("safe".to_string(), memory_text(safe).into()),
    ];
    let text = commands
        .spawn(plain(
            loc.t_args("wiz-load-all-too-large", &args),
            TypeScale::BodyMedium,
            ColorRole::Error,
        ))
        .id();
    let button = ButtonBuilder::tonal("wiz-load-all-use-radius")
        .with_key("wiz-load-all-use-radius")
        .spawn(commands);
    commands
        .entity(button)
        .insert((UseRadiusButton, UiId::new("wizard.review.use_radius")));
    let box_ = commands.spawn(column(8.0)).id();
    commands.entity(box_).add_children(&[text, button]);
    box_
}
