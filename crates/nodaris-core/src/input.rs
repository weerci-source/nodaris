use std::net::Ipv4Addr;

pub fn valid_interface_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 15
        && !name.starts_with('-')
        && !matches!(name, "." | "..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub fn external_unicast(address: Ipv4Addr) -> bool {
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
