//! Tag metadata: `$`-prefixed special tags and user-tag escaping.
//!
//! # Wire protocol (stored inside `note.tags`)
//!
//! - Canonical metadata: `$kind(args)` — e.g. `$color(#ff0000)`
//! - Legacy color (read-only): `$RRGGBB` (exactly 6 hex digits)
//! - Escaped user tag starting with `$`: `$$...` (never metadata)
//! - Ordinary tags: stored as-is
//!
//! # Public / API form
//!
//! - Display tags never include metadata
//! - User-facing `$foo` unescapes from `$$foo`
//! - Known metadata is exposed as typed fields (e.g. `NoteDTO.color = "#rrggbb"`)
//! - Unknown `$kind(...)` is preserved in storage on rewrite, not shown as display tags
//!
//! Write path always emits the canonical form. Legacy color is upgraded lazily on edit.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Color value
// ---------------------------------------------------------------------------

/// Parsed note color as 24-bit RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl NoteColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `#rrggbb` (lowercase) for DTO / frontend.
    pub fn to_css_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Canonical storage tag: `$color(#rrggbb)`.
    pub fn to_storage_tag(&self) -> String {
        format!("$color({})", self.to_css_hex())
    }

    /// Parse public color strings: `#rrggbb`, `rrggbb`, `$color(...)`, or legacy `$rrggbb`.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(color) = parse_color_storage_tag(trimmed) {
            return Some(color);
        }
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
        parse_hex6(hex)
    }
}

// ---------------------------------------------------------------------------
// Metadata bag (known fields + opaque unknowns)
// ---------------------------------------------------------------------------

/// Opaque `$kind(...)` entry preserved for forward-compat.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnknownMeta {
    pub kind: String,
    /// Full original storage tag, e.g. `$pin()` or `$priority(1)`.
    pub raw: String,
}

/// Metadata extracted from storage tags. Add known fields here as kinds grow.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMetadata {
    /// First color wins (canonical `$color(...)` preferred over legacy).
    pub color: Option<NoteColor>,
    /// Unknown `$kind(...)` entries kept so rewrite does not drop future kinds.
    pub unknown: Vec<UnknownMeta>,
}

impl TagMetadata {
    pub fn is_empty(&self) -> bool {
        self.color.is_none() && self.unknown.is_empty()
    }

    pub fn with_color(color: Option<NoteColor>) -> Self {
        Self {
            color,
            unknown: Vec::new(),
        }
    }

    pub fn set_color(&mut self, color: Option<NoteColor>) {
        self.color = color;
    }
}

/// Split storage tags into user-facing display tags + metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitTags {
    pub display_tags: Vec<String>,
    pub metadata: TagMetadata,
}

// ---------------------------------------------------------------------------
// Generic `$kind(args)` tokenizer
// ---------------------------------------------------------------------------

/// A parsed `$kind(args)` form. `args` is the raw interior (may be empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindTag<'a> {
    pub kind: &'a str,
    pub args: &'a str,
}

/// Parse `$kind(args)` — kind is `[A-Za-z][A-Za-z0-9_]*`, args unvalidated.
pub fn parse_kind_tag(tag: &str) -> Option<KindTag<'_>> {
    let trimmed = tag.trim();
    let body = trimmed.strip_prefix('$')?;
    // Escaped user tags / not kind form.
    if body.starts_with('$') {
        return None;
    }
    let open = body.find('(')?;
    if !body.ends_with(')') {
        return None;
    }
    let kind = &body[..open];
    if !is_valid_kind_name(kind) {
        return None;
    }
    let args = &body[open + 1..body.len() - 1];
    Some(KindTag { kind, args })
}

fn is_valid_kind_name(kind: &str) -> bool {
    let mut chars = kind.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

/// True when `tag` is metadata (canonical kind, legacy color, or unknown kind form).
/// Escaped user tags `$$...` are never metadata.
pub fn is_metadata_tag(tag: &str) -> bool {
    let trimmed = tag.trim();
    if trimmed.is_empty() || !trimmed.starts_with('$') || trimmed.starts_with("$$") {
        return false;
    }
    parse_kind_tag(trimmed).is_some() || parse_legacy_color_tag(trimmed).is_some()
}

/// True when `tag` carries color (canonical or legacy).
pub fn is_color_tag(tag: &str) -> bool {
    parse_color_storage_tag(tag).is_some()
}

/// Parse color from storage: `$color(#rrggbb)` / `$color(rrggbb)` or legacy `$RRGGBB`.
pub fn parse_color_storage_tag(tag: &str) -> Option<NoteColor> {
    let trimmed = tag.trim();
    if let Some(kind) = parse_kind_tag(trimmed) {
        if kind.kind.eq_ignore_ascii_case("color") {
            return NoteColor::parse(kind.args);
        }
        return None;
    }
    parse_legacy_color_tag(trimmed)
}

/// Legacy Android form: `$` + exactly 6 hex digits.
pub fn parse_legacy_color_tag(tag: &str) -> Option<NoteColor> {
    let trimmed = tag.trim();
    let hex = trimmed.strip_prefix('$')?;
    if hex.starts_with('$') {
        return None;
    }
    // Must not look like `$kind(...)`
    if hex.contains('(') {
        return None;
    }
    parse_hex6(hex)
}

// ---------------------------------------------------------------------------
// Escape
// ---------------------------------------------------------------------------

/// Escape a user-facing tag into storage form.
/// Tags that begin with `$` are prefixed with an extra `$` so they are not metadata.
pub fn escape_user_tag(display: &str) -> Option<String> {
    let trimmed = display.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('$') {
        Some(format!("${trimmed}"))
    } else {
        Some(trimmed.to_owned())
    }
}

/// Reverse of [`escape_user_tag`].
pub fn unescape_user_tag(storage: &str) -> String {
    let trimmed = storage.trim();
    if let Some(rest) = trimmed.strip_prefix("$$") {
        format!("${rest}")
    } else {
        trimmed.to_owned()
    }
}

// ---------------------------------------------------------------------------
// Split / merge
// ---------------------------------------------------------------------------

/// Split storage tags: drop metadata from display list, unescape user tags, extract metadata.
///
/// Color: first wins (canonical preferred if both present — scan order).
/// Unknown kinds: preserved in order, deduped by raw string.
pub fn split_storage_tags<I, S>(storage_tags: I) -> SplitTags
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut display_tags = Vec::new();
    let mut seen_display = HashSet::new();
    let mut color = None;
    let mut unknown = Vec::new();
    let mut seen_unknown = HashSet::new();

    for raw in storage_tags {
        let tag = raw.as_ref().trim();
        if tag.is_empty() {
            continue;
        }

        // Color (canonical or legacy)
        if let Some(parsed) = parse_color_storage_tag(tag) {
            if color.is_none() {
                color = Some(parsed);
            }
            continue;
        }

        // Other `$kind(...)` → opaque metadata (not display)
        if let Some(kind) = parse_kind_tag(tag) {
            let entry = UnknownMeta {
                kind: kind.kind.to_ascii_lowercase(),
                raw: tag.to_owned(),
            };
            if seen_unknown.insert(entry.raw.clone()) {
                unknown.push(entry);
            }
            continue;
        }

        // Legacy-looking `$......` that isn't color: if it starts with `$` and isn't `$$`,
        // treat non-kind forms as user tags only when escaped; bare `$foo` without kind
        // syntax is treated as display after unescape path below.
        let display = unescape_user_tag(tag);
        if display.is_empty() {
            continue;
        }
        if seen_display.insert(display.clone()) {
            display_tags.push(display);
        }
    }

    SplitTags {
        display_tags,
        metadata: TagMetadata { color, unknown },
    }
}

/// Merge display tags + metadata into storage tags.
///
/// - Display tags are escaped.
/// - Known color is written as `$color(#rrggbb)` only (never legacy).
/// - Unknown metadata raw tags are re-emitted as-is.
/// - Color tags accidentally present in display input are ignored after escape only if they
///   parse as color metadata; user `$aabbcc` becomes `$$aabbcc` and is kept.
pub fn merge_storage_tags<I, S>(display_tags: I, metadata: &TagMetadata) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for raw in display_tags {
        let Some(escaped) = escape_user_tag(raw.as_ref()) else {
            continue;
        };
        // Drop if client still passes a raw metadata form as a "tag".
        if is_metadata_tag(&escaped) {
            continue;
        }
        if seen.insert(escaped.clone()) {
            result.push(escaped);
        }
    }

    if let Some(color) = metadata.color {
        let storage = color.to_storage_tag();
        if seen.insert(storage.clone()) {
            result.push(storage);
        }
    }

    for entry in &metadata.unknown {
        if seen.insert(entry.raw.clone()) {
            result.push(entry.raw.clone());
        }
    }

    result
}

/// Normalize a public color input into a color value, or clear when `None`/empty.
pub fn parse_color_input(color: Option<&str>) -> Result<Option<NoteColor>, TagMetadataError> {
    match color {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => {
            NoteColor::parse(value)
                .map(Some)
                .ok_or_else(|| TagMetadataError::InvalidColor {
                    value: value.to_owned(),
                })
        }
    }
}

/// Replace color on existing storage tags while preserving display tags + unknown metadata.
///
/// Lazy upgrade: legacy `$RRGGBB` is dropped and rewritten as `$color(#..)` when color is set.
pub fn with_color_metadata(storage_tags: &[String], color: Option<NoteColor>) -> Vec<String> {
    let mut split = split_storage_tags(storage_tags.iter().map(String::as_str));
    split.metadata.set_color(color);
    merge_storage_tags(split.display_tags, &split.metadata)
}

/// Build storage tags from display tags + optional public color input.
/// Unknown metadata is empty (create/edit from client display tags only).
pub fn storage_tags_from_display_and_color(
    display_tags: impl IntoIterator<Item = impl AsRef<str>>,
    color: Option<&str>,
) -> Result<Vec<String>, TagMetadataError> {
    let color = parse_color_input(color)?;
    Ok(merge_storage_tags(
        display_tags,
        &TagMetadata::with_color(color),
    ))
}

/// Filter out metadata tags and unescape (for get_all_tags / search_tags public surfaces).
pub fn filter_public_tags<I, S>(tags: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for raw in tags {
        let tag = raw.as_ref().trim();
        if tag.is_empty() || is_metadata_tag(tag) {
            continue;
        }
        let display = unescape_user_tag(tag);
        if seen.insert(display.clone()) {
            out.push(display);
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagMetadataError {
    InvalidColor { value: String },
}

impl std::fmt::Display for TagMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidColor { value } => write!(f, "invalid note color: {value}"),
        }
    }
}

impl std::error::Error for TagMetadataError {}

fn parse_hex6(hex: &str) -> Option<NoteColor> {
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(NoteColor::new(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_canonical_roundtrip() {
        let color = NoteColor::new(0xff, 0x00, 0x80);
        assert_eq!(color.to_storage_tag(), "$color(#ff0080)");
        assert_eq!(color.to_css_hex(), "#ff0080");
        assert_eq!(NoteColor::parse("$color(#ff0080)"), Some(color));
        assert_eq!(NoteColor::parse("$color(ff0080)"), Some(color));
        assert_eq!(NoteColor::parse("#ff0080"), Some(color));
        assert_eq!(NoteColor::parse("ff0080"), Some(color));
    }

    #[test]
    fn legacy_color_still_readable() {
        let color = NoteColor::new(0xff, 0, 0);
        assert_eq!(parse_legacy_color_tag("$ff0000"), Some(color));
        assert_eq!(parse_legacy_color_tag("$FF0000"), Some(color));
        assert_eq!(NoteColor::parse("$ff0000"), Some(color));
        assert!(is_metadata_tag("$ff0000"));
        assert!(is_color_tag("$ff0000"));
    }

    #[test]
    fn rejects_invalid_color() {
        assert!(NoteColor::parse("").is_none());
        assert!(NoteColor::parse("#fff").is_none());
        assert!(NoteColor::parse("$gg0000").is_none());
        assert!(NoteColor::parse("$color(nope)").is_none());
        assert!(NoteColor::parse("red").is_none());
        assert!(matches!(
            parse_color_input(Some("nope")),
            Err(TagMetadataError::InvalidColor { .. })
        ));
    }

    #[test]
    fn kind_tag_tokenizer() {
        let k = parse_kind_tag("$color(#ff0000)").unwrap();
        assert_eq!(k.kind, "color");
        assert_eq!(k.args, "#ff0000");
        let p = parse_kind_tag("$pin()").unwrap();
        assert_eq!(p.kind, "pin");
        assert_eq!(p.args, "");
        assert!(parse_kind_tag("$ff0000").is_none());
        assert!(parse_kind_tag("$$price").is_none());
        assert!(parse_kind_tag("$1bad()").is_none());
    }

    #[test]
    fn escape_and_unescape_user_dollar_tags() {
        assert_eq!(escape_user_tag("rust").as_deref(), Some("rust"));
        assert_eq!(escape_user_tag("$price").as_deref(), Some("$$price"));
        assert_eq!(escape_user_tag("  $a  ").as_deref(), Some("$$a"));
        assert_eq!(escape_user_tag("").as_deref(), None);
        assert_eq!(unescape_user_tag("$$price"), "$price");
        assert_eq!(unescape_user_tag("rust"), "rust");
    }

    #[test]
    fn split_canonical_color_and_unknown() {
        let split = split_storage_tags(["rust", "$color(#ff0000)", "$$price", "$pin()", "work"]);
        assert_eq!(
            split.display_tags,
            vec!["rust".to_string(), "$price".to_string(), "work".to_string()]
        );
        assert_eq!(split.metadata.color, Some(NoteColor::new(0xff, 0, 0)));
        assert_eq!(split.metadata.unknown.len(), 1);
        assert_eq!(split.metadata.unknown[0].kind, "pin");
        assert_eq!(split.metadata.unknown[0].raw, "$pin()");
    }

    #[test]
    fn split_legacy_color() {
        let split = split_storage_tags(["$00ff00", "rust"]);
        assert_eq!(split.display_tags, vec!["rust".to_string()]);
        assert_eq!(split.metadata.color, Some(NoteColor::new(0, 0xff, 0)));
    }

    #[test]
    fn split_first_color_wins() {
        let split = split_storage_tags(["$color(#00ff00)", "$0000ff", "$color(#ffffff)"]);
        assert!(split.display_tags.is_empty());
        assert_eq!(split.metadata.color, Some(NoteColor::new(0, 0xff, 0)));
    }

    #[test]
    fn merge_writes_canonical_color_only() {
        let metadata = TagMetadata::with_color(Some(NoteColor::new(0, 0, 0xff)));
        let storage = merge_storage_tags(["rust", "$price"], &metadata);
        assert_eq!(
            storage,
            vec![
                "rust".to_string(),
                "$$price".to_string(),
                "$color(#0000ff)".to_string()
            ]
        );
        assert!(!storage.iter().any(|t| t == "$0000ff"));
    }

    #[test]
    fn merge_preserves_unknown_metadata() {
        let metadata = TagMetadata {
            color: Some(NoteColor::new(1, 2, 3)),
            unknown: vec![UnknownMeta {
                kind: "pin".into(),
                raw: "$pin()".into(),
            }],
        };
        let storage = merge_storage_tags(["a"], &metadata);
        assert_eq!(
            storage,
            vec![
                "a".to_string(),
                "$color(#010203)".to_string(),
                "$pin()".to_string()
            ]
        );
    }

    #[test]
    fn merge_escapes_color_shaped_user_tag() {
        let storage = merge_storage_tags(["rust", "$aabbcc"], &TagMetadata::default());
        assert_eq!(storage, vec!["rust".to_string(), "$$aabbcc".to_string()]);
    }

    #[test]
    fn with_color_upgrades_legacy() {
        let original = vec!["rust".into(), "$ff0000".into(), "$$keep".into()];
        let updated = with_color_metadata(&original, Some(NoteColor::new(0, 0xff, 0)));
        assert_eq!(
            updated,
            vec![
                "rust".to_string(),
                "$$keep".to_string(),
                "$color(#00ff00)".to_string()
            ]
        );
        let cleared = with_color_metadata(&original, None);
        assert_eq!(cleared, vec!["rust".to_string(), "$$keep".to_string()]);
    }

    #[test]
    fn with_color_preserves_unknown() {
        let original = vec!["$pin()".into(), "$ff0000".into(), "x".into()];
        let updated = with_color_metadata(&original, Some(NoteColor::new(0, 0, 1)));
        assert_eq!(
            updated,
            vec![
                "x".to_string(),
                "$color(#000001)".to_string(),
                "$pin()".to_string()
            ]
        );
    }

    #[test]
    fn filter_public_tags_drops_metadata() {
        let public =
            filter_public_tags(["rust", "$color(#ff0000)", "$ff0000", "$$price", "$pin()"]);
        assert_eq!(public, vec!["rust".to_string(), "$price".to_string()]);
    }

    #[test]
    fn merge_split_roundtrip_canonical() {
        let metadata = TagMetadata {
            color: Some(NoteColor::new(0xaa, 0xbb, 0xcc)),
            unknown: vec![UnknownMeta {
                kind: "pin".into(),
                raw: "$pin()".into(),
            }],
        };
        let storage = merge_storage_tags(["a", "$b", "c"], &metadata);
        let split = split_storage_tags(storage);
        assert_eq!(split.display_tags, vec!["a", "$b", "c"]);
        assert_eq!(split.metadata.color, metadata.color);
        assert_eq!(split.metadata.unknown, metadata.unknown);
    }

    #[test]
    fn escaped_dollar_tag_is_not_color() {
        assert!(!is_color_tag("$$ff0000"));
        assert!(!is_metadata_tag("$$ff0000"));
        assert_eq!(unescape_user_tag("$$ff0000"), "$ff0000");
        let split = split_storage_tags(["$$ff0000"]);
        assert_eq!(split.display_tags, vec!["$ff0000".to_string()]);
        assert!(split.metadata.color.is_none());
    }

    #[test]
    fn storage_from_display_and_color_helper() {
        let tags = storage_tags_from_display_and_color(["rust", "$x"], Some("#00ff00")).unwrap();
        assert_eq!(
            tags,
            vec![
                "rust".to_string(),
                "$$x".to_string(),
                "$color(#00ff00)".to_string()
            ]
        );
        let no_color = storage_tags_from_display_and_color(["a"], None).unwrap();
        assert_eq!(no_color, vec!["a".to_string()]);
    }
}
