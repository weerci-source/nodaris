use std::{
    fs,
    io::{self, ErrorKind},
    net::Ipv4Addr,
    path::Path,
};

use nodaris_core::{
    error::CoreError,
    input::{external_unicast, valid_interface_name},
    keys::diagnostics,
    models::{
        ethernet::{EthernetLink, EthernetLinkState, EthernetReport},
        measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    },
};

use crate::{collect_icmp_report, session::RunClock};

const SYSFS_NET: &str = "/sys/class/net";
const ARPHRD_ETHER: &str = "1";
const IFF_UP: u32 = 0x1;

pub fn list_ethernet_links() -> Result<Vec<EthernetLink>, CoreError> {
    read_links(Path::new(SYSFS_NET)).map_err(read_error)
}

pub async fn collect_ethernet_report(
    interface: String,
    target: String,
) -> Result<EthernetReport, CoreError> {
    if !valid_interface_name(&interface) {
        return Err(CoreError::validation(
            diagnostics::ETHERNET_INTERFACE_UNAVAILABLE,
        ));
    }
    let target = target
        .parse::<Ipv4Addr>()
        .ok()
        .filter(|address| external_unicast(*address))
        .ok_or_else(|| CoreError::validation(diagnostics::ETHERNET_INVALID_TARGET))?;

    let link = tokio::task::spawn_blocking(move || {
        let path = Path::new(SYSFS_NET).join(&interface);
        if !is_ethernet(&path).map_err(read_error)? {
            return Err(CoreError::validation(
                diagnostics::ETHERNET_INTERFACE_UNAVAILABLE,
            ));
        }
        Ok(read_link(&path, interface, &RunClock::new()))
    })
    .await
    .map_err(|error| CoreError::internal(format!("Ethernet link worker: {error}")))??;
    let probes = collect_icmp_report(link.interface.clone(), target)
        .await
        .ok_or_else(|| CoreError::network_err(diagnostics::PROBE_BUSY))?;
    Ok(EthernetReport { link, probes })
}

fn read_links(root: &Path) -> io::Result<Vec<EthernetLink>> {
    let clock = RunClock::new();
    let mut links = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| valid_interface_name(name)) else {
            continue;
        };
        let path = entry.path();
        if is_ethernet(&path)? {
            links.push(read_link(&path, name.to_owned(), &clock));
        }
    }
    links.sort_by(|left, right| left.interface.cmp(&right.interface));
    Ok(links)
}

fn is_ethernet(path: &Path) -> io::Result<bool> {
    if !path.join("device").try_exists()?
        || path.join("wireless").try_exists()?
        || path.join("phy80211").try_exists()?
    {
        return Ok(false);
    }
    // A netdev can disappear between directory enumeration and attribute reads.
    let attributes: io::Result<(String, String)> = (|| {
        Ok((
            fs::read_to_string(path.join("type"))?,
            fs::read_to_string(path.join("uevent"))?,
        ))
    })();
    match attributes {
        Ok((kind, event)) => Ok(kind.trim() == ARPHRD_ETHER
            && !event
                .lines()
                .any(|line| matches!(line, "DEVTYPE=wlan" | "DEVTYPE=wwan"))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn read_error(error: io::Error) -> CoreError {
    let public = if error.kind() == ErrorKind::PermissionDenied {
        CoreError::permission_err(diagnostics::ETHERNET_READ_FAILED)
    } else {
        CoreError::hardware_err(diagnostics::ETHERNET_READ_FAILED)
    };
    public.with_tech(format!("Ethernet sysfs: {error}"))
}

fn read_link(path: &Path, interface: String, clock: &RunClock) -> EthernetLink {
    use EthernetLinkState::Up;

    let initial_state = link_state(path);
    let (state, speed_mbps) = if initial_state == Ok(Up) {
        let speed = read_metric(path, "speed", parse_speed);
        let observed_at = clock.now();
        let state = link_state(path);
        let speed = if state == Ok(Up) {
            speed
        } else {
            Err(UnavailableReason::ContextChanged)
        };
        (state, measurement(speed, "linux.sysfs.speed", observed_at))
    } else {
        let reason = initial_state
            .err()
            .unwrap_or(UnavailableReason::NotMeasured);
        (
            initial_state,
            measurement(Err(reason), "linux.sysfs.speed", clock.now()),
        )
    };
    EthernetLink {
        interface,
        state: measurement(state, "linux.sysfs", clock.now()),
        speed_mbps,
    }
}

fn link_state(path: &Path) -> Result<EthernetLinkState, UnavailableReason> {
    let flags = read_metric(path, "flags", |value| {
        u32::from_str_radix(value.strip_prefix("0x")?, 16).ok()
    })?;
    if flags & IFF_UP == 0 {
        return Ok(EthernetLinkState::Disabled);
    }
    read_metric(path, "carrier", |value| match value {
        "1" => Some(EthernetLinkState::Up),
        "0" => Some(EthernetLinkState::Down),
        _ => None,
    })
}

fn parse_speed(value: &str) -> Option<u32> {
    // The driver can report SPEED_UNKNOWN (-1) instead of a negotiated rate.
    value
        .parse::<u32>()
        .ok()
        .filter(|speed| (1..u32::MAX).contains(speed))
}

fn read_metric<T>(
    path: &Path,
    attribute: &str,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<T, UnavailableReason> {
    let value = fs::read_to_string(path.join(attribute)).map_err(|error| match error.kind() {
        ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
        ErrorKind::Unsupported | ErrorKind::InvalidInput => UnavailableReason::DriverUnavailable,
        ErrorKind::NotFound => UnavailableReason::HardwareUnavailable,
        _ => UnavailableReason::TemporarilyUnavailable,
    })?;
    parse(value.trim()).ok_or(UnavailableReason::InvalidMeasurement)
}

fn measurement<T>(
    value: Result<T, UnavailableReason>,
    source: &str,
    observed_at: ObservationTime,
) -> Measurement<T> {
    let source = SourceId(source.to_owned());
    match value {
        Ok(value) => Measurement::available(value, source, observed_at),
        Err(reason) => Measurement::unavailable(reason, source, observed_at),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use nodaris_core::models::measurement::MeasurementState;

    struct NetFixture(PathBuf);

    impl NetFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(RunClock::new().now().session_id.0);
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn add_interface(&self, name: &str, physical: bool, kind: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::create_dir(&path).unwrap();
            if physical {
                fs::create_dir(path.join("device")).unwrap();
            }
            fs::write(path.join("type"), kind).unwrap();
            fs::write(path.join("uevent"), "").unwrap();
            path
        }
    }

    impl Drop for NetFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn picks_hardware_backed_ethernet_even_without_a_link() {
        let fixture = NetFixture::new();
        let ethernet = fixture.add_interface("lan0", true, "1\n");
        fs::write(ethernet.join("flags"), "0x1003\n").unwrap();
        fs::write(ethernet.join("carrier"), "0\n").unwrap();
        fs::write(ethernet.join("speed"), "1000\n").unwrap();
        let wifi = fixture.add_interface("wlan0", true, "1\n");
        fs::create_dir(wifi.join("phy80211")).unwrap();
        let modem = fixture.add_interface("wwan0", true, "1\n");
        fs::write(modem.join("uevent"), "DEVTYPE=wwan\n").unwrap();
        fixture.add_interface("virbr0", false, "1\n");
        fixture.add_interface("lo", false, "772\n");

        let links = read_links(&fixture.0).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].interface, "lan0");
        assert_eq!(links[0].state.value(), Some(&EthernetLinkState::Down));
        assert_eq!(
            links[0].speed_mbps.state,
            MeasurementState::Unavailable(UnavailableReason::NotMeasured)
        );
        assert!(read_links(&fixture.0.join("missing")).is_err());
    }

    #[test]
    fn reports_negotiated_speed_and_distinguishes_unknown_speed_and_disabled_link() {
        let fixture = NetFixture::new();
        let interface = fixture.add_interface("eth0", true, "1\n");
        fs::write(interface.join("flags"), "0x1003\n").unwrap();
        fs::write(interface.join("carrier"), "1\n").unwrap();
        fs::write(interface.join("speed"), "100\n").unwrap();
        let clock = RunClock::new();

        let link = read_link(&interface, "eth0".into(), &clock);
        assert_eq!(link.state.value(), Some(&EthernetLinkState::Up));
        assert_eq!(link.speed_mbps.value(), Some(&100));

        for unavailable in ["-1\n", "0\n", "4294967295\n"] {
            fs::write(interface.join("speed"), unavailable).unwrap();
            let link = read_link(&interface, "eth0".into(), &clock);
            assert_eq!(link.state.value(), Some(&EthernetLinkState::Up));
            assert!(link.speed_mbps.value().is_none());
        }

        fs::write(interface.join("flags"), "0x1002\n").unwrap();
        let link = read_link(&interface, "eth0".into(), &clock);
        assert_eq!(link.state.value(), Some(&EthernetLinkState::Disabled));
        assert!(link.speed_mbps.value().is_none());
    }
}
