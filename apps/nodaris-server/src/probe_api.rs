use std::net::Ipv4Addr;

use axum::{
    extract::rejection::JsonRejection,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use nodaris_core::{error::CoreError, keys::diagnostics};
use serde::Deserialize;

use super::radio_api::{error_response, valid_interface_name};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProbeRequest {
    interface: String,
    target: Ipv4Addr,
}

pub(super) async fn probe_handler(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    request: Result<Json<ProbeRequest>, JsonRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return error_response(
            StatusCode::FORBIDDEN,
            CoreError::permission_err(diagnostics::PROBE_FORBIDDEN),
        );
    }
    let Ok(Json(request)) = request else {
        return invalid_request();
    };
    if !valid_interface_name(&request.interface) || !external_unicast(request.target) {
        return invalid_request();
    }
    match nodaris_linux::collect_icmp_report(request.interface, request.target).await {
        Some(report) => ([(header::CACHE_CONTROL, "no-store")], Json(report)).into_response(),
        None => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            CoreError::network_err(diagnostics::PROBE_BUSY),
        ),
    }
}

fn invalid_request() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        CoreError::validation(diagnostics::PROBE_INVALID_REQUEST),
    )
}

fn local_request(headers: &HeaderMap, port: u16) -> bool {
    // Vite forwards the local dev origin with the upstream loopback Host.
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let Some(host) = host.filter(|host| local_authority(host, port)) else {
        return false;
    };
    match headers.get(header::ORIGIN) {
        None => true,
        Some(origin) => origin
            .to_str()
            .ok()
            .and_then(|origin| origin.strip_prefix("http://"))
            .is_some_and(|origin| origin == host || local_authority(origin, 5173)),
    }
}

fn local_authority(authority: &str, port: u16) -> bool {
    let Some((host, port_text)) = authority.rsplit_once(':') else {
        return port == 80 && matches!(authority, "localhost" | "127.0.0.1");
    };
    matches!(host, "localhost" | "127.0.0.1") && port_text.parse() == Ok(port)
}

fn external_unicast(address: Ipv4Addr) -> bool {
    !address.is_private()
        && !address.is_loopback()
        && !address.is_link_local()
        && !address.is_documentation()
        && !matches!(
            address.octets(),
            [0 | 224..=255, ..]
                | [100, 64..=127, ..]
                | [192, 0, 0, _]
                | [192, 88, 99, _]
                | [198, 18 | 19, ..]
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_local_browser_and_rejects_foreign_origins_and_hosts() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "127.0.0.1:8080".parse().unwrap());
        assert!(local_request(&headers, 8080));
        assert!(!local_request(&headers, 8081));
        headers.insert(header::ORIGIN, "http://localhost:5173".parse().unwrap());
        assert!(local_request(&headers, 8080));
        headers.insert(header::ORIGIN, "http://other.test".parse().unwrap());
        assert!(!local_request(&headers, 8080));
        headers.remove(header::ORIGIN);
        headers.insert(header::HOST, "other.test:8080".parse().unwrap());
        assert!(!local_request(&headers, 8080));
    }

    #[test]
    fn validates_public_ipv4_targets() {
        assert!(external_unicast("1.1.1.1".parse().unwrap()));
        for address in [
            "127.1.2.3",
            "10.2.3.4",
            "169.254.0.1",
            "100.127.0.1",
            "198.19.0.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(!external_unicast(address.parse().unwrap()), "{address}");
        }
    }
}
