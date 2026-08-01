/// Парсит "15m" / "1h" / "30s" / "90" (секунды) в секунды.
pub fn parse_interval(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("пустой интервал".to_owned());
    }
    let (num, mul) = if s.ends_with('h') {
        (&s[..s.len() - 1], 3600u64)
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], 60u64)
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], 1u64)
    } else {
        (s, 1u64)
    };
    let n: u64 = num
        .trim()
        .parse()
        .map_err(|_| format!("не число в интервале: {s}"))?;
    if n == 0 {
        return Err("интервал должен быть > 0".to_owned());
    }
    Ok(n * mul)
}