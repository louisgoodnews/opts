//! Incident files for scrambled meshes whose texture coordinates the decoder could not prove.
//!
//! One text file per mesh and session in `<log directory>/incidents/`, written when more than 10 %
//! of the mesh's vertices are `Unreliable` (the same moment as the `WARN` in the log). It records
//! what the decoder knew: the mesh and its context, the decoder parameters, counts per quality and
//! per stage, why the weakest stages are weak, and the unreliable vertices themselves (position,
//! stored value, best guess, evidence). See `docs/src/architecture/uv-decoder.md`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use opts_core::formats::o3d::uvfix::{UvQuality, UvStage};

use crate::descramble_cache::Cached;

/// Most files kept in the incident directory; the oldest are removed beyond it.
pub const MAX_INCIDENT_FILES: usize = 500;
/// Most unreliable vertices listed in one file.
pub const MAX_VERTEX_ROWS: usize = 400;

static DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Sets the directory the incident files go to (the game passes `<log directory>/incidents`).
pub fn set_directory(dir: PathBuf) {
    if let Ok(mut d) = DIRECTORY.lock() {
        *d = Some(dir);
    }
}

/// The incident directory: the one set, else `opts-incidents` in the temporary directory.
#[must_use]
pub fn directory() -> PathBuf {
    DIRECTORY
        .lock()
        .ok()
        .and_then(|d| d.clone())
        .unwrap_or_else(|| std::env::temp_dir().join("opts-incidents"))
}

/// What the loader knows about the mesh when it reports it.
pub struct UvIncident<'a> {
    /// The `.o3d` file.
    pub mesh: &'a Path,
    /// What was being loaded (vehicle or object), empty when unknown.
    pub context: &'a str,
    /// Index of the model inside the file.
    pub model_index: usize,
    /// Size of the file in bytes.
    pub file_bytes: usize,
    /// Header: version, flags and key.
    pub header: (u32, u32, Option<u32>),
    /// Vertices and triangles of the file.
    pub vertices: usize,
    /// Triangles of the file before undecodable ones were dropped.
    pub triangles: usize,
    /// The decoding (positions, texture coordinates and the evidence behind them).
    pub decoded: &'a Cached,
    /// The stored (scrambled) texture coordinates.
    pub stored_uv: &'a [[f32; 2]],
    /// Decoder parameter fingerprint (cache key part).
    pub fingerprint: u32,
    /// Other files with the same name elsewhere in the installation.
    pub siblings: usize,
    /// Where the decoding came from: "decode cache", "decoded now" or "background decode".
    pub source: &'a str,
}

fn utc(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn explain(stage: UvStage, confidence: f32) -> &'static str {
    match stage {
        UvStage::Smooth if confidence < 2.0 => {
            "several offsets explained the neighbours about equally well (energy margin below 2): the best guess may be wrong"
        }
        UvStage::Smooth => "settled by the global energy with a clear margin",
        UvStage::Hinge => "predicted across a single shared edge from settled neighbours (about 87 % right on the twin data)",
        UvStage::Unresolved => "no stage could settle the vertex",
        UvStage::Pool => "equal to values repeated elsewhere in the mesh",
        UvStage::Plane => "fitted by a plane-affine model through jitter-free anchors",
        UvStage::Lattice => "verified by the lattice search",
        UvStage::Hint => "from jointly decoded copies",
    }
}

/// Writes the incident file; returns its path. Failures to write are ignored (diagnostics only).
#[must_use]
pub fn write(i: &UvIncident<'_>) -> Option<PathBuf> {
    let Cached::Decoded {
        positions,
        uvs,
        quality,
        stage,
        confidence,
        ..
    } = i.decoded
    else {
        return None;
    };
    let n = uvs.len();
    if quality.len() != n || stage.len() != n || i.stored_uv.len() != n {
        return None;
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let stem = i
        .mesh
        .file_stem()
        .map(|s| s.to_string_lossy().replace(|c: char| !c.is_ascii_alphanumeric(), "_"))
        .unwrap_or_default();
    let mut tag = 0xcbf2_9ce4_8422_2325_u64;
    for b in i.mesh.to_string_lossy().bytes().chain(i.context.bytes()) {
        tag = (tag ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    let dir = directory();
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("uv-{now}-{stem}-{:08x}.log", tag as u32));

    let mut counts = [0usize; 3];
    for q in quality {
        counts[usize::from(*q).min(2)] += 1;
    }
    let pct = |c: usize| 100.0 * c as f64 / n.max(1) as f64;
    let mut out = String::new();
    let _ = writeln!(out, "OPTS incident: unreliable texture coordinates");
    let _ = writeln!(out, "time: {} (unix {now})", utc(now));
    let _ = writeln!(out, "mesh: {}", i.mesh.display());
    let _ = writeln!(out, "context: {}", if i.context.is_empty() { "(none)" } else { i.context });
    let _ = writeln!(out, "model in file: {}   file size: {} bytes", i.model_index, i.file_bytes);
    let _ = writeln!(
        out,
        "header: version {} flags {} key {}",
        i.header.0,
        i.header.1,
        i.header.2.map_or("none".to_string(), |k| format!("{k:#010x}"))
    );
    let _ = writeln!(out, "vertices: {}   triangles: {}   vertex count mod 5: {}", i.vertices, i.triangles, i.vertices % 5);
    let _ = writeln!(out, "decoding source: {}   decoder parameter fingerprint: {:08x}", i.source, i.fingerprint);
    let _ = writeln!(out, "other files with this name in the installation: {}", i.siblings);
    let _ = writeln!(out);
    let _ = writeln!(out, "== Result ==");
    for (c, name) in ["Exact", "Recovered", "Unreliable"].iter().enumerate() {
        let _ = writeln!(out, "{name:11} {:7} vertices ({:5.1} %)", counts[c], pct(counts[c]));
    }
    let _ = writeln!(
        out,
        "\nA mesh is reported when more than 10 % of its vertices are Unreliable. The texture coordinates\nof those vertices are the decoder's best guess; on the twin data about 71 % of them are right."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "== What settled the vertices (stage: vertices, share, unreliable among them) ==");
    let mut per_stage = [[0usize; 2]; 7];
    for v in 0..n {
        let s = usize::from(stage[v]).min(6);
        per_stage[s][0] += 1;
        per_stage[s][1] += usize::from(quality[v] == UvQuality::Unreliable.code());
    }
    for (s, c) in per_stage.iter().enumerate() {
        if c[0] > 0 {
            let _ = writeln!(out, "{:22} {:7} {:5.1} %   unreliable {:7}", UvStage::from_code(s as u8).name(), c[0], pct(c[0]), c[1]);
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "== Why ==");
    let weak: Vec<(UvStage, f32)> = (0..n)
        .filter(|&v| quality[v] == UvQuality::Unreliable.code())
        .map(|v| (UvStage::from_code(stage[v]), confidence[v]))
        .collect();
    let mut reasons: Vec<(&'static str, usize)> = Vec::new();
    for (s, c) in &weak {
        let r = explain(*s, *c);
        match reasons.iter_mut().find(|e| e.0 == r) {
            Some(e) => e.1 += 1,
            None => reasons.push((r, 1)),
        }
    }
    reasons.sort_by_key(|e| std::cmp::Reverse(e.1));
    for (r, c) in &reasons {
        let _ = writeln!(out, "{c:7} vertices: {r}");
    }
    if i.vertices % 5 == 0 {
        let _ = writeln!(out, "note: the vertex count is a multiple of 5, so most vertices carry a u offset and there are few jitter-free anchors; these meshes are decoded worst (see docs/src/architecture/uv-decoder.md, section 1).");
    }
    if i.siblings == 0 {
        let _ = writeln!(out, "note: no other file with this name exists, so neither a plain twin nor a keyed copy can supply exact coordinates.");
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "== Unreliable vertices (first {} of {}): index, position (decoded), stored uv, best-guess uv, offset removed, stage, evidence ==",
        MAX_VERTEX_ROWS.min(weak.len()),
        weak.len()
    );
    let mut rows = 0;
    for v in 0..n {
        if quality[v] != UvQuality::Unreliable.code() {
            continue;
        }
        if rows == MAX_VERTEX_ROWS {
            break;
        }
        rows += 1;
        let (p, st, g) = (positions[v], i.stored_uv[v], uvs[v]);
        let _ = writeln!(
            out,
            "{v:6}  ({:9.4} {:9.4} {:9.4})  ({:8.4} {:8.4})  ({:8.4} {:8.4})  du {:+.4} dv {:+.4}  {}  {:.2}",
            p[0], p[1], p[2], st[0], st[1], g[0], g[1], st[0] - g[0], st[1] - g[1],
            UvStage::from_code(stage[v]).name(),
            confidence[v]
        );
    }
    std::fs::write(&path, out).ok()?;
    trim(&dir);
    Some(path)
}

/// Removes the oldest incident files beyond [`MAX_INCIDENT_FILES`].
fn trim(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("uv-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= MAX_INCIDENT_FILES {
        return;
    }
    files.sort();
    let excess = files.len() - MAX_INCIDENT_FILES;
    for (_, p) in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formats_known_instants() {
        assert_eq!(utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(utc(1_760_000_000), "2025-10-09 08:53:20 UTC");
    }

    #[test]
    fn incident_file_names_the_mesh_the_counts_and_the_weak_vertices() {
        let dir = std::env::temp_dir().join(format!("opts-incident-test-{}", std::process::id()));
        set_directory(dir.clone());
        let decoded = Cached::Decoded {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            uvs: vec![[0.1, 0.2], [0.3, 0.2], [0.1, 0.4]],
            kept: vec![true],
            quality: vec![0, 2, 2],
            stage: vec![2, 6, 5],
            confidence: vec![14.0, 0.4, 1.0],
        };
        let path = write(&UvIncident {
            mesh: Path::new("/omsi/Vehicles/X/Model/Panel.o3d"),
            context: "vehicle MAN NG272",
            model_index: 0,
            file_bytes: 1234,
            header: (5, 0, Some(0x31B6)),
            vertices: 3,
            triangles: 1,
            decoded: &decoded,
            stored_uv: &[[0.1, 0.2], [0.35, 0.2], [0.1, 0.45]],
            fingerprint: 0xabcd,
            siblings: 0,
            source: "decoded now",
        })
        .expect("file written");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Panel.o3d") && text.contains("vehicle MAN NG272"));
        assert!(text.contains("Unreliable") && text.contains("66.7 %"), "{text}");
        assert!(text.contains("energy margin below 2"), "{text}");
        assert!(text.contains("no other file with this name"), "{text}");
        // vertex 1: stored 0.35, guess 0.3: offset 0.05 removed
        assert!(text.contains("du +0.0500"), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
