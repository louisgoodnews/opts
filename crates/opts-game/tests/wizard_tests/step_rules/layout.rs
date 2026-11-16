//! Layout of the wizard steps: the date and time block fits a 1024 window at 100 and 150 percent.

use super::*;

#[test]
fn the_date_and_time_block_fits_a_1024_window_at_100_and_150_percent() {
    for scale in [1.0_f32, 1.5] {
        let mut app = wizard("fit");
        {
            let mut q = app.world_mut().query::<&mut Window>();
            for mut w in q.iter_mut(app.world_mut()) {
                w.resolution.set(1024.0, 640.0);
            }
            app.world_mut().insert_resource(UiScale(scale));
        }
        // The time block sits at the left of step When and must fit the window; the date block
        // sits in the map step's detail pane and must fit that pane.
        for (stepper, name, in_window) in [
            ("wizard.stepper.where", "DateBlock", false),
            ("wizard.stepper.when", "TimeBlock", true),
        ] {
            press(&mut app, stepper);
            for _ in 0..4 {
                app.update();
            }
            let mut q = app.world_mut().query::<(
                Entity,
                &Name,
                &ComputedNode,
                &UiGlobalTransform,
                Option<&ChildOf>,
            )>();
            let rect = |c: &ComputedNode, t: &UiGlobalTransform| {
                (
                    c.size() * c.inverse_scale_factor,
                    t.translation * c.inverse_scale_factor,
                )
            };
            let (block, parent) = q
                .iter(app.world())
                .find(|(_, n, ..)| n.as_str() == name)
                .map(|(_, _, c, t, p)| (rect(c, t), p.map(ChildOf::parent)))
                .expect("the picker block");
            let limit = if in_window {
                1024.0 / scale
            } else {
                let parent = parent.expect("a parent");
                let (_, _, c, t, _) = q.get(app.world(), parent).expect("parent node");
                let (size, centre) = rect(c, t);
                centre.x + size.x / 2.0
            };
            let right = block.1.x + block.0.x / 2.0;
            assert!(
                right <= limit + 1.0,
                "{name} at scale {scale}: right edge {right} beyond {limit}"
            );
            assert!(block.0.x <= 560.0 + 1.0);
        }
        cleanup("fit");
    }
}
