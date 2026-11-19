mod ping;
mod process;
mod route;

use std::{
    net::Ipv4Addr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use nodaris_core::models::icmp::{
    IcmpAttempt, IcmpOutcome, IcmpReport, IcmpSeries, IcmpStatistics, ProbeRoute, ProbeUnavailable,
};
use nodaris_core::providers::network_probe::ProbeMethod;

static PROBE_SLOT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
const PROBE_COUNT: usize = 3;
const SAMPLE_GAP: Duration = Duration::from_millis(250);

/// Collects the two series in one window. Only one run per process is permitted.
pub async fn collect_icmp_report(interface: String, external: Ipv4Addr) -> Option<IcmpReport> {
    // Use a Tokio permit so cancellation releases the slot without poisoning it.
    // The guard covers route lookups as well as ping subprocesses.
    let _permit = PROBE_SLOT.try_acquire().ok()?;
    let started = Instant::now();
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64);
    let gateway = collect_series(
        started,
        route::selected_gateway(&interface).await,
        Some(&interface),
    )
    .await;
    let external = collect_series(started, Ok(external), None).await;
    Some(IcmpReport {
        method: ProbeMethod::Icmp,
        interface,
        started_at_unix_ms,
        gateway,
        external,
    })
}

async fn collect_series(
    started: Instant,
    target: Result<Ipv4Addr, ProbeUnavailable>,
    required_interface: Option<&str>,
) -> IcmpSeries {
    let mut series = IcmpSeries {
        target: target.ok(),
        route: None,
        started_offset_ms: elapsed_ms(started),
        finished_offset_ms: 0,
        attempts: Vec::with_capacity(PROBE_COUNT),
        statistics: None,
        unavailable_reason: None,
    };

    match target {
        Ok(target) => match sample(started, target, required_interface, &mut series).await {
            Ok(()) => series.statistics = IcmpStatistics::from_attempts(&series.attempts),
            Err(reason) => series.unavailable_reason = Some(reason),
        },
        Err(reason) => series.unavailable_reason = Some(reason),
    }
    series.finished_offset_ms = elapsed_ms(started);
    series
}

async fn sample(
    started: Instant,
    target: Ipv4Addr,
    required_interface: Option<&str>,
    series: &mut IcmpSeries,
) -> Result<(), ProbeUnavailable> {
    let route = series.route.insert(route::lookup(target).await?);
    check_interface(route, required_interface)?;

    for index in 0..PROBE_COUNT {
        if index > 0 {
            tokio::time::sleep(SAMPLE_GAP).await;
            check_route(&route::lookup(target).await?, route)?;
        }
        let started_offset_ms = elapsed_ms(started);
        let outcome = ping::probe(target).await;
        let unavailable = match outcome {
            IcmpOutcome::Unavailable { reason } => Some(reason),
            _ => None,
        };
        series.attempts.push(IcmpAttempt {
            started_offset_ms,
            finished_offset_ms: elapsed_ms(started),
            outcome,
        });
        if let Some(reason) = unavailable {
            return Err(reason);
        }
        check_route(&route::lookup(target).await?, route)?;
    }
    if let Some(interface) = required_interface {
        match route::selected_gateway(interface).await {
            Ok(gateway) if gateway == target => {}
            Ok(_) | Err(ProbeUnavailable::MissingGateway) => {
                return Err(ProbeUnavailable::RouteChanged);
            }
            Err(reason) => return Err(reason),
        }
    }
    Ok(())
}

fn check_interface(route: &ProbeRoute, required: Option<&str>) -> Result<(), ProbeUnavailable> {
    match required {
        Some(interface) if route.interface != interface => Err(ProbeUnavailable::RouteMismatch),
        _ => Ok(()),
    }
}

fn check_route(current: &ProbeRoute, initial: &ProbeRoute) -> Result<(), ProbeUnavailable> {
    if current != initial {
        return Err(ProbeUnavailable::RouteChanged);
    }
    Ok(())
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_vpn_route_is_accepted_but_cannot_be_attributed_to_wifi() {
        let route = ProbeRoute {
            interface: "wg0".into(),
            gateway: None,
            source: None,
            table: Some("51820".into()),
        };
        assert_eq!(check_interface(&route, None), Ok(()));
        assert_eq!(
            check_interface(&route, Some("wlan0")),
            Err(ProbeUnavailable::RouteMismatch)
        );
    }

    #[test]
    fn route_switch_is_rejected_even_if_the_interface_matches() {
        let initial = ProbeRoute {
            interface: "wlan0".into(),
            gateway: Some(Ipv4Addr::new(192, 0, 2, 1)),
            source: None,
            table: None,
        };
        let changed = ProbeRoute {
            gateway: Some(Ipv4Addr::new(192, 0, 2, 2)),
            interface: "wlan0".into(),
            source: None,
            table: None,
        };
        assert_eq!(
            check_route(&changed, &initial),
            Err(ProbeUnavailable::RouteChanged)
        );
    }
}
