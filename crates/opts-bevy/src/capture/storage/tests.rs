//! Unit tests for `super` (`storage.rs`).

use super::*;

fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("opts_capstore_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("temp dir");
    p
}

#[test]
fn traversal_and_absolute_names_are_rejected() {
    for bad in [
        "../../etc/passwd",
        "/etc/passwd",
        "a/../../b",
        "./x.png",
        "a//b/../../../c",
        "",
        "dir/",
        "C:\\x.png",
        "x\u{0}y",
    ] {
        let r = validate_relative(bad);
        // A drive prefix is only absolute on Windows; elsewhere it is one plain segment, and
        // `resolve_name` then keeps it inside the root like any other name.
        if bad.starts_with("C:") && cfg!(unix) {
            continue;
        }
        assert!(matches!(r, Err(CaptureError::Forbidden(_))), "{bad:?}");
    }
    assert_eq!(
        validate_relative("run1/shot.png").expect("plain"),
        PathBuf::from("run1").join("shot.png")
    );
}

#[test]
fn names_resolve_inside_the_root_and_create_parents() {
    let root = temp("a");
    let p = resolve_name(&root, "run1/deep/shot.png").expect("inside");
    assert!(p.starts_with(root.canonicalize().expect("root")));
    assert!(p.parent().expect("parent").is_dir());
}

#[cfg(unix)]
#[test]
fn a_symlink_leading_out_of_the_root_is_rejected() {
    let (root, outside) = (temp("b"), temp("c"));
    std::os::unix::fs::symlink(&outside, root.join("link")).expect("symlink");
    let err = resolve_name(&root, "link/shot.png").unwrap_err();
    assert!(matches!(err, CaptureError::Forbidden(_)), "{err}");
}

#[test]
fn explicit_paths_must_be_inside_the_root() {
    let (root, other) = (temp("d"), temp("e"));
    assert!(resolve_explicit(&root, &root.join("a.png")).is_ok());
    assert!(matches!(
        resolve_explicit(&root, &other.join("a.png")),
        Err(CaptureError::Forbidden(_))
    ));
    assert!(matches!(
        resolve_explicit(&root, Path::new("/etc/passwd")),
        Err(CaptureError::Forbidden(_))
    ));
}

#[test]
fn generated_stems_are_unique_and_carry_the_session() {
    let mut s = CaptureState::new(None);
    let (a, b) = (s.next_stem(), s.next_stem());
    assert_ne!(a, b);
    assert!(a.contains(&s.session_id));
    assert!(a.ends_with("_0001") && b.ends_with("_0002"));
}

#[test]
fn free_space_is_reported_for_an_existing_directory() {
    let root = temp("g");
    // CI containers may hide their mounts, so only a reported value is checked.
    if let Some(free) = free_bytes(&root) {
        assert!(free > 0);
    }
}

#[test]
fn atomic_write_leaves_no_part_file() {
    let root = temp("f");
    let p = root.join("x.bin");
    write_atomic(&p, b"abc").expect("write");
    assert_eq!(std::fs::read(&p).expect("read"), b"abc");
    assert!(!root.join("x.bin.part").exists());
}
