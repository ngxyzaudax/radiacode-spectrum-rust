pub fn model_from_serial(serial: &str) -> String {
    let mut parts = serial.split('-');
    let prefix = parts.next();
    let code = parts.next();
    match (prefix, code) {
        (Some(prefix), Some(code)) if !prefix.is_empty() && !code.is_empty() => {
            format!("{prefix}-{code}")
        }
        _ => "Unknown".into(),
    }
}

pub fn serial_from_advertisement(name: &str) -> Option<String> {
    let (prefix, serial) = name.split_once('#')?;
    if !prefix.eq_ignore_ascii_case("radiacode") {
        return None;
    }
    let serial = serial.trim();
    if serial.is_empty() {
        return None;
    }
    Some(serial.to_string())
}

pub fn model_from_advertisement(name: &str) -> Option<String> {
    serial_from_advertisement(name).map(|serial| model_from_serial(&serial))
}

#[cfg(test)]
mod tests {
    use super::{model_from_advertisement, model_from_serial, serial_from_advertisement};

    #[test]
    fn parses_rc110_model() {
        assert_eq!(model_from_serial("RC-110-006806"), "RC-110");
    }

    #[test]
    fn parses_rc10x_model() {
        assert_eq!(model_from_serial("RC-101-123456"), "RC-101");
    }

    #[test]
    fn parses_serial_from_advertisement() {
        assert_eq!(
            serial_from_advertisement("RadiaCode#RC-110-006806"),
            Some("RC-110-006806".into())
        );
    }

    #[test]
    fn parses_advertisement_case_insensitive() {
        assert_eq!(
            serial_from_advertisement("radiacode#RC-101-123456"),
            Some("RC-101-123456".into())
        );
    }

    #[test]
    fn ignores_advertisement_without_hash() {
        assert_eq!(serial_from_advertisement("RadiaCode"), None);
    }

    #[test]
    fn derives_model_from_advertisement() {
        assert_eq!(
            model_from_advertisement("RadiaCode#RC-110-006806"),
            Some("RC-110".into())
        );
    }
}
