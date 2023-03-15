use chrono::Timelike;

pub fn parse_time(raw_time: &Vec<&str>) -> Option<(u8, u8)> {
    let h: u8 = raw_time[0].parse().unwrap_or(255);
    let m: u8 = raw_time[1].parse().unwrap_or(255);

    if h > 23 || m >= 60 {
        return None;
    }

    Some((h, m))
}

pub fn get_band(triggers: &Vec<(u8, u8, u32)>) -> Option<u32> {
    let current_time = chrono::offset::Local::now();

    for (h, m, band) in triggers.iter() {
        if current_time.hour() >= (*h as u32) && current_time.minute() >= (*m as u32) {
            return Some(*band);
        }
    }

    triggers.last().map(|x| x.2)
}
