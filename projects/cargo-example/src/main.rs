use ham_units_example::{mhz_to_hz, wavelength_meters};

fn main() {
    let frequency_mhz = 14.074;
    let frequency_hz = mhz_to_hz(frequency_mhz);
    let wavelength = wavelength_meters(frequency_mhz);

    println!("{frequency_mhz:.3} MHz = {frequency_hz:.0} Hz");
    println!("Free-space wavelength: {wavelength:.3} m");
}
