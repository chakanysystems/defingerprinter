//! Subprocess coverage of the actual JSONL consent and failure boundaries.
use serde_json::{Value, json};
use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

fn exchange(records: &[Vec<u8>], actual_model: bool) -> Vec<Value> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_defingerprinter"));
    if actual_model {
        command.args([
            "--model-dir",
            &env::var("RAMPART_MODEL_DIR").expect("RAMPART_MODEL_DIR required"),
            "--runtime",
            &env::var("RAMPART_RUNTIME").expect("RAMPART_RUNTIME required"),
        ]);
    } else {
        command.args([
            "--model-dir",
            "/missing/seismograph-fixture",
            "--runtime",
            "/missing/seismograph-runtime",
        ]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for record in records {
        stdin.write_all(record).unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "raw errors or runtime logging reached stderr"
    );
    let responses: Vec<Value> = output
        .stdout
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("JSON response"))
        .collect();
    assert_eq!(
        responses.len(),
        records.len(),
        "one response per record, including rejected records"
    );
    responses
}
fn records(values: &[Value]) -> Vec<Vec<u8>> {
    values
        .iter()
        .map(|v| serde_json::to_vec(v).unwrap())
        .collect()
}
fn protect(language: &str, text: &str) -> Value {
    json!({"op":"protect","language":language,"text":text})
}

#[test]
fn consent_failure_revocation_and_admission() {
    let requests = records(&[
        json!({"op":"status"}),
        protect("en", "private-fixture@example.org"),
        json!({"op":"consent","enabled":true}),
        protect("ru", "private-fixture@example.org"),
        protect("en", "Αlice private-fixture@example.org"),
        protect("en", "private\0fixture"),
        protect("en", &"a".repeat(4097)),
        protect("en", "private-fixture@example.org"),
        json!({"op":"status"}),
        json!({"op":"consent","enabled":false}),
        protect("en", "private-fixture@example.org"),
        json!({"op":"status"}),
    ]);
    let responses = exchange(&requests, false);
    assert_eq!(responses[0]["enabled"], false);
    assert_eq!(responses[0]["ready"], false);
    assert_eq!(responses[1]["reason"], "disabled");
    for response in &responses[3..7] {
        assert_eq!(response["reason"], "unsupported_input");
    }
    assert_eq!(responses[7]["reason"], "redaction_failed");
    assert_eq!(responses[8]["ready"], false);
    assert_eq!(responses[10]["reason"], "disabled");
    assert_eq!(responses[11]["enabled"], false);
    assert!(
        !serde_json::to_string(&responses)
            .unwrap()
            .contains("private-fixture")
    );
}

#[test]
fn rejected_record_draining_and_eof() {
    let mut input = vec![
        b"not-json private-fixture".to_vec(),
        vec![0xff],
        vec![b'x'; 40000],
    ];
    input.extend(records(&[
        json!({"op":"status"}),
        json!({"op":"consent","enabled":true,"extra":"private-fixture"}),
        json!({"op":"status"}),
    ]));
    let responses = exchange(&input, false);
    for response in &responses[..3] {
        assert_eq!(response["reason"], "invalid_request");
    }
    assert_eq!(responses[3]["enabled"], false);
    assert_eq!(responses[4]["reason"], "invalid_request");
    assert_eq!(responses[5]["enabled"], false);
    assert!(
        !serde_json::to_string(&responses)
            .unwrap()
            .contains("private-fixture")
    );
}

#[test]
#[ignore = "requires pinned model and local ORT; set RAMPART_MODEL_DIR and RAMPART_RUNTIME"]
fn actual_model_redacts_and_revocation_drops_session() {
    let text = "Alice Johnson email alice@example.org SSN 123-45-6789 card 4111 1111 1111 1111 /Users/alice/private token=sk-synthetic-secret-value";
    let responses = exchange(
        &records(&[
            protect("en", text),
            json!({"op":"consent","enabled":true}),
            protect("en", text),
            json!({"op":"status"}),
            json!({"op":"consent","enabled":false}),
            json!({"op":"status"}),
            protect("en", text),
        ]),
        true,
    );
    assert_eq!(responses[0]["reason"], "disabled");
    assert_eq!(responses[2]["ok"], true);
    let safe = responses[2]["text"].as_str().unwrap();
    for private in [
        "Alice",
        "Johnson",
        "alice@example.org",
        "123-45-6789",
        "4111 1111 1111 1111",
        "/Users/alice/private",
        "sk-synthetic-secret-value",
    ] {
        assert!(
            !safe.contains(private),
            "PII fixture not redacted: {private}"
        );
    }
    assert!(safe.contains("["));
    assert_eq!(responses[3]["ready"], true);
    assert_eq!(responses[5]["ready"], false);
    assert_eq!(responses[5]["enabled"], false);
    assert_eq!(responses[6]["reason"], "disabled");
}

#[test]
fn unsupported_script_digits_and_symbols() {
    let cases = [
        "Account ١٢٣٤٥٦",
        "Account １２３４５６",
        "Account १२३४५६",
        "name ᚠ",
        "name Аlice",
        "symbol א",
    ];
    let mut input = vec![json!({"op":"consent","enabled":true})];
    input.extend(cases.iter().map(|text| protect("en", text)));
    let responses = exchange(&records(&input), false);
    for response in &responses[1..] {
        assert_eq!(response["reason"], "unsupported_input");
        assert!(response.get("text").is_none());
    }
}

#[test]
#[ignore = "requires pinned model and local ORT; set RAMPART_MODEL_DIR and RAMPART_RUNTIME"]
fn actual_model_consumes_complete_credentials_and_identifiers() {
    let cases = [
        ("Authorization: Bearer abcdefghijklmnop", "[SECRET]"),
        ("authorization: Basic YWJjOmRlZg==", "[SECRET]"),
        (r#"{"token":"moss-green-lantern"}"#, "{[SECRET]}"),
        (r#"{"api_key":"test-value"}"#, "{[SECRET]}"),
        (r#"password="ab\"cdefghijk""#, "[SECRET]"),
        (r#"password='ab\'cdefghijk'"#, "[SECRET]"),
        ("Contact zqx_75+tag.foo@example.com", "Contact [EMAIL_1]"),
        (
            "opaque abcdefghijklmno/pqrstuvwxyz0123456789",
            "opaque [OPAQUE]",
        ),
    ];
    let mut input = vec![json!({"op":"consent","enabled":true})];
    input.extend(cases.iter().map(|(text, _)| protect("en", text)));
    let responses = exchange(&records(&input), true);
    for (response, (text, expected)) in responses[1..].iter().zip(cases) {
        assert_eq!(response["ok"], true, "{text}");
        assert_eq!(response["text"], expected, "{text}");
    }
}

#[test]
#[ignore = "requires pinned model and local ORT; set RAMPART_MODEL_DIR and RAMPART_RUNTIME"]
fn actual_model_output_bound_and_reconsent() {
    let text = "The operation failed while repairing a damaged object. ".repeat(60);
    let responses = exchange(
        &records(&[
            json!({"op":"consent","enabled":true}),
            protect("en", &text),
            json!({"op":"consent","enabled":false}),
            json!({"op":"consent","enabled":true}),
            json!({"op":"status"}),
            protect("en", "Contact José García at jose@example.org."),
            json!({"op":"status"}),
        ]),
        true,
    );
    assert_eq!(responses[1]["ok"], true);
    assert!(responses[1]["text"].as_str().unwrap().chars().count() <= 1024);
    assert_eq!(responses[4]["ready"], false);
    let safe = responses[5]["text"].as_str().unwrap();
    assert!(
        !safe.contains("José") && !safe.contains("García") && !safe.contains("jose@example.org")
    );
    assert_eq!(responses[6]["ready"], true);
}
