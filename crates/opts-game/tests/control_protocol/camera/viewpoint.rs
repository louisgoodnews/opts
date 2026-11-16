//! Integration tests for the control protocol camera: viewpoint get, set, save and reset.
use super::*;

#[test]
fn camera_viewpoint_get_set_save_reset() {
    let mut app = drive_app();
    let path = scratch("viewpoint").join("viewpoints.json");
    app.init_resource::<opts_bevy::ViewpointState>()
        .add_message::<opts_game::hud::RaiseAlert>()
        .insert_resource(opts_game::viewpoints::Viewpoints {
            path: Some(path.clone()),
            store: opts_game::viewpoints::ViewpointStore::default(),
        });

    // Outside the driver's view there is nothing to edit.
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("not cockpit");
    assert_eq!(e.code, code::NOT_READY);
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("no identity");
    assert_eq!(e.code, code::NOT_READY);
    app.world_mut()
        .resource_mut::<opts_bevy::ViewpointState>()
        .vehicle_key = Some("man_sd77".to_owned());

    let got = call(&mut app, "camera.viewpoint.get", json!({})).expect("get");
    assert_eq!(got["editing"], false);
    assert_eq!(got["vehicle"], "man_sd77");
    assert_eq!(got["limits"]["offset_m"], 0.6);

    // Limits are refused, not clamped.
    for bad in [
        json!({"x": 1.0}),
        json!({"y": -0.7}),
        json!({"fov_deg": 10.0}),
        json!({"fov_deg": 130.0}),
        json!({}),
    ] {
        let e = call(&mut app, "camera.viewpoint.set", bad.clone()).expect_err("limit");
        assert_eq!(e.code, code::INVALID_PARAMS, "{bad}");
    }
    assert_eq!(
        call(&mut app, "camera.viewpoint.get", json!({})).expect("get")["editing"],
        false,
        "a refused set opens nothing"
    );

    let set = call(
        &mut app,
        "camera.viewpoint.set",
        json!({"x": 0.1, "y": -0.05, "fov_deg": 85.0}),
    )
    .expect("set");
    assert_eq!(set["editing"], true);
    assert_eq!(set["dirty"], true);
    assert!((set["viewpoint"]["x"].as_f64().unwrap() - 0.1).abs() < 1e-6);
    assert_eq!(set["viewpoint"]["fov_deg"], 85.0);
    assert!(!path.exists(), "nothing is stored before save");

    let e = call(&mut app, "camera.viewpoint.save", json!({"scope": "model"}))
        .expect_err("the model is unknown");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(
        &mut app,
        "camera.viewpoint.save",
        json!({"scope": "galaxy"}),
    )
    .expect_err("scope");
    assert_eq!(e.code, code::INVALID_PARAMS);
    let saved = call(&mut app, "camera.viewpoint.save", json!({})).expect("save");
    assert_eq!(saved["editing"], false);
    assert_eq!(saved["dirty"], false);
    assert!((saved["stored"]["y"].as_f64().unwrap() + 0.05).abs() < 1e-6);
    let text = std::fs::read_to_string(&path).expect("file written before the answer");
    assert!(
        text.contains("man_sd77") && text.contains("\"version\": 1"),
        "{text}"
    );

    let reset = call(&mut app, "camera.viewpoint.reset", json!({})).expect("reset");
    assert_eq!(reset["viewpoint"]["x"], 0.0);
    assert!(reset["viewpoint"]["fov_deg"].is_null());
    let text = std::fs::read_to_string(&path).expect("file");
    assert!(!text.contains("man_sd77"), "the entry is gone: {text}");

    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.viewpoint.set", json!({"x": 0.1})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}

#[test]
fn camera_reset_resets_the_current_view_and_follows_the_gates() {
    let mut app = drive_app();
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::External;
        s.external_yaw = 2.0;
        s.external_zoom = 33.0;
    }
    let r = call(&mut app, "camera.reset", json!({})).expect("reset external");
    assert_eq!(r["mode"], "external");
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.external_yaw, s.external_zoom), (0.0, 14.0));

    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::Cockpit;
        s.sub_camera_index = 2;
        s.view_yaw = 1.0;
    }
    let r = call(&mut app, "camera.reset", json!({})).expect("reset cockpit");
    assert_eq!(
        (r["mode"].as_str(), r["sub_camera"].as_u64()),
        (Some("cockpit"), Some(2))
    );
    assert_eq!(app.world().resource::<CameraState>().view_yaw, 0.0);

    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::FreeRoam;
        s.free_roam_speed = 80.0;
    }
    call(&mut app, "camera.reset", json!({})).expect("reset free");
    assert_eq!(
        app.world().resource::<CameraState>().free_roam_speed,
        opts_bevy::FREE_ROAM_MIN_SPEED
    );

    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.reset", json!({})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}

#[test]
fn camera_viewpoint_save_for_the_model_is_written_to_the_models_section() {
    let mut app = drive_app();
    let path = scratch("viewpoint-model").join("viewpoints.json");
    app.init_resource::<opts_bevy::ViewpointState>()
        .add_message::<opts_game::hud::RaiseAlert>()
        .insert_resource(opts_game::viewpoints::Viewpoints {
            path: Some(path.clone()),
            store: opts_game::viewpoints::ViewpointStore::default(),
        });
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    {
        let mut st = app.world_mut().resource_mut::<opts_bevy::ViewpointState>();
        st.vehicle_key = Some("man_sd77".to_owned());
        st.model_key = Some("man_sd200".to_owned());
    }
    call(&mut app, "camera.viewpoint.set", json!({"z": 0.1})).expect("set");
    let got = call(&mut app, "camera.viewpoint.save", json!({"scope": "model"})).expect("save");
    assert_eq!(got["model"], "man_sd200");
    let text = std::fs::read_to_string(&path).expect("file");
    let v: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(v["models"]["man_sd200"]["cameras"]["0"]["z"], 0.1);
    assert!(v["vehicles"].as_object().expect("vehicles").is_empty());
}
