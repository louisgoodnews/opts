//! The script textures: their lifecycle and the drawing callbacks.

use super::*;

#[test]
fn host_callbacks_script_textures_lifecycle_and_drawing() {
    let canvas = CpuScriptTexture::new(64, 32);
    let textures = Arc::new(RwLock::new(vec![canvas]));
    let ctx = VehicleHostContext {
        script_textures: textures.clone(),
        ..Default::default()
    };
    let mut h = vehicle_callbacks_with_context(ctx);
    let mut st = Stacks::default();

    // 1. Lock
    st.numbers.push(0.0);
    assert!(h("stlock", &mut st));
    assert!(textures.read().unwrap()[0].is_locked());

    // 2. Set color: A=255, R=255, G=128, B=64
    st.numbers.push(0.0); // tex
    st.numbers.push(255.0); // A
    st.numbers.push(255.0); // R
    st.numbers.push(128.0); // G
    st.numbers.push(64.0); // B
    assert!(h("stsetcolor", &mut st));

    // 3. Draw rect
    st.numbers.push(0.0); // tex
    st.numbers.push(2.0); // x1
    st.numbers.push(2.0); // y1
    st.numbers.push(10.0); // x2
    st.numbers.push(10.0); // y2
    assert!(h("stdrawrect", &mut st));

    // 4. Read pixel
    st.numbers.push(0.0); // tex
    st.numbers.push(5.0); // x
    st.numbers.push(5.0); // y
    assert!(h("streadpixel", &mut st));

    // 5. Get R, G, B, A
    st.numbers.push(0.0);
    assert!(h("stgetr", &mut st));
    assert_eq!(st.numbers.pop(), Some(255.0));

    st.numbers.push(0.0);
    assert!(h("stgetg", &mut st));
    assert_eq!(st.numbers.pop(), Some(128.0));

    st.numbers.push(0.0);
    assert!(h("stgetb", &mut st));
    assert_eq!(st.numbers.pop(), Some(64.0));

    st.numbers.push(0.0);
    assert!(h("stgeta", &mut st));
    assert_eq!(st.numbers.pop(), Some(255.0));

    // 6. Unlock
    st.numbers.push(0.0);
    assert!(h("stunlock", &mut st));
    assert!(!textures.read().unwrap()[0].is_locked());
    assert!(textures.read().unwrap()[0].is_dirty());
}
