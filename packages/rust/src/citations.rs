//! Inline citation extraction for Markdown content.

use std::{ops::Range, sync::LazyLock};

use pulldown_cmark::{Event, Parser, Tag};
use regex::bytes::Regex;
use serde::Serialize;

use crate::{LOCAL_ID_CHARSET, LOCAL_ID_MAX_LENGTH};

const MASK: u8 = b'\0';
const COMMENT_OPEN: &[u8] = b"<!--";
const COMMENT_CLOSE: &[u8] = b"-->";
const COMMENT_EMPTY: &[u8] = b"<!-->";
const COMMENT_DASH: &[u8] = b"<!--->";

static MARKER_RE: LazyLock<Regex> = LazyLock::new(|| {
    let id = format!(r"[{LOCAL_ID_CHARSET}]{{1,{LOCAL_ID_MAX_LENGTH}}}");
    let pattern = format!(r"\[cite:[ \t]*({id}(?:[ \t]*,[ \t]*{id})*)\]");
    // PANIC: the citation marker regex is built only from fixed syntax and a numeric catalog constant.
    #[expect(
        clippy::expect_used,
        reason = "compile-time citation regex must be valid"
    )]
    Regex::new(&pattern).expect("citation marker regex must compile")
});

/// One recognized inline citation marker, with source ids in written order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Citation {
    /// Source ids listed by the marker.
    pub ids: Vec<String>,
}

/// Extract one citation entry per recognized marker in Markdown source order.
///
/// Matching follows the OpenAKB v1 raw-source grammar: code blocks, code spans,
/// HTML blocks, and HTML comments are masked before citation markers are matched.
#[must_use]
pub fn extract_citations(markdown: &str) -> Vec<Citation> {
    let masked = mask_non_prose(markdown);
    MARKER_RE
        .captures_iter(&masked)
        .map(|captures| Citation {
            ids: split_ids(
                captures
                    .get(1)
                    .map_or(&[][..], |matched| matched.as_bytes()),
            ),
        })
        .collect()
}

fn split_ids(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == b',')
        .map(trim_horizontal_space)
        .map(String::from_utf8_lossy)
        .map(String::from)
        .collect()
}

fn trim_horizontal_space(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| *byte != b' ' && *byte != b'\t')
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| *byte != b' ' && *byte != b'\t')
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

fn mask_non_prose(markdown: &str) -> Vec<u8> {
    let source = normalize_source(markdown);
    let mut masked = source.as_bytes().to_vec();

    for (event, range) in Parser::new(&source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => mask_range(&mut masked, range),
            Event::Start(Tag::HtmlBlock) => mask_range(&mut masked, range),
            Event::Code(_) => mask_range(&mut masked, range),
            Event::Start(_) | Event::End(_) => {}
            _ => mask_comments(&source, &mut masked, range),
        }
    }

    masked
}

fn normalize_source(markdown: &str) -> String {
    let mut source = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                source.push('\n');
            }
            '\0' => source.push('\u{fffd}'),
            _ => source.push(ch),
        }
    }

    source
}

fn mask_range(masked: &mut [u8], range: Range<usize>) {
    for byte in &mut masked[range] {
        *byte = MASK;
    }
}

fn mask_comments(source: &str, masked: &mut [u8], range: Range<usize>) {
    let bytes = source.as_bytes();
    let mut index = range.start;

    while index < range.end {
        if starts_with_at(bytes, index, range.end, COMMENT_OPEN) {
            let Some(end) = comment_end(bytes, index, range.end) else {
                index += 1;
                continue;
            };
            mask_range(masked, index..end);
            index = end;
        } else {
            index += 1;
        }
    }
}

fn comment_end(bytes: &[u8], open_start: usize, end: usize) -> Option<usize> {
    if starts_with_at(bytes, open_start, end, COMMENT_EMPTY) {
        return Some(open_start + COMMENT_EMPTY.len());
    }
    if starts_with_at(bytes, open_start, end, COMMENT_DASH) {
        return Some(open_start + COMMENT_DASH.len());
    }

    find_bytes(bytes, open_start + COMMENT_OPEN.len(), end, COMMENT_CLOSE)
        .map(|close_start| close_start + COMMENT_CLOSE.len())
}

fn starts_with_at(bytes: &[u8], index: usize, end: usize, needle: &[u8]) -> bool {
    index + needle.len() <= end && bytes[index..index + needle.len()] == *needle
}

fn find_bytes(bytes: &[u8], start: usize, end: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || start + needle.len() > end {
        return None;
    }

    (start..=end - needle.len()).find(|index| bytes[*index..*index + needle.len()] == *needle)
}
