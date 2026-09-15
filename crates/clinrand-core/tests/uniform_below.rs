//! Hand-worked `uniform_below` cases and stream-consumption checks.
//!
//! Expected integers come from `validation/reference/uniform-below/`,
//! derived on paper from the documented keystream. They are not this
//! engine's own output.

use clinrand_core::{
    uniform_below, DrawPurpose, Rng, StreamDraw, StreamLog, U64Draw, UniformError,
};

const CASES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/uniform-below/cases.json"
));

#[test]
fn n_eq_1_returns_zero_and_consumes_no_u64() {
    let seed = [7u8; 32];
    let mut rng = Rng::from_seed(seed);
    let mut twin = Rng::from_seed(seed);
    let first = twin.next_u64();

    let mut log = StreamLog::default();
    let value = uniform_below(&mut rng, &mut log, 1, DrawPurpose::SimpleAllocation)
        .expect("n == 1 is a valid bound");

    assert_eq!(value, 0);
    assert!(log.draws.is_empty(), "n == 1 must not record a draw");
    assert_eq!(
        rng.next_u64(),
        first,
        "n == 1 must consume zero next_u64 draws"
    );
}

#[test]
fn n_eq_0_returns_error_and_consumes_no_u64() {
    let seed = [3u8; 32];
    let mut rng = Rng::from_seed(seed);
    let mut twin = Rng::from_seed(seed);
    let first = twin.next_u64();

    let mut log = StreamLog::default();
    let err = uniform_below(&mut rng, &mut log, 0, DrawPurpose::BlockSize)
        .expect_err("n == 0 is not a valid bound");

    assert_eq!(err, UniformError::ZeroBound);
    assert!(log.draws.is_empty());
    assert_eq!(rng.next_u64(), first);
}

#[test]
fn n_eq_0_error_matches_reference_case() {
    assert_reference_case("n_eq_0_error");
}

#[test]
fn n_eq_1_zero_consumption_matches_reference_case() {
    assert_reference_case("n_eq_1_zero_consumption");
}

#[test]
fn n_eq_3_no_rejection_matches_reference_case() {
    assert_reference_case("n_eq_3_no_rejection");
}

#[test]
fn n_eq_2_rejection_matches_reference_case() {
    assert_reference_case("n_eq_2_rejection");
}

struct DocumentedStream {
    words: Vec<u64>,
    pos: usize,
}

impl U64Draw for DocumentedStream {
    fn next_u64(&mut self) -> u64 {
        let x = self.words[self.pos];
        self.pos = self
            .pos
            .checked_add(1)
            .expect("documented keystream exhausted");
        x
    }
}

fn assert_reference_case(case_id: &str) {
    assert!(
        CASES_JSON.contains("\"primitiveId\": \"uniform-below\""),
        "cases.json must declare primitiveId uniform-below"
    );
    let object = case_object(case_id);
    let input = json_object(object, "input");
    let expect = json_object(object, "expect");
    let n = json_u64(input, "n");
    let purpose = parse_purpose(&json_ident(input, "purpose"));
    let words = json_u64_strings(input, "keystream");
    let mut stream = DocumentedStream { words, pos: 0 };
    let mut log = StreamLog::default();
    let result = uniform_below(&mut stream, &mut log, n, purpose);

    let consumed = json_u64(expect, "consumed");
    assert_eq!(
        u64::try_from(stream.pos).expect("pos fits u64"),
        consumed,
        "{case_id}: consumed words"
    );

    let expected_stream = json_stream(expect);
    assert_eq!(log.draws, expected_stream, "{case_id}: StreamLog");

    if let Some(code) = json_optional_str(expect, "error") {
        assert_eq!(code, "zero_bound", "{case_id}: only zero_bound is defined");
        let err = result.expect_err("{case_id}: expected error");
        assert_eq!(err, UniformError::ZeroBound);
        return;
    }

    let value = result.expect("{case_id}: expected a value");
    assert_eq!(value, json_u64(expect, "value"), "{case_id}: value");
}

fn parse_purpose(name: &str) -> DrawPurpose {
    match name {
        "BlockSize" => DrawPurpose::BlockSize,
        "Permutation" => DrawPurpose::Permutation,
        "SimpleAllocation" => DrawPurpose::SimpleAllocation,
        other => panic!("unknown purpose {other}"),
    }
}

fn json_stream(expect: &str) -> Vec<StreamDraw> {
    let needle = "\"stream\": ";
    let start = expect
        .find(needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing stream array"));
    let rest = expect[start..].trim_start();
    if rest.starts_with("[]") {
        return Vec::new();
    }
    let array = json_bracketed(rest, '[', ']');
    let mut draws = Vec::new();
    let mut search = array;
    while let Some(rel) = search.find('{') {
        let obj = json_bracketed(&search[rel..], '{', '}');
        draws.push(StreamDraw {
            index: json_u64(obj, "index"),
            bound: json_u64(obj, "bound"),
            value: json_u64(obj, "value"),
            purpose: parse_purpose(&json_ident(obj, "purpose")),
        });
        search = &search[rel + obj.len()..];
    }
    draws
}

fn json_u64_strings(object: &str, key: &str) -> Vec<u64> {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing array field {key}"));
    let array = json_bracketed(object[start..].trim_start(), '[', ']');
    array
        .split(',')
        .filter_map(|part| {
            let part = part.trim().trim_matches(|c| c == '[' || c == ']');
            let part = part.trim().trim_matches('"').trim();
            if part.is_empty() {
                None
            } else {
                Some(
                    part.parse::<u64>()
                        .unwrap_or_else(|err| panic!("invalid u64 {part}: {err}")),
                )
            }
        })
        .collect()
}

fn json_object<'a>(parent: &'a str, key: &str) -> &'a str {
    let needle = format!("\"{key}\": ");
    let start = parent
        .find(&needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing object field {key}"));
    json_bracketed(parent[start..].trim_start(), '{', '}')
}

fn json_bracketed(rest: &str, open: char, close: char) -> &str {
    let bytes = rest.as_bytes();
    assert_eq!(rest.chars().next(), Some(open));
    let mut depth = 0usize;
    for (offset, ch) in rest.char_indices() {
        if ch == open {
            depth = depth.saturating_add(1);
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let end = offset + close.len_utf8();
                return std::str::from_utf8(&bytes[..end]).expect("json slice is utf-8");
            }
        }
    }
    panic!("unterminated bracketed value");
}

fn case_object(case_id: &str) -> &'static str {
    let needle = format!("\"caseId\": \"{case_id}\"");
    let id_pos = CASES_JSON
        .find(&needle)
        .unwrap_or_else(|| panic!("missing case {case_id}"));
    let obj_start = CASES_JSON[..id_pos]
        .rfind('{')
        .unwrap_or_else(|| panic!("unterminated case {case_id}"));
    json_bracketed(&CASES_JSON[obj_start..], '{', '}')
}

fn json_ident(object: &str, key: &str) -> String {
    json_optional_str(object, key).unwrap_or_else(|| panic!("missing string field {key}"))
}

fn json_optional_str(object: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\": \"");
    let start = object.find(&needle).map(|i| i + needle.len())?;
    let end = object[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated string field {key}"));
    Some(object[start..start + end].to_string())
}

fn json_u64(object: &str, key: &str) -> u64 {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing integer field {key}"));
    object[start..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or_else(|| panic!("empty integer field {key}"))
        .parse()
        .unwrap_or_else(|err| panic!("invalid integer field {key}: {err}"))
}
