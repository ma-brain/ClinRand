//! RFC 8439 reference cases for the ChaCha20 generator.
//!
//! Expected bytes are loaded from `validation/reference/chacha20/cases.json`,
//! which copies RFC 8439 dumps. They are not this engine's own output.

use clinrand_core::Rng;

const CASES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../validation/reference/chacha20/cases.json"
));

struct Case {
    key: [u8; 32],
    nonce: [u8; 12],
    block_counter: u32,
    expected: Vec<u8>,
}

#[test]
fn rfc8439_2_3_2_block_matches() {
    let case = load_case("rfc8439_2_3_2_block");
    assert_eq!(case.expected.len(), 64);
    assert_keystream(&case);
}

#[test]
fn rfc8439_2_4_2_keystream_matches() {
    let case = load_case("rfc8439_2_4_2_keystream");
    assert_eq!(case.expected.len(), 114);
    assert_keystream(&case);
}

fn assert_keystream(case: &Case) {
    let mut rng = Rng::from_ietf(case.key, case.nonce, case.block_counter);
    let mut got = vec![0u8; case.expected.len()];
    rng.fill_bytes(&mut got);
    assert_eq!(got, case.expected);
}

fn load_case(case_id: &str) -> Case {
    assert!(
        CASES_JSON.contains("\"primitiveId\": \"chacha20\""),
        "cases.json must declare primitiveId chacha20"
    );
    let object = case_object(case_id);
    let key = hex_array::<32>(&json_str(object, "key"));
    let nonce = hex_array::<12>(&json_str(object, "nonce"));
    Case {
        key,
        nonce,
        block_counter: json_u32(object, "blockCounter"),
        expected: decode_hex(&json_str(object, "keystream")),
    }
}

fn case_object(case_id: &str) -> &'static str {
    let needle = format!("\"caseId\": \"{case_id}\"");
    let id_pos = CASES_JSON
        .find(&needle)
        .unwrap_or_else(|| panic!("missing case {case_id}"));
    let obj_start = CASES_JSON[..id_pos]
        .rfind('{')
        .unwrap_or_else(|| panic!("unterminated case {case_id}"));
    let bytes = CASES_JSON.as_bytes();
    let mut depth = 0usize;
    for (offset, byte) in bytes[obj_start..].iter().enumerate() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return &CASES_JSON[obj_start..=obj_start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated case {case_id}");
}

fn json_str(object: &str, key: &str) -> String {
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

fn json_u32(object: &str, key: &str) -> u32 {
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

fn decode_hex(hex: &str) -> Vec<u8> {
    assert!(hex.len().is_multiple_of(2), "hex length must be even");
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair).expect("hex is utf-8");
            u8::from_str_radix(s, 16).unwrap_or_else(|err| panic!("invalid hex {s}: {err}"))
        })
        .collect()
}

fn hex_array<const N: usize>(hex: &str) -> [u8; N] {
    decode_hex(hex)
        .try_into()
        .unwrap_or_else(|bytes: Vec<u8>| panic!("expected {N} bytes, got {}", bytes.len()))
}
