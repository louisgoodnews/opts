//! The log queries: the records, the tail and the level and text filters.

use super::*;

pub(super) fn records(list: Vec<LogRecord>) -> Value {
    let lines: Vec<Value> = list
        .into_iter()
        .map(|r| json!({ "ts": r.ts, "level": r.level, "target": r.target, "message": r.message }))
        .collect();
    json!({ "lines": lines })
}

pub(super) fn log_tail(_: &mut World, params: &Value) -> Outcome {
    match opt_u64(params, "n") {
        Ok(n) => Outcome::ok(records(opts_logging::recent_logs(
            n.unwrap_or(100) as usize,
            &LogQuery::default(),
        ))),
        Err(e) => Outcome::err(e),
    }
}

pub(super) fn parse_level(text: &str) -> Result<tracing::Level, RpcError> {
    text.parse::<tracing::Level>().map_err(|_| {
        RpcError::invalid_params("`level` must be one of trace, debug, info, warn, error")
    })
}

pub(super) fn log_filter(_: &mut World, params: &Value) -> Outcome {
    let query = (|| -> Result<(usize, LogQuery), RpcError> {
        let level = opt_str(params, "level")?.map(parse_level).transpose()?;
        Ok((
            opt_u64(params, "n")?.unwrap_or(100) as usize,
            LogQuery {
                target: opt_str(params, "target")?.map(str::to_string),
                min_level: level,
                contains: opt_str(params, "contains")?.map(str::to_string),
            },
        ))
    })();
    match query {
        Ok((n, q)) => Outcome::ok(records(opts_logging::recent_logs(n, &q))),
        Err(e) => Outcome::err(e),
    }
}
