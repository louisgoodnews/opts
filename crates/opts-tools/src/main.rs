//! OPTS asset toolsuite.
//!
//! - `opts-tools inspect <file>...` validates OMSI files with the real parsers. The format is
//!   chosen by file extension (`.o3d`, `.x`, `.sco`, `.sli`, `.oft`, `.terrain`, `.map` tiles).
//!   Each file prints one line; the exit code is 1 if any file failed.
//! - `opts-tools hardware report [--json] [--bench]` prints the hardware profile (local only, see
//!   [`hardware`]).
//! - `opts-tools l10n export|check|new|import|stats` prepares, checks and measures language packs
//!   (see [`l10n`] and `docs/src/architecture/localisation-packs.md`).
//! - `opts-tools image-metrics [--crop x,y,w,h] [--json] <a.png> [<b.png>]` measures screenshots
//!   (saturation, luminance, high-frequency energy) and the difference of two of them; see
//!   [`metrics`].

mod hardware;
mod image_metrics;
mod inspect;
mod l10n;
mod metrics;

use std::path::Path;
use std::process::ExitCode;

use image_metrics::image_metrics;
use inspect::inspect;

const USAGE: &str = concat!(
    "usage: opts-tools inspect <file>...\n       opts-tools image-metrics [--crop x,y,w,h] [--json] <a.png> [<b.png>]\n",
    "       opts-tools l10n export|check|new|import|stats ... (see `opts-tools l10n`)\n",
    "       opts-tools hardware report [--json] [--bench] [--storage-dir <dir>] [--content-dir <dir>]"
);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, files)) = args.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    if cmd == "l10n" {
        let o = l10n::execute(files);
        print!("{}", o.out);
        eprint!("{}", o.err);
        return ExitCode::from(u8::try_from(o.code).unwrap_or(2));
    }
    if cmd == "image-metrics" {
        return match image_metrics(files) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("image-metrics: {e}\n{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if cmd == "hardware" {
        return match hardware::run(files) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("hardware: {e}\n{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    if cmd != "inspect" || files.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut failed = 0;
    for f in files {
        match inspect(Path::new(f)) {
            Ok(summary) => println!("OK    {f}: {summary}"),
            Err(e) => {
                failed += 1;
                println!("FAIL  {f}: {e}");
            }
        }
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
