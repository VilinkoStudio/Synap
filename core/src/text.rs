use memchr::memchr2;

use crate::search::types::TextMatchRange;

pub fn sanitize_search_text(content: &str) -> String {
    sanitize_search_text_with_ranges(content).0
}

pub fn sanitize_search_text_with_ranges(content: &str) -> (String, Vec<TextMatchRange>) {
    let bytes = content.as_bytes();
    let utf16_offsets = utf16_offsets_by_byte(content);
    let mut output = String::with_capacity(bytes.len());
    let mut ranges = Vec::with_capacity(content.chars().count());
    let mut i = 0;

    while i < bytes.len() {
        let Some(offset) = memchr2(b'd', b'!', &bytes[i..]) else {
            append_source_slice(content, i, bytes.len(), &utf16_offsets, &mut output, &mut ranges);
            break;
        };
        let candidate = i + offset;

        if candidate > i {
            append_source_slice(content, i, candidate, &utf16_offsets, &mut output, &mut ranges);
        }

        if bytes[candidate..].starts_with(b"data:image/") {
            let mut j = candidate;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b')' {
                j += 1;
            }

            if !output.is_empty() && !output.chars().last().is_some_and(char::is_whitespace) {
                append_synthetic_space(
                    utf16_offsets[candidate],
                    utf16_offsets[j],
                    &mut output,
                    &mut ranges,
                );
            }
            i = j;
            continue;
        }

        if bytes[candidate] == b'!' && candidate + 1 < bytes.len() && bytes[candidate + 1] == b'[' {
            let mut alt_end = candidate + 2;
            while alt_end < bytes.len() && bytes[alt_end] != b']' {
                alt_end += 1;
            }

            if alt_end + 1 < bytes.len() && bytes[alt_end + 1] == b'(' {
                let mut url_end = alt_end + 2;
                let mut depth = 1usize;

                while url_end < bytes.len() {
                    match bytes[url_end] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                url_end += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    url_end += 1;
                }

                if depth == 0 {
                    if !output.is_empty() && !output.chars().last().is_some_and(char::is_whitespace) {
                        append_synthetic_space(
                            utf16_offsets[candidate],
                            utf16_offsets[url_end],
                            &mut output,
                            &mut ranges,
                        );
                    }
                    i = url_end;
                    continue;
                }
            }
        }

        append_source_slice(
            content,
            candidate,
            candidate + 1,
            &utf16_offsets,
            &mut output,
            &mut ranges,
        );
        i = candidate + 1;
    }

    collapse_whitespace(&output, &ranges)
}

fn utf16_offsets_by_byte(content: &str) -> Vec<u32> {
    let mut offsets = vec![0; content.len() + 1];
    let mut offset = 0;

    for (byte_index, ch) in content.char_indices() {
        offsets[byte_index] = offset;
        offset += ch.len_utf16() as u32;
        offsets[byte_index + ch.len_utf8()] = offset;
    }

    offsets
}

fn append_source_slice(
    content: &str,
    start: usize,
    end: usize,
    utf16_offsets: &[u32],
    output: &mut String,
    ranges: &mut Vec<TextMatchRange>,
) {
    for (relative_byte_index, ch) in content[start..end].char_indices() {
        let byte_index = start + relative_byte_index;
        output.push(ch);
        ranges.push(TextMatchRange {
            start: utf16_offsets[byte_index],
            end: utf16_offsets[byte_index + ch.len_utf8()],
        });
    }
}

fn append_synthetic_space(
    start: u32,
    end: u32,
    output: &mut String,
    ranges: &mut Vec<TextMatchRange>,
) {
    output.push(' ');
    ranges.push(TextMatchRange { start, end });
}

fn collapse_whitespace(
    text: &str,
    ranges: &[TextMatchRange],
) -> (String, Vec<TextMatchRange>) {
    let mut output = String::with_capacity(text.len());
    let mut output_ranges = Vec::with_capacity(ranges.len());
    let mut whitespace_range: Option<TextMatchRange> = None;

    for (ch, range) in text.chars().zip(ranges.iter().copied()) {
        if ch.is_whitespace() {
            whitespace_range = Some(match whitespace_range {
                Some(previous) => TextMatchRange {
                    start: previous.start,
                    end: range.end,
                },
                None => range,
            });
            continue;
        }

        if let Some(range) = whitespace_range.take() {
            if !output.is_empty() {
                output.push(' ');
                output_ranges.push(range);
            }
        }

        output.push(ch);
        output_ranges.push(range);
    }

    (output, output_ranges)
}
