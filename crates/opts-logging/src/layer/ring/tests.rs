use tracing_subscriber::layer::SubscriberExt;

use super::*;

#[test]
fn captures_and_filters_events() {
    let subscriber = tracing_subscriber::registry().with(RingLayer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "ring_test::a", "hello world");
        tracing::warn!(target: "ring_test::b", code = 7, "careful");
        tracing::debug!(target: "ring_test::a", "noise");
    });
    let all = recent(
        50,
        &LogQuery {
            target: Some("ring_test".into()),
            ..LogQuery::default()
        },
    );
    assert_eq!(all.len(), 3);
    let warn = recent(
        50,
        &LogQuery {
            target: Some("ring_test".into()),
            min_level: Some(Level::WARN),
            ..LogQuery::default()
        },
    );
    assert_eq!(warn.len(), 1);
    assert_eq!(warn[0].message, "careful code=7");
    let found = recent(
        50,
        &LogQuery {
            contains: Some("hello".into()),
            target: Some("ring_test".into()),
            ..LogQuery::default()
        },
    );
    assert_eq!(found[0].level, "INFO");
    let last = recent(
        1,
        &LogQuery {
            target: Some("ring_test".into()),
            ..LogQuery::default()
        },
    );
    assert_eq!(last[0].message, "noise");
}
