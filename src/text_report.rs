//! Plain-text report layout (box-drawing tables) — port of the former
//! `text-report.ts`. Callers pass already-formatted strings; this module owns
//! only layout. Widths are counted in code points.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    L,
    R,
}

pub struct TableSpec {
    pub title: Option<String>,
    pub columns: Vec<String>,
    pub aligns: Option<Vec<Align>>,
    pub rows: Vec<Vec<String>>,
    pub footer: Option<Vec<String>>,
    pub max_col_widths: Option<Vec<Option<usize>>>,
}

impl TableSpec {
    pub fn new(title: &str, columns: &[&str], rows: Vec<Vec<String>>) -> Self {
        TableSpec {
            title: if title.is_empty() { None } else { Some(title.to_string()) },
            columns: columns.iter().map(|s| s.to_string()).collect(),
            aligns: None,
            rows,
            footer: None,
            max_col_widths: None,
        }
    }
}

fn cell(s: &str) -> String {
    s.replace("\r\n", " / ").replace('\n', " / ").replace('\t', "  ")
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn pad(s: &str, w: usize, align: Align) -> String {
    let gap = w as isize - width(s) as isize;
    if gap <= 0 {
        return s.to_string();
    }
    let sp = " ".repeat(gap as usize);
    match align {
        Align::R => format!("{sp}{s}"),
        Align::L => format!("{s}{sp}"),
    }
}

fn cap(s: &str, max: Option<usize>) -> String {
    match max {
        Some(m) if m > 0 && width(s) > m => {
            let keep = (m as isize - 3).max(1) as usize;
            let mut out: String = s.chars().take(keep).collect();
            out.push_str("...");
            out
        }
        _ => s.to_string(),
    }
}

fn top_border(widths: &[usize], title: Option<&str>) -> String {
    let plain = format!("╭{}╮", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┬"));
    let Some(title) = title.filter(|t| !t.is_empty()) else { return plain };
    let inner: isize = widths.iter().map(|w| (*w + 3) as isize).sum::<isize>() - 1;
    let label = format!("─ {title} ");
    let rem = inner - width(&label) as isize;
    if rem < 1 {
        return plain;
    }
    format!("╭{label}{}╮", "─".repeat(rem as usize))
}

fn mid_border(widths: &[usize]) -> String {
    format!("├{}┤", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┼"))
}

fn bottom_border(widths: &[usize]) -> String {
    format!("╰{}╯", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("┴"))
}

pub fn box_table(spec: &TableSpec) -> String {
    let n = spec.columns.len();
    let aligns: Vec<Align> = spec.aligns.clone().unwrap_or_else(|| vec![Align::L; n]);
    let norm = |r: &Vec<String>| -> Vec<String> {
        let mut out: Vec<String> = r.iter().map(|c| cell(c)).collect();
        while out.len() < n {
            out.push(String::new());
        }
        out.truncate(n);
        out
    };
    let header: Vec<String> = spec.columns.iter().map(|c| cell(c)).collect();
    let mut all: Vec<Vec<String>> = vec![header.clone()];
    all.extend(spec.rows.iter().map(norm));
    if let Some(f) = &spec.footer {
        all.push(norm(f));
    }
    let capw = |i: usize| -> Option<usize> { spec.max_col_widths.as_ref().and_then(|m| m.get(i).copied().flatten()) };
    let mut widths: Vec<usize> = (0..n)
        .map(|i| all.iter().map(|c| width(&cap(&c[i], capw(i)))).max().unwrap_or(1).max(1))
        .collect();
    if let Some(title) = &spec.title {
        let need = width(&format!("─ {} ", cell(title))) + 1;
        let total: usize = widths.iter().map(|w| w + 3).sum::<usize>() - 1;
        if total < need {
            *widths.last_mut().unwrap() += need - total;
        }
    }
    let line = |r: &Vec<String>| -> String {
        let cells: Vec<String> = r.iter().enumerate().map(|(i, v)| pad(&cap(v, capw(i)), widths[i], aligns[i])).collect();
        format!("│ {} │", cells.join(" │ "))
    };
    let mut out = vec![top_border(&widths, spec.title.as_deref()), line(&header), mid_border(&widths)];
    for r in &spec.rows {
        out.push(line(&norm(r)));
    }
    if let Some(f) = &spec.footer {
        out.push(mid_border(&widths));
        out.push(line(&norm(f)));
    }
    out.push(bottom_border(&widths));
    out.join("\n")
}

pub fn kv_table(title: &str, pairs: &[(String, String)]) -> String {
    kv_table_w(title, pairs, 34, 64)
}

pub fn kv_table_w(title: &str, pairs: &[(String, String)], max_label: usize, max_value: usize) -> String {
    let rows: Vec<(String, String)> = pairs.iter().map(|(k, v)| (cell(k), cell(v))).collect();
    let kw = max_label.min(rows.iter().map(|r| width(&r.0)).max().unwrap_or(1).max(1));
    let vw = max_value.min(rows.iter().map(|r| width(&r.1)).max().unwrap_or(1).max(1));
    let inner = kw + vw + 5;
    let label = format!("─ {title} ");
    let top = format!("╭{label}{}╮", "─".repeat((inner as isize - width(&label) as isize).max(1) as usize));
    let bottom = format!("╰{}╯", "─".repeat(inner));
    let mut out = vec![top];
    for (k, v) in &rows {
        out.push(format!("│ {} {} │", pad(k, kw, Align::L), pad(v, vw, Align::R)));
    }
    out.push(bottom);
    out.join("\n")
}

pub fn banner(lines: &[String]) -> String {
    let w = lines.iter().map(|l| width(l) + 4).max().unwrap_or(0).max(78);
    let rule = "━".repeat(w);
    let body = lines.iter().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n");
    format!("{rule}\n{body}\n{rule}")
}

pub fn assemble(banner: &str, blocks: &[String], footer: &[String]) -> String {
    let mut parts: Vec<String> = vec![banner.trim_end().to_string()];
    parts.extend(blocks.iter().filter(|b| !b.trim().is_empty()).map(|b| b.trim_end().to_string()));
    let out = parts.join("\n\n");
    if footer.is_empty() { out } else { format!("{out}\n\n{}", footer.join("\n")) }
}
