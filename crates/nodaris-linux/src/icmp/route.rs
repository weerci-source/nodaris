use std::{net::Ipv4Addr, time::Duration};

use nodaris_core::models::icmp::{ProbeRoute, ProbeUnavailable};
use serde::Deserialize;

use super::process;

const ROUTE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Deserialize)]
struct RouteRow {
    dev: Option<String>,
    gateway: Option<Ipv4Addr>,
    prefsrc: Option<Ipv4Addr>,
    table: Option<String>,
    #[serde(default)]
    metric: u32,
    #[serde(rename = "type")]
    kind: Option<String>,
}

pub(super) async fn selected_gateway(interface: &str) -> Result<Ipv4Addr, ProbeUnavailable> {
    // ip omits the dev field when the command filters by device.
    let rows = route_rows(&["-j", "-4", "route", "show", "default"]).await?;
    choose_gateway(&rows, interface)
}

fn choose_gateway(rows: &[RouteRow], interface: &str) -> Result<Ipv4Addr, ProbeUnavailable> {
    let mut routes = rows
        .iter()
        .filter(|row| row.dev.as_deref() == Some(interface));
    let Some(best) = routes.clone().min_by_key(|row| row.metric) else {
        return Err(ProbeUnavailable::MissingGateway);
    };
    if routes.any(|row| row.metric == best.metric && row.gateway != best.gateway) {
        return Err(ProbeUnavailable::MissingGateway);
    }
    best.gateway.ok_or(ProbeUnavailable::MissingGateway)
}

pub(super) async fn lookup(target: Ipv4Addr) -> Result<ProbeRoute, ProbeUnavailable> {
    let rows = route_rows(&[
        "-j",
        "-4",
        "route",
        "get",
        &target.to_string(),
        "ipproto",
        "icmp",
    ])
    .await?;
    let [row] = rows.as_slice() else {
        return Err(ProbeUnavailable::RouteUnavailable);
    };
    if !matches!(row.kind.as_deref(), None | Some("unicast")) {
        return Err(ProbeUnavailable::RouteUnavailable);
    }

    Ok(ProbeRoute {
        interface: row.dev.clone().ok_or(ProbeUnavailable::RouteUnavailable)?,
        gateway: row.gateway,
        source: row.prefsrc,
        table: row.table.clone(),
    })
}

async fn route_rows(args: &[&str]) -> Result<Vec<RouteRow>, ProbeUnavailable> {
    let output = process::run("ip", args, ROUTE_TIMEOUT).await?;
    if !output.status.success() {
        return Err(if process::permission_error(&output) {
            ProbeUnavailable::PermissionDenied
        } else {
            ProbeUnavailable::RouteUnavailable
        });
    }

    serde_json::from_slice(&output.stdout).map_err(|_| ProbeUnavailable::InvalidOutput)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_uses_an_unambiguous_default_on_the_selected_interface() {
        let rows: Vec<RouteRow> = serde_json::from_str(
            r#"[
                {"dev":"eth0","gateway":"192.0.2.1","metric":10},
                {"dev":"wlan0","gateway":"192.0.2.2","metric":90},
                {"dev":"wlan0","gateway":"192.0.2.3","metric":80}
            ]"#,
        )
        .unwrap();
        assert_eq!(
            choose_gateway(&rows, "wlan0"),
            Ok(Ipv4Addr::new(192, 0, 2, 3))
        );
        assert_eq!(
            choose_gateway(&rows, "wlan1"),
            Err(ProbeUnavailable::MissingGateway)
        );
        let mut rows = rows;
        rows[1].metric = 80;
        assert_eq!(
            choose_gateway(&rows, "wlan0"),
            Err(ProbeUnavailable::MissingGateway)
        );
    }
}
