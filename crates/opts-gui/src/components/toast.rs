//! Material Design 3 Toast / Snackbar notification widget.

use crate::components::Button;
use bevy::prelude::*;

use crate::{
    interaction::{activation::Activated, events::ToastDismissed, state::Disabled},
    interaction::{
        focus::{FocusRegion, Landmark},
        layer::StateLayerRole,
    },
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

/// Marker component for an M3 Toast root entity.
#[derive(Component, Debug, Clone)]
pub struct M3Toast {
    pub message: String,
    pub timer: Option<Timer>,
}

/// How serious a toast is; sets its colours and how long it stays.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastSeverity {
    /// Something finished (4 s).
    #[default]
    Info,
    /// Something to be aware of (10 s).
    Warning,
    /// Something failed (10 s). An error is never *only* a toast: its owner also shows it in a
    /// durable place (inline, banner or dialog).
    Error,
}

impl ToastSeverity {
    /// Seconds a toast of this severity stays by default.
    pub const fn default_seconds(self) -> f32 {
        match self {
            Self::Info => 4.0,
            Self::Warning | Self::Error => 10.0,
        }
    }

    fn roles(self) -> (ColorRole, ColorRole) {
        match self {
            Self::Info => (ColorRole::InverseSurface, ColorRole::InverseOnSurface),
            Self::Warning => (ColorRole::TertiaryContainer, ColorRole::OnTertiaryContainer),
            Self::Error => (ColorRole::ErrorContainer, ColorRole::OnErrorContainer),
        }
    }

    /// Colour of the action label: the accent of the inverse surface for information, the text
    /// colour of the container otherwise (the inverse accent is unreadable on those).
    fn action_role(self) -> ColorRole {
        match self {
            Self::Info => ColorRole::InversePrimary,
            Self::Warning | Self::Error => self.roles().1,
        }
    }
}

/// Spawn order of a toast (stacking order; the newest has the highest number).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToastSeq(pub u64);

/// Toasts with the same key are merged while one is alive (a counter shows how often).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ToastDedupe(pub String);

/// How many identical notifications this toast stands for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastCount(pub u32);

/// A toast waiting for a free slot (at most [`crate::components::toast_stack::MAX_VISIBLE`] show
/// at once); its timer does not run.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToastQueued;

static NEXT_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Marker component for the optional action button on a toast.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastActionButton(pub Entity);

/// Builder for constructing Material Design 3 toast notifications.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_toast(mut commands: Commands) {
///     ToastBuilder::new("Service dispatched successfully")
///         .with_duration(4.0)
///         .with_action("DISMISS")
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ToastBuilder {
    pub message: String,
    pub duration_secs: Option<f32>,
    pub action_label: Option<String>,
    /// Severity (colours, default duration).
    pub severity: ToastSeverity,
    /// Key for merging identical toasts.
    pub dedupe_key: Option<String>,
}

impl ToastBuilder {
    /// Creates a new toast builder with the specified message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            duration_secs: Some(4.0),
            action_label: None,
            severity: ToastSeverity::Info,
            dedupe_key: None,
        }
    }

    /// Sets the severity; the duration follows unless it was set explicitly afterwards.
    pub fn with_severity(mut self, severity: ToastSeverity) -> Self {
        self.severity = severity;
        self.duration_secs = Some(severity.default_seconds());
        self
    }

    /// Merges this toast with an identical live one (same key) instead of stacking a second.
    pub fn with_dedupe_key(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    /// Sets the display duration in seconds before automatic dismissal (or None for persistent).
    pub fn with_duration(mut self, seconds: f32) -> Self {
        self.duration_secs = Some(seconds);
        self
    }

    /// Makes the toast persistent until manually dismissed.
    pub fn persistent(mut self) -> Self {
        self.duration_secs = None;
        self
    }

    /// Sets an optional action button label (e.g. "UNDO" or "CLOSE").
    pub fn with_action(mut self, action_label: impl Into<String>) -> Self {
        self.action_label = Some(action_label.into());
        self
    }

    fn root_bundle(&self) -> impl Bundle {
        let timer = self
            .duration_secs
            .map(|secs| Timer::from_seconds(secs, TimerMode::Once));
        (
            (
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    min_width: Val::Px(280.0),
                    max_width: Val::Px(560.0),
                    min_height: Val::Px(48.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                    column_gap: Val::Px(16.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    right: Val::Px(24.0),
                    ..default()
                },
                GlobalZIndex(100),
            ),
            M3Toast {
                message: self.message.clone(),
                timer,
            },
            FocusRegion(Landmark::Toasts),
            BackgroundRole(self.severity.roles().0),
            RadiusToken(RadiusLevel::ExtraSmall),
            ElevationToken(ElevationLevel::Level3),
            self.severity,
            ToastSeq(NEXT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            ToastCount(1),
        )
    }

    /// Spawns the toast into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let dedupe = self.dedupe_key.clone();
        let message = self.message.clone();
        let action = self.action_label.clone();
        let severity = self.severity;
        let root = commands.spawn(self.root_bundle()).id();
        if let Some(key) = dedupe {
            commands.entity(root).insert(ToastDedupe(key));
        }
        commands.entity(root).with_children(|parent| {
            populate_toast_children(parent, root, &message, action.as_deref(), severity);
        });
        root
    }

    /// Spawns the toast as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let message = self.message.clone();
        let action = self.action_label.clone();
        let severity = self.severity;
        let mut root_cmd = parent.spawn(self.root_bundle());
        let root = root_cmd.id();
        root_cmd.with_children(|child_parent| {
            populate_toast_children(child_parent, root, &message, action.as_deref(), severity);
        });
        root
    }
}

fn populate_toast_children(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    message: &str,
    action_label: Option<&str>,
    severity: ToastSeverity,
) {
    parent.spawn((
        (
            Text::new(message),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor::default(),
        ),
        ForegroundRole(severity.roles().1),
        TypeStyleToken(TypeScale::BodyMedium),
    ));

    if let Some(action) = action_label {
        parent
            .spawn((
                (
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                        min_width: Val::Px(48.0),
                        min_height: Val::Px(32.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ),
                StateLayerRole(severity.roles().1),
                ToastActionButton(root),
            ))
            .with_children(|btn_parent| {
                btn_parent.spawn((
                    (
                        Text::new(action),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    ForegroundRole(severity.action_role()),
                    TypeStyleToken(TypeScale::BodyMedium),
                ));
            });
    }
}

/// System ticking toast timers and despawning expired toasts.
pub fn toast_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut toast_query: Query<(Entity, &mut M3Toast), Without<ToastQueued>>,
    mut events: MessageWriter<ToastDismissed>,
) {
    for (entity, mut toast) in &mut toast_query {
        if let Some(ref mut timer) = toast.timer {
            timer.tick(time.delta());
            if timer.just_finished() {
                events.write(ToastDismissed { entity });
                commands.entity(entity).despawn();
            }
        }
    }
}

/// System handling clicks on toast action buttons.
pub fn toast_action_click_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    action_query: Query<&ToastActionButton, Without<Disabled>>,
    mut events: MessageWriter<ToastDismissed>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(action) = action_query.get(*entity) {
            events.write(ToastDismissed { entity: action.0 });
            commands.entity(action.0).despawn();
        }
    }
}

/// Plugin registering Toast widget systems and events.
#[derive(Default)]
pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<ToastDismissed>>() {
            app.add_message::<ToastDismissed>();
        }
        app.add_systems(Update, (toast_timer_system, toast_action_click_system));
    }
}

#[cfg(test)]
mod tests;
