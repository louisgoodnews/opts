//! Builds the road lane graph of a real map and reports how well the geometric connectivity rule
//! works. Usage: `cargo run -p opts-traffic --example lane_graph_audit -- <map dir>`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::{
    SceneryObject, ScoPath, ScoTrafficLightGroup, SplinePath, SplineProfile,
};
use opts_traffic::{
    segments_from_tile, signals_from_tile, tile_origin, Aspect, ConnectParams, LaneGraph, LaneKind,
    PathSource,
};

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

struct Files {
    root: PathBuf,
    sli: HashMap<String, Option<Vec<SplinePath>>>,
    sco: HashMap<String, Option<SceneryObject>>,
}

impl PathSource for Files {
    fn sli_paths(&mut self, f: &str) -> Option<&[SplinePath]> {
        let root = &self.root;
        self.sli
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SplineProfile::parse(&bytes).ok().map(|p| p.paths)
            })
            .as_deref()
    }

    fn sco_paths(&mut self, f: &str) -> Option<&[ScoPath]> {
        let root = &self.root;
        self.sco
            .entry(f.to_owned())
            .or_insert_with(|| {
                let bytes = std::fs::read(find_ci(root, f)?).ok()?;
                SceneryObject::parse(&bytes).ok()
            })
            .as_ref()
            .map(|o| o.paths.as_slice())
    }

    fn sco_lights(&mut self, f: &str) -> Option<&[ScoTrafficLightGroup]> {
        self.sco_paths(f)?;
        self.sco
            .get(f)?
            .as_ref()
            .map(|o| o.traffic_light_groups.as_slice())
    }
}

fn main() {
    let map_dir = PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let root = map_dir
        .parent()
        .and_then(Path::parent)
        .expect("OMSI root")
        .to_path_buf();
    let mut files = Files {
        root,
        sli: HashMap::new(),
        sco: HashMap::new(),
    };
    let mut segments = Vec::new();
    let mut programs = HashMap::new();
    for e in std::fs::read_dir(&map_dir).expect("map dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(rest) = name
            .strip_prefix("tile_")
            .and_then(|r| r.strip_suffix(".map"))
        else {
            continue;
        };
        let mut it = rest.split('_');
        let (Some(Ok(x)), Some(Ok(y))) = (
            it.next().map(str::parse::<i32>),
            it.next().map(str::parse::<i32>),
        ) else {
            continue;
        };
        let Some(tile) = std::fs::read(e.path())
            .ok()
            .and_then(|b| MapTile::parse(&b).ok())
        else {
            continue;
        };
        let origin = tile_origin(x, y);
        segments.extend(segments_from_tile(&tile, origin, &mut files));
        programs.extend(signals_from_tile(&tile, &mut files));
    }
    segments.retain(|s| s.kind == LaneKind::Road);
    let params = ConnectParams::default();
    let t = std::time::Instant::now();
    let graph = LaneGraph::build(&segments, &params);
    let ms = t.elapsed().as_millis();
    let n = graph.len();
    let with_succ = graph
        .lane_ids()
        .filter(|&l| !graph.successors(l).is_empty())
        .count();
    let mut has_pred = vec![false; n];
    for l in graph.lane_ids() {
        for s in graph.successors(l) {
            has_pred[s.0 as usize] = true;
        }
    }
    let with_pred = has_pred.iter().filter(|&&b| b).count();
    let branching = graph
        .lane_ids()
        .filter(|&l| graph.successors(l).len() > 1)
        .count();
    // Weakly connected components.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut c = x;
        while p[c] != r {
            let next = p[c];
            p[c] = r;
            c = next;
        }
        r
    }
    for l in graph.lane_ids() {
        for s in graph.successors(l) {
            let (a, b) = (
                find(&mut parent, l.0 as usize),
                find(&mut parent, s.0 as usize),
            );
            parent[a] = b;
        }
    }
    let mut sizes: HashMap<usize, usize> = HashMap::new();
    for i in 0..n {
        *sizes.entry(find(&mut parent, i)).or_default() += 1;
    }
    let mut sizes: Vec<usize> = sizes.into_values().collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    let total_len: f64 = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .map(|l| l.length())
        .sum();
    println!(
        "road segments: {}  directed lanes: {n}  total length: {:.1} km  build: {ms} ms",
        segments.len(),
        total_len / 1000.0
    );
    println!(
        "lanes with a successor: {with_succ} ({:.1} %), with a predecessor: {with_pred} ({:.1} %), branching: {branching}",
        100.0 * with_succ as f64 / n.max(1) as f64,
        100.0 * with_pred as f64 / n.max(1) as f64
    );
    let t = std::time::Instant::now();
    let conflicts: Vec<usize> = graph.lane_ids().map(|l| graph.conflicts(l).len()).collect();
    println!(
        "conflict zones: {} lanes have some ({:.1} %), {} zones in total, max {} on one lane; computed for all lanes in {} ms",
        conflicts.iter().filter(|&&c| c > 0).count(),
        100.0 * conflicts.iter().filter(|&&c| c > 0).count() as f64 / n.max(1) as f64,
        conflicts.iter().sum::<usize>(),
        conflicts.iter().max().unwrap_or(&0),
        t.elapsed().as_millis()
    );
    let signalled = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .filter(|l| l.signal.is_some())
        .count();
    let with_program = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .filter_map(|l| l.signal)
        .filter(|k| programs.contains_key(k))
        .count();
    let mut aspects = [0usize; 4];
    for p in programs.values() {
        for t in (0..p.cycle() as usize).step_by(1) {
            aspects[match p.aspect_at(t as f64) {
                Aspect::Stop => 0,
                Aspect::Go => 1,
                Aspect::Yellow => 2,
                Aspect::Off => 3,
            }] += 1;
        }
    }
    let total: usize = aspects.iter().sum::<usize>().max(1);
    println!(
        "lights: {} programs, {signalled} lanes governed ({with_program} with a known program); share of the cycle red {:.0} %, green {:.0} %, yellow {:.0} %, dark {:.0} %",
        programs.len(),
        100.0 * aspects[0] as f64 / total as f64,
        100.0 * aspects[1] as f64 / total as f64,
        100.0 * aspects[2] as f64 / total as f64,
        100.0 * aspects[3] as f64 / total as f64,
    );
    let t = std::time::Instant::now();
    let neighbours: Vec<usize> = graph
        .lane_ids()
        .map(|l| graph.neighbours(l).len())
        .collect();
    println!(
        "neighbour lanes: {} lanes have one ({:.1} %); computed for all lanes in {} ms",
        neighbours.iter().filter(|&&c| c > 0).count(),
        100.0 * neighbours.iter().filter(|&&c| c > 0).count() as f64 / n.max(1) as f64,
        t.elapsed().as_millis()
    );
    let lanes = || graph.lane_ids().filter_map(|l| graph.lane(l));
    println!(
        "rules on lanes: speed limit {}, density {}, priority {}, trucks {}, bus {}, no_cars {}, no overtaking {}",
        lanes().filter(|l| l.speed_limit.is_some()).count(),
        lanes().filter(|l| l.rules.density.is_some()).count(),
        lanes().filter(|l| l.rules.priority.is_some()).count(),
        lanes().filter(|l| l.rules.trucks).count(),
        lanes().filter(|l| l.rules.bus).count(),
        lanes().filter(|l| l.rules.no_cars).count(),
        lanes().filter(|l| l.rules.no_overtaking).count(),
    );
    println!(
        "components: {}  largest: {:?}",
        sizes.len(),
        &sizes[..sizes.len().min(5)]
    );
    for tol in [0.25, 0.75, 2.0] {
        let g = LaneGraph::build(
            &segments,
            &ConnectParams {
                position_tolerance: tol,
                ..params
            },
        );
        let joined = g
            .lane_ids()
            .filter(|&l| !g.successors(l).is_empty())
            .count();
        println!("position tolerance {tol:>4} m: {joined} lanes with a successor");
    }
}
