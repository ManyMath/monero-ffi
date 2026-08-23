use monero_rust::SeedFormat;

pub(crate) fn parse_seed_format(value: &str) -> Result<SeedFormat, String> {
    match value {
        "classic" | "legacy" => Ok(SeedFormat::Classic),
        "polyseed" => Ok(SeedFormat::Polyseed),
        "bip39" => Ok(SeedFormat::Bip39),
        _ => Err(format!("Unsupported seed format: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_compatibility_surface_and_rejects_unknown_formats() {
        assert_eq!(parse_seed_format("classic"), Ok(SeedFormat::Classic));
        assert_eq!(parse_seed_format("legacy"), Ok(SeedFormat::Classic));
        assert_eq!(parse_seed_format("polyseed"), Ok(SeedFormat::Polyseed));
        assert_eq!(parse_seed_format("bip39"), Ok(SeedFormat::Bip39));
        assert!(parse_seed_format("anything-else").is_err());
    }
}
