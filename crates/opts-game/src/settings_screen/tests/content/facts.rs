//! The content page fills in as its facts arrive, and the scan chip waits for them.

use super::*;

/// M-0088: whatever the facts cost, the page builds within a frame budget, shows "Calculating..."
/// and fills in when the answer arrives. The provider takes 3 seconds in all.
///
/// Frame time is relative (ISS-030): the test measures idle frames of this same app while the
/// facts are still pending (the median is the baseline), then asserts that the worst frame during
/// fill-in is under the larger of ten times that baseline and the 33 ms real budget. A loaded
/// machine slows both measurements, so the ratio holds; on a quiet machine the limit is 33 ms.
#[test]
fn the_content_page_builds_within_a_frame_and_fills_in_when_the_facts_arrive() {
    const FRAME_BUDGET: Duration = Duration::from_millis(33);
    const BASELINE_FRAMES: usize = 5;
    const FACTOR: u32 = 10;
    let started = std::time::Instant::now();
    let (mut app, _, _) = page_app_with(
        "content-freeze",
        "content",
        Arc::new(SlowFacts(Duration::from_millis(1500))),
    );
    let keys =
        |app: &mut App| -> Vec<(super::super::super::info::ContentLine, &'static str, String)> {
            let mut q = app.world_mut().query::<(
                &super::super::super::info::ContentLine,
                &opts_gui::l10n::L10nKey,
                &opts_gui::l10n::L10nArgs,
            )>();
            q.iter(app.world())
                .map(|(l, k, a)| (*l, k.0, format!("{:?}", a.0)))
                .collect()
        };
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "building the page waited for the facts"
    );
    let first = keys(&mut app);
    assert_eq!(first.len(), 3, "the three lines exist at once: {first:?}");
    assert!(
        first.iter().all(|(_, k, _)| k.ends_with("-pending")),
        "placeholders first: {first:?}"
    );
    let mut idle: Vec<Duration> = (0..BASELINE_FRAMES)
        .map(|_| {
            let t = std::time::Instant::now();
            app.update();
            t.elapsed()
        })
        .collect();
    idle.sort();
    let baseline = idle[BASELINE_FRAMES / 2];
    let limit = (baseline * FACTOR).max(FRAME_BUDGET);
    let mut worst = Duration::ZERO;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let t = std::time::Instant::now();
        app.update();
        worst = worst.max(t.elapsed());
        let now = keys(&mut app);
        if now.iter().all(|(_, k, _)| !k.ends_with("-pending")) {
            let all: String = now.iter().map(|(_, k, a)| format!("{k} {a}")).collect();
            for n in ["679543", "75313", "947", "20000"] {
                assert!(all.contains(n), "{n} missing in {all}");
            }
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the facts never arrived: {now:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        worst < limit,
        "a frame took {worst:?} while the facts were gathered (limit {limit:?}, idle baseline {baseline:?})"
    );
}

/// The scan chip never claims a verdict before the facts are there.
#[test]
fn the_scan_chip_says_it_is_checking_until_the_facts_arrive() {
    let (mut app, _, _) = page_app_with(
        "content-chip",
        "content",
        Arc::new(SlowFacts(Duration::from_millis(600))),
    );
    let label = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<&opts_gui::l10n::L10nKey, With<super::super::super::scan_status::ScanChipLabel>>();
        q.iter(app.world()).next().map(|k| k.0)
    };
    assert_eq!(label(&mut app), Some("scan-chip-pending"));
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while label(&mut app) == Some("scan-chip-pending") {
        assert!(std::time::Instant::now() < deadline, "no verdict");
        app.update();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(label(&mut app), Some("scan-chip"));
}
