//! `opts-tools hardware report [--json] [--bench] [--storage-dir <dir>]`: prints the hardware
//! profile of this machine for the user. Nothing is written or sent anywhere.

use std::path::PathBuf;
use std::time::Duration;

use opts_hardware::bench::{self, BenchOptions, CancelToken};
use opts_hardware::inputs::ProbeInputs;
use opts_hardware::report::render_text;

/// Runs `hardware <args>`; `Err` is a usage or runtime message.
pub fn run(args: &[String]) -> Result<(), String> {
    let Some((sub, rest)) = args.split_first() else {
        return Err("expected `report`".into());
    };
    if sub != "report" {
        return Err(format!("unknown hardware command `{sub}`"));
    }
    let (mut json, mut bench_on) = (false, false);
    let (mut storage_dir, mut content_dir) = (None::<PathBuf>, None::<PathBuf>);
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--bench" => bench_on = true,
            "--storage-dir" => {
                storage_dir = Some(it.next().ok_or("--storage-dir needs a folder")?.into())
            }
            "--content-dir" => {
                content_dir = Some(it.next().ok_or("--content-dir needs a folder")?.into())
            }
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let mut profile = opts_hardware::probe_with(&ProbeInputs {
        content_dir,
        ..ProbeInputs::default()
    });
    if bench_on {
        let cancel = CancelToken::new();
        let c = cancel.clone();
        // Enter cancels the run; the thread is detached and ends with the process.
        std::thread::spawn(move || {
            let mut line = String::new();
            if std::io::stdin().read_line(&mut line).is_ok() {
                c.cancel();
            }
        });
        eprintln!("benchmarks: a few seconds; press Enter to cancel");
        let opts = BenchOptions {
            cpu_time: Duration::from_millis(400),
            storage_dir,
            ..BenchOptions::default()
        };
        bench::run(&mut profile, &opts, &cancel, &mut |step, i, n| {
            eprintln!("  [{}/{n}] {}", i + 1, step.key());
        });
    }
    if json {
        println!("{}", profile.to_json());
    } else {
        print!("{}", render_text(&profile));
    }
    Ok(())
}
