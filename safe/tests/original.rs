use std::cell::RefCell;

use csv::{
    write_to_buffer, write_to_buffer_with_quote, Error, Parser, CSV_COMMA, CSV_EMPTY_IS_NULL,
    CSV_QUOTE, CSV_REPALL_NL, CSV_SPACE, CSV_STRICT, CSV_STRICT_FINI, CSV_TAB, END_OF_INPUT,
};

// The original C test suite in `original/test_csv.c` asserts only on public
// behavior. This Rust translation keeps that boundary intact by checking
// emitted field/row events and public configuration methods only.

#[derive(Clone, Debug, PartialEq, Eq)]
enum Event {
    Col(Option<Vec<u8>>),
    Row(i32),
}

#[derive(Clone)]
struct Case<'a> {
    upstream_case: &'a str,
    options_label: &'a str,
    input: &'a [u8],
    options: u8,
    expected: Vec<Event>,
    expect_error: bool,
    delimiter: u8,
    quote: u8,
    space_fn: Option<fn(u8) -> bool>,
    term_fn: Option<fn(u8) -> bool>,
}

fn col(data: &[u8]) -> Event {
    Event::Col(Some(data.to_vec()))
}

fn null_col() -> Event {
    Event::Col(None)
}

fn row(term: i32) -> Event {
    Event::Row(term)
}

fn run_case(case: &Case<'_>) {
    let max_chunk_size = case.input.len().max(1);
    let case_name = format!("{} [{}]", case.upstream_case, case.options_label);

    for chunk_size in 1..=max_chunk_size {
        let mut parser = Parser::new(case.options);
        parser.set_delimiter(case.delimiter);
        parser.set_quote(case.quote);
        parser.set_space_predicate(case.space_fn);
        parser.set_term_predicate(case.term_fn);

        let actual = RefCell::new(Vec::new());
        let mut on_field = |field: Option<&[u8]>| {
            actual
                .borrow_mut()
                .push(Event::Col(field.map(|bytes| bytes.to_vec())));
        };
        let mut on_row = |term: i32| {
            actual.borrow_mut().push(Event::Row(term));
        };

        let mut bytes_processed = 0usize;
        let mut saw_error = false;

        while bytes_processed < case.input.len() {
            let bytes = chunk_size.min(case.input.len() - bytes_processed);
            let consumed = parser.parse(
                &case.input[bytes_processed..bytes_processed + bytes],
                &mut on_field,
                &mut on_row,
            );

            if consumed != bytes {
                saw_error = true;
                assert!(
                    case.expect_error,
                    "{}: unexpected parse error after consuming {consumed} of {bytes} bytes",
                    case_name
                );
                assert_eq!(parser.error(), Error::Parse, "{}: wrong error", case_name);
                break;
            }

            bytes_processed += bytes;
        }

        if !saw_error {
            match parser.finish(&mut on_field, &mut on_row) {
                Ok(()) => assert!(
                    !case.expect_error,
                    "{}: expected parse error during finish",
                    case_name
                ),
                Err(err) => {
                    saw_error = true;
                    assert!(
                        case.expect_error,
                        "{}: unexpected finish error {err:?}",
                        case_name
                    );
                    assert_eq!(err, Error::Parse, "{}: wrong finish error", case_name);
                }
            }
        }

        assert_eq!(
            actual.into_inner(),
            case.expected,
            "{}: event mismatch at chunk size {chunk_size}",
            case_name
        );
        assert_eq!(
            saw_error, case.expect_error,
            "{}: error expectation mismatch at chunk size {chunk_size}",
            case_name
        );
    }
}

#[test]
fn translated_original_parser_cases() {
    let test04_data = concat!(
        "\"I call our world Flatland,\n",
        "not because we call it so,\n",
        "but to make its nature clearer\n",
        "to you, my happy readers,\n",
        "who are privileged to live in Space.\""
    );
    let test04_field = concat!(
        "I call our world Flatland,\n",
        "not because we call it so,\n",
        "but to make its nature clearer\n",
        "to you, my happy readers,\n",
        "who are privileged to live in Space."
    );
    let test08_data = concat!(
        "\" abc\"                                             ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                          \", \"123\""
    );
    let test08_first_field = concat!(
        " abc\"                                               ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                                     ",
        "                                        "
    );

    let test01_expected = vec![
        col(b"1"),
        col(b"2"),
        col(b"3"),
        col(b"4"),
        col(b"5"),
        row(i32::from(b'\r')),
    ];
    let test02_expected = vec![
        col(b""),
        col(b""),
        col(b""),
        col(b""),
        col(b""),
        col(b""),
        row(i32::from(b'\n')),
    ];
    let test03_expected = vec![col(b","), col(b","), col(b""), row(END_OF_INPUT)];
    let test04_expected = vec![col(test04_field.as_bytes()), row(END_OF_INPUT)];
    let test05_expected = vec![
        col(b"\"a,b\""),
        col(b""),
        col(b" \"\" "),
        col(b"\"\" "),
        col(b" \"\""),
        col(b"\"\""),
        row(END_OF_INPUT),
    ];
    let test06_expected = vec![
        col(b" a, b ,c "),
        col(b"a b  c"),
        col(b""),
        row(END_OF_INPUT),
    ];
    let test07_expected = vec![col(b" \" \" \" \" "), row(END_OF_INPUT)];
    let test08_expected = vec![
        col(test08_first_field.as_bytes()),
        col(b"123"),
        row(END_OF_INPUT),
    ];
    let test09_expected = Vec::new();
    let test10_expected = vec![col(b"a"), row(i32::from(b'\n'))];
    let test11_expected = vec![
        col(b"1"),
        col(b"2"),
        col(b"3"),
        col(b"4"),
        row(i32::from(b'\n')),
    ];
    let test12_expected = Vec::new();
    let test12b_expected = vec![
        row(i32::from(b'\n')),
        row(i32::from(b'\n')),
        row(i32::from(b'\n')),
        row(i32::from(b'\n')),
    ];
    let test13_expected = vec![col(b"abc"), row(END_OF_INPUT)];
    let test14_expected = vec![
        col(b"1"),
        col(b"2"),
        col(b"3"),
        col(b""),
        row(i32::from(b'\n')),
        col(b"4"),
        col(b""),
        row(i32::from(b'\r')),
        col(b""),
        col(b""),
        row(END_OF_INPUT),
    ];
    let test15_expected = vec![
        col(b"1"),
        col(b"2"),
        col(b"3"),
        col(b""),
        row(i32::from(b'\n')),
        col(b"4"),
        col(b""),
        row(i32::from(b'\r')),
        col(b""),
        row(END_OF_INPUT),
    ];
    let test16_expected = vec![col(b"1"), col(b"2"), col(b" 3 "), row(END_OF_INPUT)];
    let test17_expected = vec![col(b"a\0b\0c"), row(END_OF_INPUT)];
    let test19_expected = vec![null_col(), col(b""), null_col(), row(END_OF_INPUT)];
    let custom01_expected = vec![
        col(b"'a;b'"),
        col(b""),
        col(b" '' "),
        col(b"'' "),
        col(b" ''"),
        col(b"''"),
        row(END_OF_INPUT),
    ];

    let cases = vec![
        Case {
            upstream_case: "test01",
            options_label: "0",
            input: b" 1,2 ,  3         ,4,5\r\n",
            options: 0,
            expected: test01_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test01",
            options_label: "CSV_STRICT",
            input: b" 1,2 ,  3         ,4,5\r\n",
            options: CSV_STRICT,
            expected: test01_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test01",
            options_label: "CSV_STRICT | CSV_EMPTY_IS_NULL",
            input: b" 1,2 ,  3         ,4,5\r\n",
            options: CSV_STRICT | CSV_EMPTY_IS_NULL,
            expected: test01_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test02",
            options_label: "0",
            input: b",,,,,\n",
            options: 0,
            expected: test02_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test02",
            options_label: "CSV_STRICT",
            input: b",,,,,\n",
            options: CSV_STRICT,
            expected: test02_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test03",
            options_label: "0",
            input: b"\",\",\",\",\"\"",
            options: 0,
            expected: test03_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test03",
            options_label: "CSV_STRICT",
            input: b"\",\",\",\",\"\"",
            options: CSV_STRICT,
            expected: test03_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test04",
            options_label: "0",
            input: test04_data.as_bytes(),
            options: 0,
            expected: test04_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test04",
            options_label: "CSV_STRICT",
            input: test04_data.as_bytes(),
            options: CSV_STRICT,
            expected: test04_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test05",
            options_label: "0",
            input: b"\"\"\"a,b\"\"\",,\" \"\"\"\" \",\"\"\"\"\" \",\" \"\"\"\"\",\"\"\"\"\"\"",
            options: 0,
            expected: test05_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test05",
            options_label: "CSV_STRICT",
            input: b"\"\"\"a,b\"\"\",,\" \"\"\"\" \",\"\"\"\"\" \",\" \"\"\"\"\",\"\"\"\"\"\"",
            options: CSV_STRICT,
            expected: test05_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test05",
            options_label: "CSV_STRICT | CSV_STRICT_FINI",
            input: b"\"\"\"a,b\"\"\",,\" \"\"\"\" \",\"\"\"\"\" \",\" \"\"\"\"\",\"\"\"\"\"\"",
            options: CSV_STRICT | CSV_STRICT_FINI,
            expected: test05_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test06",
            options_label: "0",
            input: b"\" a, b ,c \", a b  c,",
            options: 0,
            expected: test06_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test06",
            options_label: "CSV_STRICT",
            input: b"\" a, b ,c \", a b  c,",
            options: CSV_STRICT,
            expected: test06_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test07",
            options_label: "0",
            input: b"\" \"\" \" \" \"\" \"",
            options: 0,
            expected: test07_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test07b",
            options_label: "CSV_STRICT",
            input: b"\" \"\" \" \" \"\" \"",
            options: CSV_STRICT,
            expected: Vec::new(),
            expect_error: true,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test08",
            options_label: "0",
            input: test08_data.as_bytes(),
            options: 0,
            expected: test08_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test09",
            options_label: "0",
            input: b"",
            options: 0,
            expected: test09_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test09",
            options_label: "CSV_EMPTY_IS_NULL",
            input: b"",
            options: CSV_EMPTY_IS_NULL,
            expected: test09_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test10",
            options_label: "0",
            input: b"a\n",
            options: 0,
            expected: test10_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test11",
            options_label: "0",
            input: b"1,2 ,3,4\n",
            options: 0,
            expected: test11_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test11",
            options_label: "CSV_EMPTY_IS_NULL",
            input: b"1,2 ,3,4\n",
            options: CSV_EMPTY_IS_NULL,
            expected: test11_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test12",
            options_label: "0",
            input: b"\n\n\n\n",
            options: 0,
            expected: test12_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test12",
            options_label: "CSV_EMPTY_IS_NULL",
            input: b"\n\n\n\n",
            options: CSV_EMPTY_IS_NULL,
            expected: test12_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test12b",
            options_label: "CSV_REPALL_NL",
            input: b"\n\n\n\n",
            options: CSV_REPALL_NL,
            expected: test12b_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test12b",
            options_label: "CSV_REPALL_NL | CSV_EMPTY_IS_NULL",
            input: b"\n\n\n\n",
            options: CSV_REPALL_NL | CSV_EMPTY_IS_NULL,
            expected: test12b_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test13",
            options_label: "0",
            input: b"\"abc\"",
            options: 0,
            expected: test13_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test14",
            options_label: "0",
            input: b"1, 2, 3,\n\r\n  \"4\", \r,",
            options: 0,
            expected: test14_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test14",
            options_label: "CSV_STRICT",
            input: b"1, 2, 3,\n\r\n  \"4\", \r,",
            options: CSV_STRICT,
            expected: test14_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test15",
            options_label: "0",
            input: b"1, 2, 3,\n\r\n  \"4\", \r\"\"",
            options: 0,
            expected: test15_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test15",
            options_label: "CSV_STRICT",
            input: b"1, 2, 3,\n\r\n  \"4\", \r\"\"",
            options: CSV_STRICT,
            expected: test15_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test16",
            options_label: "0",
            input: b"\"1\",\"2\",\" 3 ",
            options: 0,
            expected: test16_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test16",
            options_label: "CSV_STRICT",
            input: b"\"1\",\"2\",\" 3 ",
            options: CSV_STRICT,
            expected: test16_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test16b",
            options_label: "CSV_STRICT | CSV_STRICT_FINI",
            input: b"\"1\",\"2\",\" 3 ",
            options: CSV_STRICT | CSV_STRICT_FINI,
            expected: vec![col(b"1"), col(b"2")],
            expect_error: true,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test17",
            options_label: "0",
            input: b" a\0b\0c ",
            options: 0,
            expected: test17_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test17",
            options_label: "CSV_STRICT",
            input: b" a\0b\0c ",
            options: CSV_STRICT,
            expected: test17_expected.clone(),
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test17",
            options_label: "CSV_STRICT | CSV_EMPTY_IS_NULL",
            input: b" a\0b\0c ",
            options: CSV_STRICT | CSV_EMPTY_IS_NULL,
            expected: test17_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "test19",
            options_label: "CSV_EMPTY_IS_NULL",
            input: b"  , \"\" ,",
            options: CSV_EMPTY_IS_NULL,
            expected: test19_expected,
            expect_error: false,
            delimiter: CSV_COMMA,
            quote: CSV_QUOTE,
            space_fn: None,
            term_fn: None,
        },
        Case {
            upstream_case: "custom01",
            options_label: "0",
            input: b"'''a;b''';;' '''' ';''''' ';' ''''';''''''",
            options: 0,
            expected: custom01_expected,
            expect_error: false,
            delimiter: b';',
            quote: b'\'',
            space_fn: None,
            term_fn: None,
        },
    ];

    for case in &cases {
        run_case(case);
    }
}

#[test]
fn translated_original_writer_cases() {
    let mut buffer = vec![0; b"abc".len() * 2 + 2];
    let actual_len = write_to_buffer(&mut buffer, b"abc");
    assert_eq!(actual_len, 5);
    assert_eq!(&buffer[..actual_len], b"\"abc\"");

    let mut buffer = vec![0; b"\"\"\"\"\"\"\"\"".len() * 2 + 2];
    let actual_len = write_to_buffer(&mut buffer, b"\"\"\"\"\"\"\"\"");
    assert_eq!(actual_len, 18);
    assert_eq!(
        &buffer[..actual_len],
        b"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\""
    );

    let mut buffer = vec![0; b"abc".len() * 2 + 2];
    let actual_len = write_to_buffer_with_quote(&mut buffer, b"abc", b'\'');
    assert_eq!(actual_len, 5);
    assert_eq!(&buffer[..actual_len], b"'abc'");

    let mut buffer = vec![0; b"''''''''".len() * 2 + 2];
    let actual_len = write_to_buffer_with_quote(&mut buffer, b"''''''''", b'\'');
    assert_eq!(actual_len, 18);
    assert_eq!(&buffer[..actual_len], b"''''''''''''''''''");
}

#[test]
fn public_configuration_round_trip() {
    let mut parser = Parser::new(CSV_STRICT);
    parser.set_delimiter(CSV_TAB);
    parser.set_quote(CSV_SPACE);
    assert_eq!(parser.options(), CSV_STRICT);
    assert_eq!(parser.delimiter(), CSV_TAB);
    assert_eq!(parser.quote(), CSV_SPACE);

    parser.set_options(CSV_REPALL_NL | CSV_EMPTY_IS_NULL);
    assert_eq!(parser.options(), CSV_REPALL_NL | CSV_EMPTY_IS_NULL);
}
