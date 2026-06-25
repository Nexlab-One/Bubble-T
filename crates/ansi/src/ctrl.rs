//! Device attribute queries and terminal identification sequences.

use crate::seq::dcs_st;

/// Requests the terminal name and version (XTVERSION).
pub const REQUEST_NAME_VERSION: &str = "\x1b[>q";

/// Alias for [`REQUEST_NAME_VERSION`].
pub const XTVERSION: &str = REQUEST_NAME_VERSION;

/// Requests primary device attributes (DA1).
pub const REQUEST_PRIMARY_DEVICE_ATTRIBUTES: &str = "\x1b[c";

/// Requests secondary device attributes (DA2).
pub const REQUEST_SECONDARY_DEVICE_ATTRIBUTES: &str = "\x1b[>c";

/// Requests tertiary device attributes (DA3).
pub const REQUEST_TERTIARY_DEVICE_ATTRIBUTES: &str = "\x1b[=c";

/// Builds a primary device attributes (DA1) response or request sequence.
pub fn primary_device_attributes(attrs: &[i32]) -> String {
    match attrs.len() {
        0 => REQUEST_PRIMARY_DEVICE_ATTRIBUTES.to_string(),
        1 if attrs[0] == 0 => "\x1b[0c".to_string(),
        _ => {
            let body: Vec<String> = attrs.iter().map(|a| a.to_string()).collect();
            format!("\x1b[?{}c", body.join(";"))
        }
    }
}

/// Alias for [`primary_device_attributes`].
pub fn da1(attrs: &[i32]) -> String {
    primary_device_attributes(attrs)
}

/// Builds a secondary device attributes (DA2) response or request sequence.
pub fn secondary_device_attributes(attrs: &[i32]) -> String {
    if attrs.is_empty() {
        return REQUEST_SECONDARY_DEVICE_ATTRIBUTES.to_string();
    }
    let body: Vec<String> = attrs.iter().map(|a| a.to_string()).collect();
    format!("\x1b[>{}c", body.join(";"))
}

/// Alias for [`secondary_device_attributes`].
pub fn da2(attrs: &[i32]) -> String {
    secondary_device_attributes(attrs)
}

/// Builds a tertiary device attributes (DA3) response or request sequence.
pub fn tertiary_device_attributes(unit_id: &str) -> String {
    match unit_id {
        "" => REQUEST_TERTIARY_DEVICE_ATTRIBUTES.to_string(),
        "0" => "\x1b[=0c".to_string(),
        id => dcs_st(&format!("!|{id}")),
    }
}

/// Alias for [`tertiary_device_attributes`].
pub fn da3(unit_id: &str) -> String {
    tertiary_device_attributes(unit_id)
}

/// Requests Termcap/Terminfo capability strings (XTGETTCAP).
pub fn xtgettcap(caps: &[&str]) -> String {
    if caps.is_empty() {
        return String::new();
    }
    let mut body = String::from("+q");
    for (i, cap) in caps.iter().enumerate() {
        if i > 0 {
            body.push(';');
        }
        for b in cap.bytes() {
            body.push_str(&format!("{b:02X}"));
        }
    }
    dcs_st(&body)
}

/// Alias for [`xtgettcap`].
pub fn request_termcap(caps: &[&str]) -> String {
    xtgettcap(caps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn da1_request() {
        assert_eq!(primary_device_attributes(&[]), "\x1b[c");
    }

    #[test]
    fn xtgettcap_rgb() {
        assert_eq!(xtgettcap(&["RGB"]), "\x1bP+q524742\x1b\\");
    }
}
