fn iso_to_unix_ms(iso: &str) -> Option<u64> {
    let parts: Vec<&str> = iso.split(|c| c == 'T' || c == 'Z' || c == '-' || c == ':' || c == '.').collect();
    if parts.len() < 6 { return None; }
    let y: u64 = parts[0].parse().ok()?;
    let m: u64 = parts[1].parse().ok()?;
    let d: u64 = parts[2].parse().ok()?;
    let h: u64 = parts[3].parse().ok()?;
    let min: u64 = parts[4].parse().ok()?;
    let s: u64 = parts[5].parse().ok()?;
    let ms: u64 = if parts.len() > 6 && !parts[6].is_empty() {
        let ms_str = &parts[6][0..std::cmp::min(3, parts[6].len())];
        let mut val: u64 = ms_str.parse().ok()?;
        if ms_str.len() == 1 { val *= 100; }
        else if ms_str.len() == 2 { val *= 10; }
        val
    } else { 0 };

    let mut days = 0;
    for year in 1970..y {
        days += if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 366 } else { 365 };
    }
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    for month in 1..m {
        days += month_days[(month - 1) as usize];
        if month == 2 && is_leap { days += 1; }
    }
    days += d - 1;

    let total_s = days * 86400 + h * 3600 + min * 60 + s;
    Some(total_s * 1000 + ms)
}
fn main() {
    println!("{}", iso_to_unix_ms("2026-10-06T15:53:23.627982000Z").unwrap());
}
