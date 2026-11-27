//! ISS-070: the tile parse coordinator hands out each job's result once, runs no more than its
//! in-flight limit and drops a cancelled tile. Missing files and empty bytes have hand-computed
//! outcomes; no game data is read.

use std::path::{Path, PathBuf};
use std::time::Duration;

use opts_bevy::map_stream::{
    parse_tile_bytes, TileKey, TileParseCoordinator, TileParseError, TileParseResult,
};

/// A path that does not exist under the test's working directory.
fn missing(key: TileKey) -> PathBuf {
    PathBuf::from(format!("no-such-dir-iss070/tile_{}_{}.map", key.0, key.1))
}

/// Polls until `key` has a result (at most about 5 s) and returns it.
fn finish(c: &mut TileParseCoordinator, key: TileKey) -> TileParseResult {
    for _ in 0..5000 {
        c.poll();
        if let Some(result) = c.take(key) {
            return result;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("tile {key:?} did not finish within 5 s");
}

#[test]
fn missing_file_reports_read_error_with_path() {
    let mut c = TileParseCoordinator::new(4);
    let key = (3, 4);
    assert!(c.request(key, missing(key)));
    match finish(&mut c, key) {
        Err(TileParseError::Read(msg)) => assert!(msg.contains("tile_3_4.map"), "{msg}"),
        other => panic!("expected a read error, got {other:?}"),
    }
    // A result is handed out once.
    assert!(c.take(key).is_none());
    assert!(!c.is_known(key));
}

#[test]
fn duplicate_request_is_refused_while_known() {
    let mut c = TileParseCoordinator::new(4);
    let key = (1, 1);
    assert!(c.request(key, missing(key)));
    assert!(!c.request(key, missing(key)));
    assert_eq!(c.queued_count(), 1);
    assert_eq!(c.running_count(), 0);
}

#[test]
fn in_flight_limit_holds_the_rest_in_the_queue() {
    let mut c = TileParseCoordinator::new(2);
    assert_eq!(c.max_in_flight(), 2);
    for x in 0..3 {
        assert!(c.request((x, 0), missing((x, 0))));
    }
    // Nothing starts before a poll.
    assert_eq!(c.queued_count(), 3);
    assert_eq!(c.running_count(), 0);
    // Harvest finds nothing; two slots start, one job waits. Checked straight after the poll,
    // before any later poll could harvest a finished job.
    c.poll();
    assert_eq!(c.running_count(), 2);
    assert_eq!(c.queued_count(), 1);
    for x in 0..3 {
        assert!(matches!(finish(&mut c, (x, 0)), Err(TileParseError::Read(_))));
    }
    assert_eq!(c.queued_count(), 0);
    assert_eq!(c.running_count(), 0);
}

#[test]
fn zero_limit_is_raised_to_one() {
    let c = TileParseCoordinator::new(0);
    assert_eq!(c.max_in_flight(), 1);
}

#[test]
fn cancel_drops_a_queued_or_running_tile() {
    let mut c = TileParseCoordinator::new(1);
    let (a, b) = ((5, 5), (6, 5));
    assert!(c.request(a, missing(a)));
    assert!(c.request(b, missing(b)));
    c.poll();
    assert_eq!(c.running_count(), 1);
    assert_eq!(c.queued_count(), 1);
    c.cancel(a);
    c.cancel(b);
    assert_eq!(c.running_count(), 0);
    assert_eq!(c.queued_count(), 0);
    assert!(!c.is_known(a));
    assert!(c.take(a).is_none());
    // A cancelled tile can be requested again.
    assert!(c.request(a, missing(a)));
}

#[test]
fn empty_bytes_parse_to_an_empty_tile() {
    let tile = parse_tile_bytes(b"").expect("empty input is an empty tile");
    assert_eq!(tile.version, None);
    assert!(!tile.has_terrain);
    assert!(!tile.has_water);
}

#[test]
fn parse_of_missing_path_is_not_a_parse_error() {
    let path: &Path = Path::new("no-such-dir-iss070/tile_0_0.map");
    let result = opts_bevy::map_stream::parse_tile_file(path);
    assert!(matches!(result, Err(TileParseError::Read(_))));
}
