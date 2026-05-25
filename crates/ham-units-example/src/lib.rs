/// Converts megahertz to hertz.
pub fn mhz_to_hz(mhz: f64) -> f64 {
    mhz * 1_000_000.0
}

/// Returns the free-space wavelength, in meters, for a frequency in megahertz.
pub fn wavelength_meters(frequency_mhz: f64) -> f64 {
    const SPEED_OF_LIGHT_METERS_PER_SECOND: f64 = 299_792_458.0;

    SPEED_OF_LIGHT_METERS_PER_SECOND / mhz_to_hz(frequency_mhz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_mhz_to_hz() {
        assert_eq!(mhz_to_hz(14.074), 14_074_000.0);
    }

    #[test]
    fn calculates_wavelength() {
        let wavelength = wavelength_meters(14.074);

        assert!((wavelength - 21.302).abs() < 0.001);
    }
}
