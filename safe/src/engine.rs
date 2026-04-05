use alloc::vec::Vec;

use crate::{
    CSV_APPEND_NULL, CSV_COMMA, CSV_CR, CSV_EINVALID, CSV_EMPTY_IS_NULL, CSV_ENOMEM, CSV_EPARSE,
    CSV_ETOOBIG, CSV_LF, CSV_QUOTE, CSV_REPALL_NL, CSV_SPACE, CSV_STRICT, CSV_STRICT_FINI,
    CSV_SUCCESS, CSV_TAB, END_OF_INPUT,
};

pub(crate) const DEFAULT_BLOCK_SIZE: usize = 128;
pub(crate) type BytePredicate = fn(u8) -> bool;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Success,
    Parse,
    NoMemory,
    TooBig,
}

impl Error {
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => CSV_SUCCESS,
            Self::Parse => CSV_EPARSE,
            Self::NoMemory => CSV_ENOMEM,
            Self::TooBig => CSV_ETOOBIG,
        }
    }
}

pub const fn strerror(code: u8) -> &'static str {
    match code {
        CSV_SUCCESS => "success",
        CSV_EPARSE => "error parsing data while strict checking enabled",
        CSV_ENOMEM => "memory exhausted while increasing buffer size",
        CSV_ETOOBIG => "data size too large",
        CSV_EINVALID..=u8::MAX => "invalid status code",
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParserState {
    ROW_NOT_BEGUN,
    FIELD_NOT_BEGUN,
    FIELD_BEGUN,
    FIELD_MIGHT_HAVE_ENDED,
}

pub(crate) trait EntryBuffer {
    fn len(&self) -> usize;
    fn try_resize(&mut self, new_len: usize) -> bool;
    fn set_byte(&mut self, index: usize, byte: u8);
    fn field_bytes(&self, len: usize) -> &[u8];
    fn free(&mut self);
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VecBuffer {
    bytes: Vec<u8>,
}

impl VecBuffer {
    pub(crate) const fn new() -> Self {
        Self { bytes: Vec::new() }
    }
}

impl EntryBuffer for VecBuffer {
    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn try_resize(&mut self, new_len: usize) -> bool {
        if new_len > self.bytes.len() {
            let additional = new_len - self.bytes.len();
            if self.bytes.try_reserve_exact(additional).is_err() {
                return false;
            }
        }
        self.bytes.resize(new_len, 0);
        true
    }

    fn set_byte(&mut self, index: usize, byte: u8) {
        self.bytes[index] = byte;
    }

    fn field_bytes(&self, len: usize) -> &[u8] {
        &self.bytes[..len]
    }

    fn free(&mut self) {
        self.bytes = Vec::new();
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ParserEngine<B> {
    pub(crate) pstate: ParserState,
    pub(crate) quoted: bool,
    pub(crate) spaces: usize,
    pub(crate) entry_pos: usize,
    status: Error,
    options: u8,
    quote_char: u8,
    delim_char: u8,
    is_space: Option<BytePredicate>,
    is_term: Option<BytePredicate>,
    blk_size: usize,
    pub(crate) buffer: B,
}

impl ParserEngine<VecBuffer> {
    pub(crate) fn new(options: u8) -> Self {
        Self::with_buffer(options, VecBuffer::new())
    }
}

impl<B: EntryBuffer> ParserEngine<B> {
    pub(crate) fn with_buffer(options: u8, buffer: B) -> Self {
        Self {
            pstate: ParserState::ROW_NOT_BEGUN,
            quoted: false,
            spaces: 0,
            entry_pos: 0,
            status: Error::Success,
            options,
            quote_char: CSV_QUOTE,
            delim_char: CSV_COMMA,
            is_space: None,
            is_term: None,
            blk_size: DEFAULT_BLOCK_SIZE,
            buffer,
        }
    }

    pub(crate) const fn error(&self) -> Error {
        self.status
    }

    pub(crate) const fn options(&self) -> u8 {
        self.options
    }

    pub(crate) fn set_options(&mut self, options: u8) {
        self.options = options;
    }

    pub(crate) fn set_delimiter(&mut self, delimiter: u8) {
        self.delim_char = delimiter;
    }

    pub(crate) fn set_quote(&mut self, quote: u8) {
        self.quote_char = quote;
    }

    pub(crate) const fn delimiter(&self) -> u8 {
        self.delim_char
    }

    pub(crate) const fn quote(&self) -> u8 {
        self.quote_char
    }

    pub(crate) fn set_space_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.is_space = predicate;
    }

    pub(crate) fn set_term_predicate(&mut self, predicate: Option<BytePredicate>) {
        self.is_term = predicate;
    }

    pub(crate) fn set_block_size(&mut self, size: usize) {
        self.blk_size = size;
    }

    pub(crate) fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn free(&mut self) {
        self.buffer.free();
    }

    fn commit_state(&mut self, quoted: bool, pstate: ParserState, spaces: usize, entry_pos: usize) {
        self.quoted = quoted;
        self.pstate = pstate;
        self.spaces = spaces;
        self.entry_pos = entry_pos;
    }

    fn increase_buffer(&mut self) -> Result<(), Error> {
        let mut to_add = self.blk_size;

        if self.buffer.len() >= usize::MAX - to_add {
            to_add = usize::MAX - self.buffer.len();
        }

        if to_add == 0 {
            self.status = Error::TooBig;
            return Err(Error::TooBig);
        }

        while !self.buffer.try_resize(self.buffer.len() + to_add) {
            to_add /= 2;
            if to_add == 0 {
                self.status = Error::NoMemory;
                return Err(Error::NoMemory);
            }
        }

        Ok(())
    }

    fn is_space_byte(is_space: Option<BytePredicate>, byte: u8) -> bool {
        is_space.map_or(byte == CSV_SPACE || byte == CSV_TAB, |predicate| {
            predicate(byte)
        })
    }

    fn is_term_byte(is_term: Option<BytePredicate>, byte: u8) -> bool {
        is_term.map_or(byte == CSV_CR || byte == CSV_LF, |predicate| {
            predicate(byte)
        })
    }

    fn submit_char(&mut self, entry_pos: &mut usize, byte: u8) {
        self.buffer.set_byte(*entry_pos, byte);
        *entry_pos += 1;
    }

    fn submit_field<F>(
        &mut self,
        quoted: &mut bool,
        pstate: &mut ParserState,
        spaces: &mut usize,
        entry_pos: &mut usize,
        field_cb: &mut F,
    ) where
        F: FnMut(Option<&[u8]>),
    {
        if !*quoted {
            *entry_pos -= *spaces;
        }

        if self.options & CSV_APPEND_NULL != 0 {
            self.buffer.set_byte(*entry_pos, 0);
        }

        if self.options & CSV_EMPTY_IS_NULL != 0 && !*quoted && *entry_pos == 0 {
            field_cb(None);
        } else {
            field_cb(Some(self.buffer.field_bytes(*entry_pos)));
        }

        *pstate = ParserState::FIELD_NOT_BEGUN;
        *entry_pos = 0;
        *quoted = false;
        *spaces = 0;
    }

    fn submit_row<F>(
        &mut self,
        quoted: &mut bool,
        pstate: &mut ParserState,
        spaces: &mut usize,
        entry_pos: &mut usize,
        row_cb: &mut F,
        term: i32,
    ) where
        F: FnMut(i32),
    {
        row_cb(term);
        *pstate = ParserState::ROW_NOT_BEGUN;
        *entry_pos = 0;
        *quoted = false;
        *spaces = 0;
    }

    pub(crate) fn parse<F1, F2>(
        &mut self,
        input: &[u8],
        field_cb: &mut F1,
        row_cb: &mut F2,
    ) -> usize
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        let mut pos = 0usize;
        let delim = self.delim_char;
        let quote = self.quote_char;
        let is_space = self.is_space;
        let is_term = self.is_term;
        let mut quoted = self.quoted;
        let mut pstate = self.pstate;
        let mut spaces = self.spaces;
        let mut entry_pos = self.entry_pos;

        if self.buffer.len() == 0 && pos < input.len() && self.increase_buffer().is_err() {
            self.commit_state(quoted, pstate, spaces, entry_pos);
            return pos;
        }

        while pos < input.len() {
            let limit = if self.options & CSV_APPEND_NULL != 0 {
                self.buffer.len() - 1
            } else {
                self.buffer.len()
            };

            if entry_pos == limit && self.increase_buffer().is_err() {
                self.commit_state(quoted, pstate, spaces, entry_pos);
                return pos;
            }

            let byte = input[pos];
            pos += 1;

            match pstate {
                ParserState::ROW_NOT_BEGUN | ParserState::FIELD_NOT_BEGUN => {
                    if Self::is_space_byte(is_space, byte) && byte != delim {
                        continue;
                    }

                    if Self::is_term_byte(is_term, byte) {
                        if pstate == ParserState::FIELD_NOT_BEGUN {
                            self.submit_field(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                field_cb,
                            );
                            self.submit_row(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                row_cb,
                                i32::from(byte),
                            );
                        } else if self.options & CSV_REPALL_NL != 0 {
                            self.submit_row(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                row_cb,
                                i32::from(byte),
                            );
                        }
                        continue;
                    }

                    if byte == delim {
                        self.submit_field(
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            field_cb,
                        );
                    } else if byte == quote {
                        pstate = ParserState::FIELD_BEGUN;
                        quoted = true;
                    } else {
                        pstate = ParserState::FIELD_BEGUN;
                        quoted = false;
                        self.submit_char(&mut entry_pos, byte);
                    }
                }
                ParserState::FIELD_BEGUN => {
                    if byte == quote {
                        if quoted {
                            self.submit_char(&mut entry_pos, byte);
                            pstate = ParserState::FIELD_MIGHT_HAVE_ENDED;
                        } else {
                            if self.options & CSV_STRICT != 0 {
                                self.status = Error::Parse;
                                self.commit_state(quoted, pstate, spaces, entry_pos);
                                return pos - 1;
                            }
                            self.submit_char(&mut entry_pos, byte);
                            spaces = 0;
                        }
                    } else if byte == delim {
                        if quoted {
                            self.submit_char(&mut entry_pos, byte);
                        } else {
                            self.submit_field(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                field_cb,
                            );
                        }
                    } else if Self::is_term_byte(is_term, byte) {
                        if quoted {
                            self.submit_char(&mut entry_pos, byte);
                        } else {
                            self.submit_field(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                field_cb,
                            );
                            self.submit_row(
                                &mut quoted,
                                &mut pstate,
                                &mut spaces,
                                &mut entry_pos,
                                row_cb,
                                i32::from(byte),
                            );
                        }
                    } else if !quoted && Self::is_space_byte(is_space, byte) {
                        self.submit_char(&mut entry_pos, byte);
                        spaces += 1;
                    } else {
                        self.submit_char(&mut entry_pos, byte);
                        spaces = 0;
                    }
                }
                ParserState::FIELD_MIGHT_HAVE_ENDED => {
                    if byte == delim {
                        entry_pos -= spaces + 1;
                        self.submit_field(
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            field_cb,
                        );
                    } else if Self::is_term_byte(is_term, byte) {
                        entry_pos -= spaces + 1;
                        self.submit_field(
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            field_cb,
                        );
                        self.submit_row(
                            &mut quoted,
                            &mut pstate,
                            &mut spaces,
                            &mut entry_pos,
                            row_cb,
                            i32::from(byte),
                        );
                    } else if Self::is_space_byte(is_space, byte) {
                        self.submit_char(&mut entry_pos, byte);
                        spaces += 1;
                    } else if byte == quote {
                        if spaces != 0 {
                            if self.options & CSV_STRICT != 0 {
                                self.status = Error::Parse;
                                self.commit_state(quoted, pstate, spaces, entry_pos);
                                return pos - 1;
                            }
                            spaces = 0;
                            self.submit_char(&mut entry_pos, byte);
                        } else {
                            pstate = ParserState::FIELD_BEGUN;
                        }
                    } else {
                        if self.options & CSV_STRICT != 0 {
                            self.status = Error::Parse;
                            self.commit_state(quoted, pstate, spaces, entry_pos);
                            return pos - 1;
                        }
                        pstate = ParserState::FIELD_BEGUN;
                        spaces = 0;
                        self.submit_char(&mut entry_pos, byte);
                    }
                }
            }
        }

        self.commit_state(quoted, pstate, spaces, entry_pos);
        pos
    }

    pub(crate) fn finish<F1, F2>(&mut self, field_cb: &mut F1, row_cb: &mut F2) -> Result<(), Error>
    where
        F1: FnMut(Option<&[u8]>),
        F2: FnMut(i32),
    {
        let mut quoted = self.quoted;
        let mut pstate = self.pstate;
        let mut spaces = self.spaces;
        let mut entry_pos = self.entry_pos;

        if self.pstate == ParserState::FIELD_BEGUN
            && self.quoted
            && self.options & CSV_STRICT != 0
            && self.options & CSV_STRICT_FINI != 0
        {
            self.status = Error::Parse;
            return Err(Error::Parse);
        }

        match self.pstate {
            ParserState::FIELD_MIGHT_HAVE_ENDED => {
                entry_pos -= spaces + 1;
                self.submit_field(
                    &mut quoted,
                    &mut pstate,
                    &mut spaces,
                    &mut entry_pos,
                    field_cb,
                );
                self.submit_row(
                    &mut quoted,
                    &mut pstate,
                    &mut spaces,
                    &mut entry_pos,
                    row_cb,
                    END_OF_INPUT,
                );
            }
            ParserState::FIELD_NOT_BEGUN | ParserState::FIELD_BEGUN => {
                self.submit_field(
                    &mut quoted,
                    &mut pstate,
                    &mut spaces,
                    &mut entry_pos,
                    field_cb,
                );
                self.submit_row(
                    &mut quoted,
                    &mut pstate,
                    &mut spaces,
                    &mut entry_pos,
                    row_cb,
                    END_OF_INPUT,
                );
            }
            ParserState::ROW_NOT_BEGUN => {}
        }

        self.spaces = 0;
        self.quoted = false;
        self.entry_pos = 0;
        self.status = Error::Success;
        self.pstate = ParserState::ROW_NOT_BEGUN;
        Ok(())
    }
}

pub(crate) fn write_size_with_quote(src: &[u8], quote: u8) -> usize {
    let escaped_quotes = src.iter().filter(|&&byte| byte == quote).count();
    write_size_from_stats(src.len(), escaped_quotes)
}

pub(crate) fn quoted_bytes(src: &[u8], quote: u8) -> QuotedBytes<'_> {
    QuotedBytes {
        src,
        quote,
        pos: 0,
        emitted_open: false,
        pending_escape: false,
        emitted_close: false,
    }
}

pub(crate) fn write_to_buffer_with_quote(dest: &mut [u8], src: &[u8], quote: u8) -> usize {
    let mut chars = 0usize;

    for byte in quoted_bytes(src, quote) {
        if dest.len() > chars {
            dest[chars] = byte;
        }
        chars = chars.saturating_add(1);
    }
    chars
}

pub(crate) const fn write_size_from_stats(src_size: usize, escaped_quotes: usize) -> usize {
    let chars = 1usize.saturating_add(src_size.saturating_add(escaped_quotes));
    chars.saturating_add(1)
}

pub(crate) struct QuotedBytes<'a> {
    src: &'a [u8],
    quote: u8,
    pos: usize,
    emitted_open: bool,
    pending_escape: bool,
    emitted_close: bool,
}

impl Iterator for QuotedBytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.emitted_open {
            self.emitted_open = true;
            return Some(self.quote);
        }

        if self.pending_escape {
            self.pending_escape = false;
            return Some(self.quote);
        }

        if self.pos < self.src.len() {
            let byte = self.src[self.pos];
            self.pos += 1;
            if byte == self.quote {
                self.pending_escape = true;
            }
            return Some(byte);
        }

        if !self.emitted_close {
            self.emitted_close = true;
            return Some(self.quote);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::{write_size_from_stats, EntryBuffer, Error, ParserEngine, ParserState};
    use crate::{CSV_COMMA, CSV_QUOTE};

    #[derive(Debug, Default)]
    struct TestBuffer {
        len: usize,
        bytes: Vec<u8>,
        max_success_len: Option<usize>,
        attempts: usize,
    }

    impl TestBuffer {
        fn with_max_success_len(max_success_len: Option<usize>) -> Self {
            Self {
                len: 0,
                bytes: Vec::new(),
                max_success_len,
                attempts: 0,
            }
        }

        fn with_virtual_len(len: usize) -> Self {
            Self {
                len,
                bytes: Vec::new(),
                max_success_len: None,
                attempts: 0,
            }
        }
    }

    impl EntryBuffer for TestBuffer {
        fn len(&self) -> usize {
            self.len
        }

        fn try_resize(&mut self, new_len: usize) -> bool {
            self.attempts += 1;
            if self
                .max_success_len
                .is_some_and(|max_success_len| new_len > max_success_len)
            {
                return false;
            }
            if new_len > self.bytes.len() {
                self.bytes.resize(new_len, 0);
            } else {
                self.bytes.truncate(new_len);
            }
            self.len = new_len;
            true
        }

        fn set_byte(&mut self, index: usize, byte: u8) {
            self.bytes[index] = byte;
        }

        fn field_bytes(&self, len: usize) -> &[u8] {
            &self.bytes[..len]
        }

        fn free(&mut self) {
            self.len = 0;
            self.bytes.clear();
        }
    }

    #[test]
    fn quote_heavy_growth_failure_is_bounded_and_reports_enomem() {
        // relevant_cves.json carries rgamble/libcsv#29 as the quote-heavy growth note.
        let input = br#"ABC|jkkdf|1664550195943489|28|0|"wxyz.th|"wxyz.th|::|||17301"#;
        let mut engine = ParserEngine::with_buffer(0, TestBuffer::with_max_success_len(Some(16)));
        let mut fields = Vec::new();
        let mut rows = Vec::new();

        engine.set_delimiter(b'|');
        engine.set_quote(CSV_QUOTE);
        engine.set_block_size(16);

        let consumed = engine.parse(
            input,
            &mut |field| fields.push(field.map(|bytes| bytes.to_vec())),
            &mut |term| rows.push(term),
        );

        assert_eq!(consumed, 26);
        assert_eq!(engine.error(), Error::NoMemory);
        assert!(rows.is_empty());
        assert_eq!(fields, vec![Some(b"ABC".to_vec()), Some(b"jkkdf".to_vec())]);
        assert_eq!(engine.buffer.attempts, 6);
    }

    #[test]
    fn toobig_is_reported_without_real_allocation() {
        let mut engine = ParserEngine::with_buffer(0, TestBuffer::with_virtual_len(usize::MAX));

        engine.pstate = ParserState::FIELD_BEGUN;
        engine.entry_pos = usize::MAX;
        engine.set_block_size(1);

        let consumed = engine.parse(b"x", &mut |_| {}, &mut |_| {});

        assert_eq!(consumed, 0);
        assert_eq!(engine.error(), Error::TooBig);
        assert_eq!(engine.buffer.attempts, 0);
    }

    #[test]
    fn writer_size_saturates_at_usize_max() {
        assert_eq!(write_size_from_stats(usize::MAX - 1, 1), usize::MAX);
        assert_eq!(write_size_from_stats(usize::MAX, usize::MAX), usize::MAX);
    }

    #[test]
    fn growth_retry_halves_requested_size_before_succeeding() {
        let mut engine = ParserEngine::with_buffer(0, TestBuffer::with_max_success_len(Some(12)));
        let mut fields = Vec::new();

        engine.set_delimiter(CSV_COMMA);
        engine.set_quote(CSV_QUOTE);
        engine.set_block_size(8);

        let consumed = engine.parse(
            br#""abcdefghij""#,
            &mut |field| fields.push(field.map(|bytes| bytes.to_vec())),
            &mut |_| {},
        );

        assert_eq!(consumed, br#""abcdefghij""#.len());
        assert_eq!(
            engine.finish(
                &mut |field| fields.push(field.map(|bytes| bytes.to_vec())),
                &mut |_| {}
            ),
            Ok(())
        );
        assert_eq!(fields, vec![Some(b"abcdefghij".to_vec())]);
        assert_eq!(engine.error(), Error::Success);
        assert_eq!(engine.buffer_size(), 12);
        assert_eq!(engine.buffer.attempts, 3);
    }
}
