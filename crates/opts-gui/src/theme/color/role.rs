//! The semantic colour roles: the M3 roles and the status families, with their `On…` pairs.

/// Semantic color roles defined by Material Design 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorRole {
    // Primary group
    Primary,
    OnPrimary,
    PrimaryContainer,
    OnPrimaryContainer,

    // Secondary group
    Secondary,
    OnSecondary,
    SecondaryContainer,
    OnSecondaryContainer,

    // Tertiary group
    Tertiary,
    OnTertiary,
    TertiaryContainer,
    OnTertiaryContainer,

    // Error group
    Error,
    OnError,
    ErrorContainer,
    OnErrorContainer,

    // Surface & background group
    Surface,
    OnSurface,
    SurfaceVariant,
    OnSurfaceVariant,
    InverseSurface,
    InverseOnSurface,
    InversePrimary,
    SurfaceDim,
    SurfaceBright,
    SurfaceContainerLowest,
    SurfaceContainerLow,
    SurfaceContainer,
    SurfaceContainerHigh,
    SurfaceContainerHighest,

    // Outline & utility group
    Outline,
    OutlineVariant,
    Scrim,
    Shadow,

    // Status group (semantic roles M3 lacks)
    Success,
    OnSuccess,
    SuccessContainer,
    OnSuccessContainer,
    Warning,
    OnWarning,
    WarningContainer,
    OnWarningContainer,
    Info,
    OnInfo,
    InfoContainer,
    OnInfoContainer,
}

impl ColorRole {
    /// Returns a static slice of all standard color roles.
    pub const fn all() -> &'static [ColorRole] {
        &[
            ColorRole::Primary,
            ColorRole::OnPrimary,
            ColorRole::PrimaryContainer,
            ColorRole::OnPrimaryContainer,
            ColorRole::Secondary,
            ColorRole::OnSecondary,
            ColorRole::SecondaryContainer,
            ColorRole::OnSecondaryContainer,
            ColorRole::Tertiary,
            ColorRole::OnTertiary,
            ColorRole::TertiaryContainer,
            ColorRole::OnTertiaryContainer,
            ColorRole::Error,
            ColorRole::OnError,
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
            ColorRole::Surface,
            ColorRole::OnSurface,
            ColorRole::SurfaceVariant,
            ColorRole::OnSurfaceVariant,
            ColorRole::InverseSurface,
            ColorRole::InverseOnSurface,
            ColorRole::InversePrimary,
            ColorRole::SurfaceDim,
            ColorRole::SurfaceBright,
            ColorRole::SurfaceContainerLowest,
            ColorRole::SurfaceContainerLow,
            ColorRole::SurfaceContainer,
            ColorRole::SurfaceContainerHigh,
            ColorRole::SurfaceContainerHighest,
            ColorRole::Outline,
            ColorRole::OutlineVariant,
            ColorRole::Scrim,
            ColorRole::Shadow,
            ColorRole::Success,
            ColorRole::OnSuccess,
            ColorRole::SuccessContainer,
            ColorRole::OnSuccessContainer,
            ColorRole::Warning,
            ColorRole::OnWarning,
            ColorRole::WarningContainer,
            ColorRole::OnWarningContainer,
            ColorRole::Info,
            ColorRole::OnInfo,
            ColorRole::InfoContainer,
            ColorRole::OnInfoContainer,
        ]
    }

    /// Number of roles (the length of [`ColorRole::all`]).
    pub const COUNT: usize = 46;

    /// The `On…` role whose colour is drawn on top of this role, if this role is a fill.
    pub const fn on_role(self) -> Option<ColorRole> {
        Some(match self {
            Self::Primary => Self::OnPrimary,
            Self::PrimaryContainer => Self::OnPrimaryContainer,
            Self::Secondary => Self::OnSecondary,
            Self::SecondaryContainer => Self::OnSecondaryContainer,
            Self::Tertiary => Self::OnTertiary,
            Self::TertiaryContainer => Self::OnTertiaryContainer,
            Self::Error => Self::OnError,
            Self::ErrorContainer => Self::OnErrorContainer,
            Self::Success => Self::OnSuccess,
            Self::SuccessContainer => Self::OnSuccessContainer,
            Self::Warning => Self::OnWarning,
            Self::WarningContainer => Self::OnWarningContainer,
            Self::Info => Self::OnInfo,
            Self::InfoContainer => Self::OnInfoContainer,
            Self::Surface => Self::OnSurface,
            Self::SurfaceVariant => Self::OnSurfaceVariant,
            Self::InverseSurface => Self::InverseOnSurface,
            _ => return None,
        })
    }
}
