use std::{cell::RefCell, fs, path::PathBuf, process::Command};

use csv::{Error, Parser, CSV_STRICT, CSV_STRICT_FINI};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Field(Option<Vec<u8>>),
    Row(i32),
}

fn field(bytes: &[u8]) -> Event {
    Event::Field(Some(bytes.to_vec()))
}

fn row(term: i32) -> Event {
    Event::Row(term)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_bytes(path: &str) -> Vec<u8> {
    fs::read(workspace_root().join(path)).unwrap()
}

fn parse_and_finish(parser: &mut Parser, input: &[u8]) -> (usize, Result<(), Error>, Vec<Event>) {
    let events = RefCell::new(Vec::new());
    let mut on_field = |value: Option<&[u8]>| {
        events
            .borrow_mut()
            .push(Event::Field(value.map(|bytes| bytes.to_vec())));
    };
    let mut on_row = |term: i32| {
        events.borrow_mut().push(Event::Row(term));
    };

    let consumed = parser.parse(input, &mut on_field, &mut on_row);
    let finish_result = if consumed == input.len() {
        parser.finish(&mut on_field, &mut on_row)
    } else {
        Ok(())
    };

    (consumed, finish_result, events.into_inner())
}

#[test]
fn readstat_semicolon_fixture_preserves_embedded_delimiters() {
    let input = fixture_bytes("downstream/fixtures/readstat/input.csv");
    let mut parser = Parser::new(0);
    parser.set_delimiter(b';');

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, &input);

    assert_eq!(consumed, input.len());
    assert_eq!(finish_result, Ok(()));
    assert_eq!(
        events,
        vec![
            field(b"name"),
            field(b"score"),
            field(b"notes"),
            row(i32::from(b'\n')),
            field(b"Alice, A."),
            field(b"42"),
            field(b"likes;semicolons"),
            row(i32::from(b'\n')),
            field(b"Bob"),
            field(b"7"),
            field(b"plain text"),
            row(i32::from(b'\n')),
        ]
    );
}

#[test]
fn csvvalid_fixture_reports_the_original_malformed_byte_offset() {
    let input = fixture_bytes("downstream/fixtures/shared/bad-malformed.csv");
    let mut parser = Parser::new(CSV_STRICT);
    let events = RefCell::new(Vec::new());
    let mut on_field = |value: Option<&[u8]>| {
        events
            .borrow_mut()
            .push(Event::Field(value.map(|bytes| bytes.to_vec())));
    };
    let mut on_row = |term: i32| {
        events.borrow_mut().push(Event::Row(term));
    };

    let consumed = parser.parse(&input, &mut on_field, &mut on_row);

    assert_eq!(consumed + 1, 23);
    assert_eq!(parser.error(), Error::Parse);
    assert_eq!(
        events.into_inner(),
        vec![
            field(b"name"),
            field(b"notes"),
            row(i32::from(b'\n')),
            field(b"Alice"),
        ]
    );
}

#[test]
fn csvcheck_fixture_requires_strict_finish_for_unterminated_quotes() {
    let input = fixture_bytes("downstream/fixtures/shared/bad-unterminated.csv");
    let mut parser = Parser::new(CSV_STRICT | CSV_STRICT_FINI);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, &input);

    assert_eq!(consumed, input.len());
    assert_eq!(finish_result, Err(Error::Parse));
    assert_eq!(parser.error(), Error::Parse);
    assert_eq!(
        events,
        vec![
            field(b"name"),
            field(b"notes"),
            row(i32::from(b'\n')),
            field(b"Alice"),
        ]
    );
}

#[test]
fn downstream_inventory_pins_final_verifier_topology() {
    let workspace_root = workspace_root();
    let inventory_path = workspace_root.join("downstream-apps.json");
    let inventory = fs::read_to_string(&inventory_path).unwrap();

    assert!(inventory.contains("\"id\": \"check_final_software_tester\""));
    assert!(inventory.contains("\"id\": \"check_final_senior_tester\""));
    assert!(inventory.contains("\"bounce_target\": \"impl_final_catchall\""));
    assert!(!inventory.contains("check_wave2_software_tester"));
    assert!(!inventory.contains("impl_wave2_integration_fixes"));
    assert!(!inventory.contains("check_wave1_software_tester"));
    assert!(!inventory.contains("impl_wave1_compat_fixes"));

    let output = Command::new("python3")
        .arg(workspace_root.join("scripts/downstream-matrix.py"))
        .arg("validate")
        .arg("--inventory")
        .arg(&inventory_path)
        .current_dir(&workspace_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "validator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
