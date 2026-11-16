//! Folds one vehicle's run into the totals and writes its report line.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;

use super::super::*;
use super::*;

/// Adds one to the counter of every name.
fn bump<'a>(counts: &mut BTreeMap<String, usize>, names: impl IntoIterator<Item = &'a String>) {
    for n in names {
        *counts.entry(n.clone()).or_default() += 1;
    }
}

/// Folds one vehicle's run into the totals, writes its JSON line and checks the golden.
pub(in super::super) fn report_vehicle(
    args: &Args,
    totals: &mut Totals,
    info: &VehicleInfo<'_>,
    engine: &ScriptEngine,
    run: VehicleRun,
    out: &mut Option<std::fs::File>,
) {
    let VehicleRun {
        plan,
        usage,
        undeclared,
        samples,
        runtime,
        non_finite,
        ops_total,
        micros_total,
    } = run;
    let frames = args.frames.max(1) as f64;
    totals.frame_us.push(micros_total / frames);

    let unfed_reads: BTreeSet<String> = usage
        .reads
        .iter()
        .filter(|n| !usage.writes.contains(*n) && plan.status(n).is_none())
        .filter(|n| undeclared.contains(*n))
        .cloned()
        .collect();
    let unfed_system: BTreeSet<String> = usage
        .system_reads
        .iter()
        .filter(|n| plan.status(n).is_none() && !ENGINE_SET.contains(&n.as_str()))
        .cloned()
        .collect();
    let stats = engine.callback_stats();
    let stub_names: BTreeSet<String> = stats.stub.keys().cloned().collect();
    let real_names: BTreeSet<String> = stats.real.keys().cloned().collect();
    let unknown_names: BTreeSet<String> = stats.unknown.keys().cloned().collect();
    let unconsumed_outputs = unconsumed(&usage.writes);
    let underflows = runtime.iter().filter(|m| m.contains("underflow")).count();
    totals.underflow_vehicles += usize::from(underflows > 0);
    totals.non_finite_vehicles += usize::from(!non_finite.is_empty());
    bump(&mut totals.unfed, unfed_reads.iter().chain(&unfed_system));
    bump(&mut totals.stubbed, &stub_names);
    bump(&mut totals.real, &real_names);
    bump(&mut totals.unconsumed_count, &unconsumed_outputs);
    let line = serde_json::json!({
        "vehicle": info.rel,
        "scripts": info.scripts,
        "load_errors": info.load_errors,
        "load_warnings": info.load_warnings,
        "unfed_inputs": unfed_reads,
        "unfed_system_values": unfed_system,
        "unconsumed_outputs": unconsumed_outputs,
        "callbacks_called": usage.external,
        "callbacks_real": real_names,
        "callbacks_stubbed": stub_names,
        "callbacks_unknown": unknown_names,
        "stack_underflows": underflows,
        "non_finite": non_finite,
        "ops_per_frame": ops_total as f64 / frames,
        "micros_per_frame": micros_total / frames,
        "runtime_messages": runtime.iter().filter(|m| !m.contains("neutral stub")).take(8).collect::<Vec<_>>(),
    });
    if let Some(o) = out.as_mut() {
        let _ = writeln!(o, "{line}");
    }
    write_or_check_golden(
        info.rel,
        &samples,
        args.golden.as_deref(),
        args.check.as_deref(),
    );
    if args.filter.is_some() {
        println!(
            "{}",
            serde_json::to_string_pretty(&line).unwrap_or_default()
        );
    }
}
