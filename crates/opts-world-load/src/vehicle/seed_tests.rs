//! Tests: seed_tests.

use super::script_seed;
use std::path::Path;

#[test]
fn the_seed_follows_the_vehicle_and_its_slot_not_the_spawn_order() {
    let root = Path::new("/omsi");
    let a = Path::new("/omsi/Vehicles/MAN_NG272/GN92.bus");
    let again = script_seed(root, a, 1);
    assert_eq!(script_seed(root, a, 1), again, "stable");
    assert_ne!(script_seed(root, a, 2), again, "another slot");
    assert_ne!(
        script_seed(root, Path::new("/omsi/Vehicles/MAN_NG272/GN93.bus"), 1),
        again,
        "another vehicle"
    );
    // The same vehicle in another installation folder, with the path written the other way.
    assert_eq!(
        script_seed(
            Path::new("/other"),
            Path::new("/other/vehicles\\man_ng272\\gn92.bus"),
            1
        ),
        again
    );
}
