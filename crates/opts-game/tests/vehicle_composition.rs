//! The lookups of a composed rail vehicle ([`VehicleComposition`]): the parts by name, the number of
//! parts and the length. Moved out of `rail/composition.rs` (budget room in opts-game; M-0117).

use bevy::prelude::Entity;
use opts_game::in_game::omsi_world::{Part, VehicleComposition};

fn part(name: &str, behind: f32) -> Part {
    Part {
        entity: Entity::PLACEHOLDER,
        name: name.into(),
        behind,
        half_span: 1.0,
        reversed: false,
        meshes: 1,
        vertices: 3,
        body: None,
    }
}

#[test]
fn composition_finds_parts_by_name_and_reports_length() {
    let c = VehicleComposition {
        head: part("NF6D", 0.0),
        followers: vec![part("NF6D_C.bus", 12.5), part("NF6D_B_User.bus", 30.0)],
    };
    assert_eq!(c.len(), 3);
    assert!((c.length() - 30.0).abs() < 1e-6);
    assert_eq!(c.find("nf6d_c").map(|p| p.behind), Some(12.5));
    assert!(c.find("missing").is_none());
    assert_eq!(c.parts().count(), 3);
}
