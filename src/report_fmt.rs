//! JS-compatible number formatting + regime display tables for the
//! server-rendered report text (findings, notes, the plain-text report).
//!
//! The report used to be formatted in the browser; these helpers reproduce
//! `Number.prototype.toFixed`, `Math.round` and `toLocaleString('en-US')`
//! exactly (notably: JS rounds exact ties half-up, Rust's `{:.N}` rounds them
//! half-to-even), so server text matches what the GUI shows.

/// `Math.round`: ties round toward +infinity.
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `Number.prototype.toFixed(digits)`.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let neg = x < 0.0;
    let ax = x.abs();
    // Exact-enough decimal expansion: 30 extra digits is far beyond the point
    // where a non-tie can look like a tie for any f64 magnitude we format.
    let s = format!("{:.*}", digits + 30, ax);
    let (int_part, frac_part) = s.split_once('.').unwrap_or((s.as_str(), ""));
    let keep: Vec<u8> = int_part.bytes().chain(frac_part.bytes().take(digits)).collect();
    let rest = &frac_part.as_bytes()[digits.min(frac_part.len())..];
    // round half up on the exact expansion
    let round_up = !rest.is_empty() && rest[0] >= b'5';
    let mut digits_v = keep;
    if round_up {
        let mut i = digits_v.len();
        loop {
            if i == 0 {
                digits_v.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits_v[i] == b'9' {
                digits_v[i] = b'0';
            } else {
                digits_v[i] += 1;
                break;
            }
        }
    }
    let int_len = digits_v.len() - digits;
    let int_s = String::from_utf8(digits_v[..int_len].to_vec()).unwrap();
    let frac_s = String::from_utf8(digits_v[int_len..].to_vec()).unwrap();
    let mut out = if digits > 0 { format!("{int_s}.{frac_s}") } else { int_s };
    if neg {
        out.insert(0, '-');
    }
    out
}

fn group_thousands(s: &str) -> String {
    let (sign, digits) = if let Some(r) = s.strip_prefix('-') { ("-", r) } else { ("", s) };
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

pub fn fmt_int(v: Option<f64>) -> String {
    match v {
        None => "—".into(),
        Some(x) => group_thousands(&format!("{}", js_round(x) as i64)),
    }
}

pub fn fmt1(v: f64) -> String {
    if v >= 100.0 { to_fixed(v, 0) } else { to_fixed(v, 1) }
}

pub fn fmt1n(v: Option<f64>) -> String {
    v.map(fmt1).unwrap_or_else(|| "—".into())
}

pub fn fmt_ms(ms: Option<f64>) -> String {
    match ms {
        None => "—".into(),
        Some(m) if m < 1000.0 => format!("{}ms", js_round(m) as i64),
        Some(m) => format!("{}s", to_fixed(m / 1000.0, 2)),
    }
}

pub fn fmt_ms_prec(ms: Option<f64>) -> String {
    match ms {
        None => "—".into(),
        Some(m) if m < 1.0 => format!("{}ms", to_fixed(m, 2)),
        Some(m) if m < 100.0 => format!("{}ms", to_fixed(m, 1)),
        Some(m) if m < 1000.0 => format!("{}ms", js_round(m) as i64),
        Some(m) => format!("{}s", to_fixed(m / 1000.0, 2)),
    }
}

pub fn fmt_s(ms: Option<f64>) -> String {
    ms.map(|m| format!("{}s", to_fixed(m / 1000.0, 2))).unwrap_or_else(|| "—".into())
}

pub fn fmt_pct(v: Option<f64>, digits: usize) -> String {
    v.map(|x| format!("{}%", to_fixed(x, digits))).unwrap_or_else(|| "—".into())
}

pub fn fmt_dur(ms: f64) -> String {
    if ms == 0.0 || ms.is_nan() {
        return "0.0s".into();
    }
    let s = ms / 1000.0;
    if s < 60.0 {
        return format!("{}s", to_fixed(s, 1));
    }
    let m = (s / 60.0).floor();
    format!("{}m {}s", m as i64, to_fixed(s - m * 60.0, 1))
}

// ---------------------------------------------------------------- regimes

/// (key, label, color, initial) in canonical order.
pub const REGIMES: [(&str, &str, &str); 7] = [
    ("code", "Code", "#22D3EE"),
    ("math", "Math", "#F6C84C"),
    ("structured", "Structured", "#39D98A"),
    ("prose", "Prose", "#7E8BA3"),
    ("reasoning_prose", "Reasoning prose", "#A78BFA"),
    ("creative_prose", "Creative prose", "#FF7AA8"),
    ("other_prose", "Other prose", "#AAB6C8"),
];

const FALLBACK_COLOR: &str = "#8CA3C3";

fn alias(key: &str) -> Option<&'static str> {
    Some(match key {
        "chat" => "other_prose",
        "json" | "table" | "list" => "structured",
        "reasoning" => "reasoning_prose",
        "mixed" | "other" | "unknown" => "other_prose",
        _ => return None,
    })
}

fn resolve(key: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    REGIMES
        .iter()
        .find(|r| r.0 == key)
        .or_else(|| alias(key).and_then(|a| REGIMES.iter().find(|r| r.0 == a)))
}

pub fn regime_label(key: &str) -> String {
    resolve(key).map(|r| r.1.to_string()).unwrap_or_else(|| key.to_string())
}

pub fn regime_color(key: &str) -> String {
    resolve(key).map(|r| r.2.to_string()).unwrap_or_else(|| FALLBACK_COLOR.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_js() {
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(0.5, 0), "1");
        assert_eq!(to_fixed(1.005, 2), "1.00"); // exact value is below the tie
        assert_eq!(to_fixed(1.45, 1), "1.4"); // 1.45 is 1.4499999999999999556
        assert_eq!(to_fixed(0.125, 2), "0.13"); // exact tie rounds up in JS
        assert_eq!(to_fixed(99.95, 1), "100.0");
        assert_eq!(to_fixed(-0.0001, 2), "-0.00");
        assert_eq!(to_fixed(1234.5678, 1), "1234.6");
        assert_eq!(to_fixed(9.999, 2), "10.00");
        assert_eq!(to_fixed(0.0, 1), "0.0");
    }

    #[test]
    fn int_and_dur_formatting() {
        assert_eq!(fmt_int(Some(12974.4)), "12,974");
        assert_eq!(fmt_int(Some(999.5)), "1,000");
        assert_eq!(fmt_int(None), "—");
        assert_eq!(fmt_dur(75_164.0), "1m 15.2s");
        assert_eq!(fmt_dur(1500.0), "1.5s");
        assert_eq!(fmt_ms(Some(999.5)), "1000ms");
        assert_eq!(fmt_ms(Some(1500.0)), "1.50s");
        assert_eq!(fmt1(123.456), "123");
        assert_eq!(fmt1(12.34), "12.3");
    }
}
