//! Privacy policy adapted from National Design Studio Rampart 0.1.3 (CC BY 4.0).
//! See ../ATTRIBUTION.md. All offsets are UTF-8 bytes, never UTF-16 indices.
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub label: String,
    pub score: f32,
}
fn span(start: usize, end: usize, label: &str, score: f32) -> Span {
    Span {
        start,
        end,
        label: label.into(),
        score,
    }
}
fn valid(text: &str, s: &Span) -> bool {
    s.start < s.end
        && s.end <= text.len()
        && text.is_char_boundary(s.start)
        && text.is_char_boundary(s.end)
        && s.score.is_finite()
}
struct Patterns {
    digit: Regex,
    text: Vec<(&'static str, Regex)>,
    ipv6: Regex,
    ipv6_candidate: Regex,
    connector: Regex,
    left: Regex,
    right: Regex,
    extra: Vec<(Regex, &'static str)>,
}
fn patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        // Every expression is fixed source, never input. Compilation failures are programmer errors.
        let re = |s: &str| Regex::new(s).expect("fixed Rampart policy regex");
        Patterns {
            digit: re(r"[0-9](?:[ .-]?[0-9])*"),
            text: vec![
                ("EMAIL", re(r"(?-u:\b)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?-u:\b)")),
                ("URL", re(r#"(?-u:\b)https?://[^\s<>"'\])}]+"#)),
                ("URL", re(r#"(?-u:\b)www\.[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?:/[^\s<>"'\])}]*)?"#)),
                ("IP_ADDRESS", re(r"(?-u:\b)(?:(?:25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])(?-u:\b)")),
                ("IP_ADDRESS", re(r"(?-u:\b)(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}(?-u:\b)")),
            ],
            ipv6_candidate: re(r"[0-9A-Fa-f:]+"),
            ipv6: re(r"^(?:(?:[0-9A-Fa-f]{1,4}:){7}[0-9A-Fa-f]{1,4}|(?:[0-9A-Fa-f]{1,4}:){1,7}:|(?:[0-9A-Fa-f]{1,4}:){1,6}:[0-9A-Fa-f]{1,4}|(?:[0-9A-Fa-f]{1,4}:){1,5}(?::[0-9A-Fa-f]{1,4}){1,2}|(?:[0-9A-Fa-f]{1,4}:){1,4}(?::[0-9A-Fa-f]{1,4}){1,3}|(?:[0-9A-Fa-f]{1,4}:){1,3}(?::[0-9A-Fa-f]{1,4}){1,4}|(?:[0-9A-Fa-f]{1,4}:){1,2}(?::[0-9A-Fa-f]{1,4}){1,5}|[0-9A-Fa-f]{1,4}:(?::[0-9A-Fa-f]{1,4}){1,6}|::(?:[0-9A-Fa-f]{1,4}:){0,6}[0-9A-Fa-f]{1,4})$"),
            connector: re(r"^[\s'\u{2019}.-]*$"),
            left: re(r"([\p{Lu}][\p{L}\p{M}\u{2019}']{0,3})([\s'\u{2019}.-]{1,3})$"),
            right: re(r"^([\s'\u{2019}.-]{1,3})([\p{Lu}][\p{L}\p{M}\u{2019}']{0,3})"),
            extra: vec![
                (re(r"-----BEGIN [^-\r\n]*PRIVATE KEY-----[\s\S]*?-----END [^-\r\n]*PRIVATE KEY-----"), "[SECRET]"),
                (re(r#"(?i)(?-u:\b)(?:Bearer\s+[^\s,;]+|(?:password|passwd|secret|token|api[_-]?key|authorization)\s*[:=]\s*(?:"[^"]*"|'[^']*'|[^\s,;]+))"#), "[SECRET]"),
                (re(r"(?-u:\b)(?:sk-[A-Za-z0-9_-]+|gh[pousr]_[A-Za-z0-9]+|AKIA[A-Z0-9]{16})(?-u:\b)"), "[SECRET]"),
                (re(r#"(?i)(?-u:\b)[a-z][a-z0-9+.-]*://[^\s<>"']+"#), "[URL]"),
                (re(r#"(?:[A-Za-z]:\\|\\\\)[^\s<>"']+"#), "[PATH]"),
                (re(r#"(?:/~|~/|/)[^\s<>"']+"#), "[PATH]"),
                (re(r"(?i)(?-u:\b)(?:[a-z0-9-]+\.)+(?:[a-z]{2,63}|local|internal)(?-u:\b)"), "[HOST]"),
                (re(r"(?i)(?-u:\b)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(?-u:\b)"), "[ID]"),
                (re(r"(?-u:\b)[A-Za-z0-9_+/=-]{32,}(?-u:\b)"), "[OPAQUE]"),
            ],
        }
    })
}
fn luhn(digits: &[u8]) -> bool {
    digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let mut d = u32::from(b - b'0');
            if i % 2 == 1 {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            d
        })
        .sum::<u32>()
        % 10
        == 0
}
fn ssn(d: &[u8]) -> bool {
    d.len() == 9
        && &d[..3] != b"000"
        && &d[..3] != b"666"
        && d[0] != b'9'
        && &d[3..5] != b"00"
        && &d[5..] != b"0000"
}
fn ipv6_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| !(c == ':' || c == '.' || c.is_ascii_alphanumeric() || c == '_'))
}
/// The full synchronous structured-PII layer from the pinned package.
pub fn deterministic(text: &str) -> Vec<Span> {
    let p = patterns();
    let mut out = Vec::new();
    for m in p.digit.find_iter(text) {
        let digits: Vec<u8> = m.as_str().bytes().filter(u8::is_ascii_digit).collect();
        let label = if [14, 15, 16].contains(&digits.len()) && luhn(&digits) {
            Some("CREDIT_CARD")
        } else if ssn(&digits) {
            Some("SSN")
        } else {
            None
        };
        if let Some(label) = label {
            out.push(span(m.start(), m.end(), label, 1.0));
        }
    }
    for (label, re) in &p.text {
        for m in re.find_iter(text) {
            out.push(span(m.start(), m.end(), label, 1.0));
        }
    }
    for m in p.ipv6_candidate.find_iter(text) {
        if p.ipv6.is_match(m.as_str())
            && ipv6_boundary(text[..m.start()].chars().next_back())
            && ipv6_boundary(text[m.end()..].chars().next())
            && m.as_str().parse::<std::net::Ipv6Addr>().is_ok()
        {
            out.push(span(m.start(), m.end(), "IP_ADDRESS", 1.0));
        }
    }
    out
}
fn heuristic(s: &Span) -> bool {
    s.score == 1.0
        && matches!(
            s.label.as_str(),
            "SSN" | "CREDIT_CARD" | "IP_ADDRESS" | "EMAIL" | "URL"
        )
}
fn prefer(a: &Span, b: &Span) -> bool {
    if a.score != b.score {
        return a.score > b.score;
    }
    if a.end - a.start != b.end - b.start {
        return a.end - a.start > b.end - b.start;
    }
    heuristic(a)
}
/// Confidence, length, heuristic tie-break; partial overlaps retain their union.
/// Invalid UTF-8 offsets and non-finite scores are rejected before slicing.
pub fn merge_spans(text: &str, mut spans: Vec<Span>) -> Vec<Span> {
    spans.retain(|s| valid(text, s));
    spans.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    let mut out: Vec<Span> = Vec::new();
    for s in spans {
        let Some(prev) = out.last_mut() else {
            out.push(s);
            continue;
        };
        if s.start >= prev.end {
            out.push(s);
            continue;
        }
        let containment = prev.end >= s.end || (s.start <= prev.start && s.end >= prev.end);
        let mut winner = if prefer(prev, &s) {
            prev.clone()
        } else {
            s.clone()
        };
        if !containment {
            winner.start = prev.start.min(s.start);
            winner.end = prev.end.max(s.end);
        }
        *prev = winner;
    }
    out
}
/// Byte-preserving blank mask, useful only for callers explicitly needing identity offsets.
/// Use `premask_mapped` for inference: the model was trained with label sentinels.
pub fn premask(text: &str, spans: &[Span]) -> String {
    let mut bytes = text.as_bytes().to_vec();
    for s in merge_spans(text, spans.to_vec()) {
        bytes[s.start..s.end].fill(b' ');
    }
    String::from_utf8(bytes).expect("whole UTF-8 spans replaced with ASCII")
}
#[derive(Clone, Debug)]
pub struct PremaskResult {
    pub text: String,
    raw_start: Vec<usize>,
    raw_end: Vec<usize>,
}
/// Exact label sentinels and per-byte projection onto the original UTF-8 input.
pub fn premask_mapped(text: &str, spans: &[Span]) -> PremaskResult {
    let mut result = PremaskResult {
        text: String::new(),
        raw_start: Vec::new(),
        raw_end: Vec::new(),
    };
    let mut cursor = 0;
    let copy = |r: &mut PremaskResult, from: usize, to: usize| {
        r.text.push_str(&text[from..to]);
        for (i, c) in text[from..to].char_indices() {
            for _ in 0..c.len_utf8() {
                r.raw_start.push(from + i);
                r.raw_end.push(from + i + c.len_utf8());
            }
        }
    };
    for s in merge_spans(text, spans.to_vec()) {
        copy(&mut result, cursor, s.start);
        let sentinel = format!("[{}]", safe_label(&s.label));
        for _ in sentinel.bytes() {
            result.raw_start.push(s.start);
            result.raw_end.push(s.end);
        }
        result.text.push_str(&sentinel);
        cursor = s.end;
    }
    copy(&mut result, cursor, text.len());
    result
}
pub fn project_masked_span(map: &PremaskResult, s: &Span) -> Option<Span> {
    if !valid(&map.text, s) {
        return None;
    }
    Some(span(
        *map.raw_start.get(s.start)?,
        *map.raw_end.get(s.end - 1)?,
        &s.label,
        s.score,
    ))
}
fn initial(text: &str, end: usize) -> bool {
    let mut chars = text[..end].chars().rev();
    chars.next().is_some_and(char::is_uppercase) && chars.next().is_none_or(|c| !c.is_alphabetic())
}
fn bridge(text: &str, a: &Span, b: &Span) -> bool {
    if a.label != b.label {
        return false;
    }
    let (left, right) = if a.start <= b.start { (a, b) } else { (b, a) };
    // JS slice(start,end) is empty when end < start (overlapping spans).
    let gap = if left.end > right.start {
        ""
    } else {
        &text[left.end..right.start]
    };
    patterns().connector.is_match(gap) && (!gap.contains('.') || initial(text, left.end))
}
fn adjacent(text: &str, mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by_key(|s| (s.start, s.end));
    let mut out: Vec<Span> = Vec::new();
    for s in spans {
        if let Some(p) = out.last_mut()
            && bridge(text, p, &s)
        {
            p.end = p.end.max(s.end);
            p.score = p.score.max(s.score);
            continue;
        }
        out.push(s);
    }
    out
}
fn rescue(text: &str, s: &Span, all: &[Span], index: usize) -> Span {
    if !matches!(s.label.as_str(), "GIVEN_NAME" | "SURNAME") {
        return s.clone();
    }
    let mut out = s.clone();
    let mut left_bound = 0;
    let mut right_bound = text.len();
    for (i, other) in all.iter().enumerate() {
        if i == index {
            continue;
        }
        if other.end <= s.start {
            left_bound = left_bound.max(other.end);
        }
        if other.start >= s.end {
            right_bound = right_bound.min(other.start);
        }
    }
    if let Some(c) = patterns().left.captures(&text[..s.start]) {
        let m = c.get(0).unwrap();
        if (!c[2].contains('.') || c[1].chars().count() == 1) && m.start() >= left_bound {
            out.start = m.start();
        }
    }
    if let Some(c) = patterns().right.captures(&text[s.end..]) {
        let m = c.get(0).unwrap();
        if !c[1].contains('.') && s.end + m.end() <= right_bound {
            out.end = s.end + m.end();
        }
    }
    out
}
/// Recall-biased bridge/connector/capitalized-particle repair, bounded to 32 passes.
pub fn repair_ner(text: &str, candidates: Vec<Span>, min_score: f32) -> Vec<Span> {
    if !min_score.is_finite() {
        return Vec::new();
    }
    let candidates: Vec<Span> = candidates.into_iter().filter(|s| valid(text, s)).collect();
    let mut kept: Vec<Span> = candidates
        .iter()
        .filter(|s| s.score >= min_score)
        .cloned()
        .collect();
    let mut low: Vec<Span> = candidates
        .into_iter()
        .filter(|s| s.score >= 0.15 && s.score < min_score)
        .collect();
    for _ in 0..32 {
        let mut changed = false;
        for i in (0..low.len()).rev() {
            if kept.iter().any(|s| bridge(text, &low[i], s)) {
                kept.push(low.remove(i));
                changed = true;
            }
        }
        let merged = adjacent(text, kept.clone());
        if merged.len() != kept.len()
            || merged
                .iter()
                .zip(&kept)
                .any(|(a, b)| a.start != b.start || a.end != b.end)
        {
            changed = true;
        }
        kept = merged;
        for i in 0..kept.len() {
            let repaired = rescue(text, &kept[i], &kept, i);
            if repaired.start != kept[i].start || repaired.end != kept[i].end {
                kept[i] = repaired;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    kept.sort_by_key(|s| (s.start, s.end));
    kept
}
fn safe_label(label: &str) -> &str {
    if !label.is_empty() && label.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        label
    } else {
        "REDACTED"
    }
}
/// Request-local stable placeholders; all labels, including geography, are denied.
/// No reverse/rehydration map survives or is exported. Numbering follows JS right-to-left.
pub fn replace(text: &str, spans: &[Span]) -> String {
    let mut result = text.to_owned();
    let mut forward: HashMap<(String, String), String> = HashMap::new();
    let mut counters: HashMap<String, usize> = HashMap::new();
    for s in merge_spans(text, spans.to_vec()).into_iter().rev() {
        let normalized = text[s.start..s.end]
            .to_lowercase()
            .nfc()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let key = (s.label.clone(), normalized);
        let token = forward.entry(key).or_insert_with(|| {
            let name = safe_label(&s.label).to_owned();
            let count = counters.entry(name.clone()).or_default();
            *count += 1;
            format!("[{name}_{count}]")
        });
        result.replace_range(s.start..s.end, token);
    }
    result
}
/// Additional credential, path, host and opaque-ID scrub, in JS replacement order.
/// Preserve email domains until the deterministic email recognizer can mask the whole address.
pub fn pre_redact(text: &str) -> String {
    patterns()
        .extra
        .iter()
        .fold(text.to_owned(), |text, (re, replacement)| {
            if *replacement == "[HOST]" {
                let mut result = String::new();
                let mut end = 0;
                for m in re.find_iter(&text) {
                    result.push_str(&text[end..m.start()]);
                    if text[..m.start()].ends_with('@') {
                        result.push_str(m.as_str());
                    } else {
                        result.push_str(replacement);
                    }
                    end = m.end();
                }
                result.push_str(&text[end..]);
                result
            } else {
                re.replace_all(&text, *replacement).into_owned()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_scrub_preserves_complete_email_detection() {
        let text = pre_redact("Contact zqx_75@example.com; inspect example.com");
        let spans = deterministic(&text);
        assert_eq!(spans.len(), 1);
        assert_eq!(&text[spans[0].start..spans[0].end], "zqx_75@example.com");
        assert!(!replace(&text, &spans).contains("zqx_75"));
        assert!(text.ends_with("[HOST]"));
    }
    #[test]
    fn validated_digit_runs() {
        let text = "4111-1111-1111-1111; 4111111111111112; 123 45 6789; 666-12-3456; 1234567890";
        let spans = deterministic(text);
        assert_eq!(
            spans.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(),
            ["CREDIT_CARD", "SSN"]
        );
        assert_eq!(&text[spans[0].start..spans[0].end], "4111-1111-1111-1111");
        for d in ["000123456", "900123456", "123003456", "123450000"] {
            assert!(deterministic(d).is_empty());
        }
    }
    #[test]
    fn structured_text_and_utf8_offsets() {
        let text = "é🙂 alex+housing@sub.example.gov https://example.gov/a www.example.org/x 192.168.1.2 2001:db8::1 ::1 aa:bb:cc:dd:ee:ff";
        let spans = deterministic(text);
        assert_eq!(spans.iter().filter(|s| s.label == "EMAIL").count(), 1);
        assert_eq!(spans.iter().filter(|s| s.label == "URL").count(), 2);
        assert_eq!(spans.iter().filter(|s| s.label == "IP_ADDRESS").count(), 4);
        assert!(spans.iter().all(|s| valid(text, s)));
        assert!(deterministic("12:34 x2001:db8::1y 256.256.256.256").is_empty());
    }
    #[test]
    fn sentinel_projection_and_multibyte_identity() {
        let text = "🙂 José x@example.org fin";
        let map = premask_mapped(text, &deterministic(text));
        assert_eq!(map.text, "🙂 José [EMAIL] fin");
        let start = map.text.find("José").unwrap();
        let p = project_masked_span(&map, &span(start, start + "José".len(), "GIVEN_NAME", 0.8))
            .unwrap();
        assert_eq!(&text[p.start..p.end], "José");
        let start = map.text.find("[EMAIL]").unwrap();
        let p = project_masked_span(&map, &span(start + 1, start + 3, "EMAIL", 0.8)).unwrap();
        assert_eq!(&text[p.start..p.end], "x@example.org");
        assert!(project_masked_span(&map, &span(1, 2, "SURNAME", 0.8)).is_none());
        let blank = premask(text, &[span(0, 4, "GIVEN_NAME", 1.0)]);
        assert_eq!(blank.len(), text.len());
        assert!(blank.starts_with("    "));
    }
    #[test]
    fn overlap_union_and_containment() {
        let text = "abcdefghij";
        let m = merge_spans(text, vec![span(0, 5, "A", 0.9), span(3, 8, "B", 0.8)]);
        assert_eq!((m[0].start, m[0].end, m[0].label.as_str()), (0, 8, "A"));
        let m = merge_spans(text, vec![span(0, 8, "A", 0.8), span(3, 5, "B", 0.9)]);
        assert_eq!((m[0].start, m[0].end, m[0].label.as_str()), (3, 5, "B"));
    }
    #[test]
    fn repair_low_scores_and_initials() {
        let text = "Jean-Luc";
        let m = repair_ner(
            text,
            vec![
                span(0, 4, "GIVEN_NAME", 0.8),
                span(5, 8, "GIVEN_NAME", 0.15),
            ],
            0.4,
        );
        assert_eq!((m[0].start, m[0].end), (0, 8));
        let text = "John F. Kennedy";
        let m = repair_ner(text, vec![span(8, 15, "SURNAME", 0.8)], 0.4);
        assert_eq!(&text[m[0].start..m[0].end], "John F. Kennedy");
        let text = "Garcia. I";
        let m = repair_ner(
            text,
            vec![span(0, 6, "SURNAME", 0.8), span(8, 9, "SURNAME", 0.2)],
            0.4,
        );
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].end, 6);
    }
    #[test]
    fn repair_respects_neighbor_and_unicode_particle() {
        let text = "José Å Smith";
        let m = repair_ner(
            text,
            vec![span(0, 5, "GIVEN_NAME", 0.8), span(9, 14, "SURNAME", 0.8)],
            0.4,
        );
        assert_eq!(m.len(), 2);
        assert!(m[0].end <= m[1].start);
        assert!(m.iter().all(|s| valid(text, s)));
    }
    #[test]
    fn all_labels_replaced_and_stable_nfc() {
        let text = "José Jose\u{301} Paris";
        let result = replace(
            text,
            &[
                span(0, 5, "GIVEN_NAME", 0.8),
                span(6, 12, "GIVEN_NAME", 0.8),
                span(13, 18, "CITY", 0.9),
            ],
        );
        assert_eq!(result, "[GIVEN_NAME_1] [GIVEN_NAME_1] [CITY_1]");
        assert_eq!(replace("é", &[span(1, 2, "SURNAME", 0.8)]), "é");
    }
    #[test]
    fn extra_scrub_covers_credentials_and_paths() {
        let text = "password='secret stuff' Bearer abc, sk-demo github.internal /Users/person/a C:\\Users\\person\\b 123e4567-e89b-12d3-a456-426614174000";
        let out = pre_redact(text);
        assert_eq!(out, "[SECRET] [SECRET], [SECRET] [HOST] [PATH] [PATH] [ID]");
        assert_eq!(
            pre_redact("-----BEGIN RSA PRIVATE KEY-----\nsecret\n-----END RSA PRIVATE KEY-----"),
            "[SECRET]"
        );
    }
}
