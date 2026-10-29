//! Tickets, change coins, and passenger callbacks.

use super::*;

pub(super) fn handle_service_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        "getticketname" => {
            let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
            let name = usize::try_from(idx)
                .ok()
                .and_then(|i| ctx.tickets.tickets.get(i))
                .map_or_else(
                    || "Normalfahrschein".to_string(),
                    |t| t.display.replace('@', " "),
                );
            st.strings.push(name);
            Some(true)
        }
        "getticketvalue" => {
            let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
            let price = usize::try_from(idx)
                .ok()
                .and_then(|i| ctx.tickets.price(i))
                .unwrap_or(2.50);
            st.numbers.push(price);
            Some(true)
        }
        "givechangecoin" => {
            let idx = st.numbers.pop().unwrap_or(0.0).round() as i64;
            let value = usize::try_from(idx)
                .ok()
                .and_then(|i| ctx.coins.get(i))
                .copied();
            if let (Some(v), Ok(mut log)) = (value, ctx.change_given.write()) {
                if log.len() < 64 {
                    log.push(v);
                }
            }
            Some(true)
        }
        "gethumancountonseat" => {
            let seat = st.numbers.pop().unwrap_or(0.0).round() as i64;
            let n = usize::try_from(seat).ok().and_then(|i| {
                ctx.passengers
                    .read()
                    .ok()
                    .and_then(|c| c.on_place.get(i).copied())
            });
            st.numbers.push(n.unwrap_or(0) as f32);
            Some(true)
        }
        "gethumancountonpathlink" => {
            let link = st.numbers.pop().unwrap_or(0.0).round() as i64;
            let n = usize::try_from(link).ok().and_then(|i| {
                ctx.passengers
                    .read()
                    .ok()
                    .and_then(|c| c.on_link.get(i).copied())
            });
            st.numbers.push(n.unwrap_or(0) as f32);
            Some(true)
        }
        "getheightabovepoint" => {
            let _z = st.numbers.pop();
            let _y = st.numbers.pop();
            let _x = st.numbers.pop();
            st.numbers.push(0.0);
            Some(true)
        }
        _ => None,
    }
}
