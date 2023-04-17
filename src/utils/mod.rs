use std::io;

pub mod airframes;
pub mod arguments;
pub mod systable;

pub fn get_sampling_rate(freqs: &Vec<u16>, sampling_rates: &Vec<u32>) -> io::Result<u32> {
    let freq_diff =
        (*freqs.iter().max().unwrap_or(&0) as i32) - (*freqs.iter().min().unwrap_or(&0) as i32);
    if freq_diff < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid frequencies, no max exists: {:?}", freqs),
        ));
    }

    let mut sampling_rate: u32 = 0;
    for rate in sampling_rates {
        if (freq_diff as u32) * 1000 < ((*rate as f64) * 0.9) as u32 {
            sampling_rate = *rate;
            break;
        }
    }
    if sampling_rate == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Frequency difference {} is too big for sampling rates: {:?}",
                freq_diff * 1000,
                sampling_rates
            ),
        ));
    }

    Ok(sampling_rate)
}
