/// 原始文本中的 UTF-16 半开区间 `[start, end)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMatchRange {
    pub start: u32,
    pub end: u32,
}

/// 供索引匹配的文本，以及每个索引字符在原始文本中的位置。
#[derive(Debug, Clone)]
pub struct SearchText {
    pub text: String,
    pub source_ranges: Vec<TextMatchRange>,
}

impl SearchText {
    pub fn identity(text: String) -> Self {
        let mut source_ranges = Vec::with_capacity(text.chars().count());
        let mut offset = 0;

        for ch in text.chars() {
            let start = offset;
            offset += ch.len_utf16() as u32;
            source_ranges.push(TextMatchRange { start, end: offset });
        }

        Self {
            text,
            source_ranges,
        }
    }

    pub fn mapped(text: String, source_ranges: Vec<TextMatchRange>) -> Self {
        debug_assert_eq!(text.chars().count(), source_ranges.len());
        Self {
            text,
            source_ranges,
        }
    }
}

/// 能力特征：任何实现了这个 Trait 的数据，都可以被极速搜索
pub trait Searchable: Clone + Send + Sync + 'static {
    /// 唯一标识符的类型
    type Id: Clone + Send + Sync + 'static;

    /// 获取主键
    fn get_id(&self) -> Self::Id;

    /// 获取需要被检索的纯文本
    fn get_search_text(&self) -> String;

    /// 获取索引文本及其到原始文本的偏移映射。
    fn get_search_text_with_ranges(&self) -> SearchText {
        SearchText::identity(self.get_search_text())
    }
}
