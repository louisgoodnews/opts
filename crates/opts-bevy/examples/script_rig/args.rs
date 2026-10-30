//! The command line of the script rig.

use super::*;

/// The options of one rig run.
pub(super) struct Args {
    pub(super) root: PathBuf,
    pub(super) filter: Option<String>,
    pub(super) frames: usize,
    pub(super) json: Option<PathBuf>,
    pub(super) csv: Option<PathBuf>,
    pub(super) limit: usize,
    pub(super) golden: Option<PathBuf>,
    pub(super) check: Option<PathBuf>,
    pub(super) physics: bool,
    pub(super) hz: f32,
    pub(super) route_code: Option<i32>,
    pub(super) hof_filter: Option<String>,
    pub(super) map_name: Option<String>,
    pub(super) timetable: Option<(String, usize, usize)>,
}

impl Args {
    /// Parses `std::env::args`; panics on an unknown argument.
    pub(super) fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut a = Self {
            root: PathBuf::from(args.next().expect("OMSI root")),
            filter: None,
            frames: 720,
            json: None,
            csv: None,
            limit: usize::MAX,
            golden: None,
            check: None,
            physics: false,
            hz: 60.0,
            route_code: None,
            hof_filter: None,
            map_name: None,
            timetable: None,
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--vehicle" => a.filter = args.next().map(|s| s.to_lowercase()),
                "--frames" => a.frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(720),
                "--json" => a.json = args.next().map(PathBuf::from),
                "--csv" => a.csv = args.next().map(PathBuf::from),
                "--physics" => a.physics = true,
                "--route" => a.route_code = args.next().and_then(|v| v.parse().ok()),
                "--hof" => a.hof_filter = args.next().map(|s| s.to_lowercase()),
                "--map" => a.map_name = args.next(),
                "--timetable" => {
                    a.timetable = args.next().and_then(|v| {
                        let mut p = v.rsplitn(3, '/');
                        let run = p.next()?.parse().ok()?;
                        let tour = p.next()?.parse().ok()?;
                        Some((p.next()?.to_string(), tour, run))
                    });
                }
                "--hz" => a.hz = args.next().and_then(|v| v.parse().ok()).unwrap_or(60.0),
                "--golden" => a.golden = args.next().map(PathBuf::from),
                "--check" => a.check = args.next().map(PathBuf::from),
                "--limit" => {
                    a.limit = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(usize::MAX)
                }
                other => panic!("unknown argument {other}"),
            }
        }
        a
    }
}
