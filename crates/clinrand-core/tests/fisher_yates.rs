//! Hand-worked Fisher–Yates cases and stream-consumption checks.
//!
//! Expected orders come from `validation/reference/fisher-yates/`,
//! derived on paper from the documented keystream. They are not this
//! engine's own output.

use clinrand_core::{permute, DrawPurpose, StreamDraw, StreamLog, U64Draw, UniformError};

const CASES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/fisher-yates/cases.json"
));

#[test]
fn len_0_identity_matches_reference_case() {
    assert_reference_case("len_0_identity");
}

#[test]
fn len_1_zero_consumption_matches_reference_case() {
    assert_reference_case("len_1_zero_consumption");
}

#[test]
fn len_2_one_draw_swap_matches_reference_case() {
    assert_reference_case("len_2_one_draw_swap");
}

#[test]
fn len_3_descending_swaps_matches_reference_case() {
    assert_reference_case("len_3_descending_swaps");
}

#[test]
fn len_0_and_len_1_consume_no_u64_on_live_rng() {
    use clinrand_core::Rng;

    let seed = [9u8; 32];
    let mut rng = Rng::from_seed(seed);
    let mut twin = Rng::from_seed(seed);
    let first = twin.next_u64();

    let mut log = StreamLog::default();
    let mut empty: [u32; 0] = [];
    permute(&mut rng, &mut log, &mut empty).expect("empty slice is valid");
    assert!(log.draws.is_empty());

    let mut singleton = [7u32];
    permute(&mut rng, &mut log, &mut singleton).expect("len 1 is valid");
    assert_eq!(singleton, [7]);
    assert!(log.draws.is_empty());
    assert_eq!(
        rng.next_u64(),
        first,
        "len 0 and len 1 must consume zero next_u64 draws"
    );
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
        CASES_JSON.contains("\"primitiveId\": \"fisher-yates\""),
        "cases.json must declare primitiveId fisher-yates"
    );
    let object = case_object(case_id);
    let input = json_object(object, "input");
    let expect = json_object(object, "expect");
    let mut items = json_i64_array(input, "items");
    let words = json_u64_strings(input, "keystream");
    let mut stream = DocumentedStream { words, pos: 0 };
    let mut log = StreamLog::default();

    let result: Result<(), UniformError> = permute(&mut stream, &mut log, &mut items);
    result.unwrap_or_else(|err| panic!("{case_id}: permute must succeed: {err}"));

    let consumed = json_u64(expect, "consumed");
    assert_eq!(
        u64::try_from(stream.pos).expect("pos fits u64"),
        consumed,
        "{case_id}: consumed words"
    );
    assert_eq!(
        items,
        json_i64_array(expect, "items"),
        "{case_id}: final order"
    );
    assert_eq!(log.draws, json_stream(expect), "{case_id}: StreamLog");
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

fn json_i64_array(object: &str, key: &str) -> Vec<i64> {
    let needle = format!("\"{key}\": ");
    let start = object
        .find(&needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing array field {key}"));
    let array = json_bracketed(object[start..].trim_start(), '[', ']');
    let inner = array.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|part| {
            let part = part.trim();
            part.parse::<i64>()
                .unwrap_or_else(|err| panic!("invalid i64 {part}: {err}"))
        })
        .collect()
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
    let needle = format!("\"{key}\": \"");
    let start = object
        .find(&needle)
        .map(|i| i + needle.len())
        .unwrap_or_else(|| panic!("missing string field {key}"));
    let end = object[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated string field {key}"));
    object[start..start + end].to_string()
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
