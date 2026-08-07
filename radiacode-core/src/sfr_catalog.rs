pub fn sfr_supports_leds_on(text: &str) -> bool {
    text.lines().any(|line| {
        line.contains("VSFR_LEDS_ON") || line.contains("Addr=0x00000545")
    })
}

#[cfg(test)]
mod tests {
    use super::sfr_supports_leds_on;

    #[test]
    fn detects_leds_on_entry() {
        let sample = "[VSFR_LEDS_ON]\nAddr=0x00000545\nVirtual=1\nSize=1\n";
        assert!(sfr_supports_leds_on(sample));
    }

    #[test]
    fn absent_when_not_listed() {
        let sample = "[VSFR_VIBRO_ON]\nAddr=0x00000531\n";
        assert!(!sfr_supports_leds_on(sample));
    }
}
