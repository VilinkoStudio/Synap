use memchr::memchr2;

pub fn sanitize_search_text(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        let Some(offset) = memchr2(b'd', b'!', &bytes[i..]) else {
            output.extend_from_slice(&bytes[i..]);
            break;
        };
        let candidate = i + offset;

        if candidate > i {
            output.extend_from_slice(&bytes[i..candidate]);
        }

        if bytes[candidate..].starts_with(b"data:image/") {
            let mut j = candidate;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b')' {
                j += 1;
            }

            if !output.is_empty() && !output.last().is_some_and(|b| b.is_ascii_whitespace()) {
                output.push(b' ');
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
                    if !output.is_empty() && !output.last().is_some_and(|b| b.is_ascii_whitespace())
                    {
                        output.push(b' ');
                    }
                    i = url_end;
                    continue;
                }
            }
        }

        output.push(bytes[candidate]);
        i = candidate + 1;
    }

    String::from_utf8(output)
        .unwrap_or_else(|_| content.to_string())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
