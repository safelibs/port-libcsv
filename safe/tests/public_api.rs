use std::{
    cell::RefCell,
    io::{self, Write},
};

use csv::{
    fwrite, fwrite_with_quote, strerror, write, write_to_buffer, write_with_quote, Error, Parser,
    CSV_APPEND_NULL, CSV_COMMA, CSV_EINVALID, CSV_EMPTY_IS_NULL, CSV_ENOMEM, CSV_EPARSE,
    CSV_ETOOBIG, CSV_QUOTE, CSV_SUCCESS, END_OF_INPUT,
};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Field(Option<Vec<u8>>),
    Row(i32),
}

fn field(bytes: &[u8]) -> Event {
    Event::Field(Some(bytes.to_vec()))
}

fn null_field() -> Event {
    Event::Field(None)
}

fn row(term: i32) -> Event {
    Event::Row(term)
}

fn parse_and_finish(parser: &mut Parser, input: &[u8]) -> (usize, Result<(), Error>, Vec<Event>) {
    let events = RefCell::new(Vec::new());
    let (consumed, finish_result) = {
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

        (consumed, finish_result)
    };

    (consumed, finish_result, events.into_inner())
}

fn is_underscore(byte: u8) -> bool {
    byte == b'_'
}

fn is_semicolon(byte: u8) -> bool {
    byte == b';'
}

#[derive(Default)]
struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic write failure"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn default_configuration_and_error_strings_are_public() {
    let parser = Parser::default();
    assert_eq!(parser.options(), 0);
    assert_eq!(parser.delimiter(), CSV_COMMA);
    assert_eq!(parser.quote(), CSV_QUOTE);
    assert_eq!(parser.error(), Error::Success);

    assert_eq!(Error::Success.code(), CSV_SUCCESS);
    assert_eq!(Error::Parse.code(), CSV_EPARSE);
    assert_eq!(Error::NoMemory.code(), CSV_ENOMEM);
    assert_eq!(Error::TooBig.code(), CSV_ETOOBIG);

    assert_eq!(strerror(CSV_SUCCESS), "success");
    assert_eq!(
        strerror(CSV_EPARSE),
        "error parsing data while strict checking enabled"
    );
    assert_eq!(
        strerror(CSV_ENOMEM),
        "memory exhausted while increasing buffer size"
    );
    assert_eq!(strerror(CSV_ETOOBIG), "data size too large");
    assert_eq!(strerror(CSV_EINVALID), "invalid status code");
    assert_eq!(strerror(u8::MAX), "invalid status code");
}

#[test]
fn append_null_reserves_an_extra_buffer_byte() {
    let mut parser = Parser::new(CSV_APPEND_NULL);
    parser.set_block_size(1);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"a");

    assert_eq!(consumed, 1);
    assert_eq!(finish_result, Ok(()));
    assert_eq!(events, vec![field(b"a"), row(END_OF_INPUT)]);
    assert_eq!(parser.buffer_size(), 2);

    let mut parser = Parser::new(0);
    parser.set_block_size(1);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"a");

    assert_eq!(consumed, 1);
    assert_eq!(finish_result, Ok(()));
    assert_eq!(events, vec![field(b"a"), row(END_OF_INPUT)]);
    assert_eq!(parser.buffer_size(), 1);
}

#[test]
fn custom_space_and_term_predicates_affect_empty_detection() {
    let mut parser = Parser::new(CSV_EMPTY_IS_NULL);
    parser.set_space_predicate(Some(is_underscore));
    parser.set_term_predicate(Some(is_semicolon));

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"__,_,ab_;");

    assert_eq!(consumed, b"__,_,ab_;".len());
    assert_eq!(finish_result, Ok(()));
    assert_eq!(
        events,
        vec![
            null_field(),
            null_field(),
            field(b"ab"),
            row(i32::from(b';')),
        ]
    );
}

#[test]
fn finish_preserves_buffer_and_parser_can_be_reused() {
    let mut parser = Parser::new(0);
    parser.set_block_size(2);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"abcd");

    assert_eq!(consumed, 4);
    assert_eq!(finish_result, Ok(()));
    assert_eq!(events, vec![field(b"abcd"), row(END_OF_INPUT)]);
    assert_eq!(parser.buffer_size(), 4);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"xy\n");

    assert_eq!(consumed, 3);
    assert_eq!(finish_result, Ok(()));
    assert_eq!(events, vec![field(b"xy"), row(i32::from(b'\n'))]);
    assert_eq!(parser.buffer_size(), 4);

    parser.free();
    assert_eq!(parser.buffer_size(), 0);

    let (consumed, finish_result, events) = parse_and_finish(&mut parser, b"z");

    assert_eq!(consumed, 1);
    assert_eq!(finish_result, Ok(()));
    assert_eq!(events, vec![field(b"z"), row(END_OF_INPUT)]);
    assert_eq!(parser.buffer_size(), 2);
}

#[test]
fn zero_block_size_reports_toobig_before_consuming_input() {
    let mut parser = Parser::new(0);
    parser.set_block_size(0);

    let events = RefCell::new(Vec::new());
    let mut on_field = |value: Option<&[u8]>| {
        events
            .borrow_mut()
            .push(Event::Field(value.map(|bytes| bytes.to_vec())));
    };
    let mut on_row = |term: i32| {
        events.borrow_mut().push(Event::Row(term));
    };

    let consumed = parser.parse(b"x", &mut on_field, &mut on_row);

    assert_eq!(consumed, 0);
    assert!(events.into_inner().is_empty());
    assert_eq!(parser.error(), Error::TooBig);
    assert_eq!(strerror(parser.error().code()), "data size too large");
}

#[test]
fn writer_helpers_match_buffer_and_io_variants() {
    let expected = b"\"a\"\"b\"";
    let mut truncated = [0_u8; 4];

    let actual_len = write_to_buffer(&mut truncated, b"a\"b");
    assert_eq!(actual_len, expected.len());
    assert_eq!(&truncated, &expected[..truncated.len()]);
    assert_eq!(write_to_buffer(&mut [], b"a\"b"), expected.len());

    assert_eq!(write(b"a\"b"), expected);
    assert_eq!(write_with_quote(b"a'b", b'\''), b"'a''b'");

    let mut output = Vec::new();
    fwrite(&mut output, b"a\"b").unwrap();
    assert_eq!(output, expected);

    let mut output = Vec::new();
    fwrite_with_quote(&mut output, b"a'b", b'\'').unwrap();
    assert_eq!(output, b"'a''b'");
}

#[test]
fn fwrite_propagates_io_errors() {
    let err = fwrite(&mut FailingWriter, b"abc").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);

    let err = fwrite_with_quote(&mut FailingWriter, b"abc", b'\'').unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::Other);
}
