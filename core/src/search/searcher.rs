use crate::search::types::{Searchable, TextMatchRange};
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Matcher, Nucleo, Utf32String,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 词法命中的匹配方式。
///
/// `Contiguous` 表示查询文本在条目中以连续子串出现；`Fuzzy` 表示仅按字符顺序匹配，
/// 中间允许存在间隔。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FuzzyMatchKind {
    Fuzzy,
    Contiguous,
}

/// 单条匹配结果
#[derive(Debug, Clone)]
pub struct MatchItem<Id> {
    /// 匹配到的文档 ID
    pub id: Id,
    /// nucleo 打分，越高越匹配
    pub score: u32,
    /// 本次词法命中的匹配方式
    pub match_kind: FuzzyMatchKind,
    /// 相对原始文本的 UTF-16 匹配区间，用于前端高亮
    pub match_ranges: Vec<TextMatchRange>,
}

/// 搜索返回值
#[derive(Debug, Clone)]
pub struct SearchOutput<Id> {
    /// 按分数降序排列的匹配结果
    pub items: Vec<MatchItem<Id>>,
    /// 是否已处理完全部候选项
    pub is_complete: bool,
    /// 当前总匹配数（可能大于 items.len()，因为 limit 截断）
    pub total_matched: u32,
}

/// 存储在 nucleo 内部的条目
#[derive(Clone)]
struct IndexEntry<Id: Clone + Send + Sync + 'static> {
    id: Id,
    text: String,
    source_ranges: Vec<TextMatchRange>,
}

/// 泛型模糊检索器
pub struct FuzzyIndex<T: Searchable> {
    nucleo: Mutex<Nucleo<IndexEntry<T::Id>>>,
}

impl<T: Searchable> FuzzyIndex<T> {
    /// 创建新的模糊搜索索引
    pub fn new() -> Self {
        let nucleo = Nucleo::new(
            Config::DEFAULT,
            Arc::new(|| {}),
            None, // 自动选择线程数
            1,    // 单列搜索
        );

        Self {
            nucleo: Mutex::new(nucleo),
        }
    }

    /// 注入单条文档
    pub fn insert(&self, doc: T) {
        let search_text = doc.get_search_text_with_ranges();
        let entry = IndexEntry {
            id: doc.get_id(),
            text: search_text.text,
            source_ranges: search_text.source_ranges,
        };

        let nucleo = self.nucleo.lock().unwrap();
        let injector = nucleo.injector();
        injector.push(entry, |item, columns| {
            columns[0] = Utf32String::from(item.text.as_str());
        });
    }

    /// 批量注入
    pub fn insert_batch(&self, docs: impl Iterator<Item = T>) {
        let nucleo = self.nucleo.lock().unwrap();
        let injector = nucleo.injector();

        for doc in docs {
            let search_text = doc.get_search_text_with_ranges();
            let entry = IndexEntry {
                id: doc.get_id(),
                text: search_text.text,
                source_ranges: search_text.source_ranges,
            };
            injector.push(entry, |item, columns| {
                columns[0] = Utf32String::from(item.text.as_str());
            });
        }
    }

    /// 执行搜索
    ///
    /// # 参数
    /// - `query`: 模糊搜索文本
    /// - `limit`: 最多返回条数
    /// - `timeout`: 超时时间
    ///   - `None` → 阻塞直到全部匹配完成
    ///   - `Some(duration)` → 最多等这么久，返回已有的部分结果
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        timeout: Option<Duration>,
    ) -> SearchOutput<T::Id> {
        let mut nucleo = self.nucleo.lock().unwrap();

        nucleo
            .pattern
            .reparse(0, query, CaseMatching::Smart, Normalization::Smart, false);

        let deadline = timeout.map(|d| Instant::now() + d);
        let mut is_complete = false;

        loop {
            let status = nucleo.tick(10);

            if !status.running || !status.changed {
                is_complete = true;
                break;
            }

            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    // 超时，带走当前已有结果
                    break;
                }
            }
        }

        let snapshot = nucleo.snapshot();
        let total_matched = snapshot.matched_item_count();
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut items: Vec<MatchItem<T::Id>> = snapshot
            .matched_items(..)
            .map(|item| {
                let mut indices = Vec::new();
                let score = snapshot
                    .pattern()
                    .column_pattern(0)
                    .indices(item.matcher_columns[0].slice(..), &mut matcher, &mut indices)
                    .unwrap_or(0);

                MatchItem {
                    id: item.data.id.clone(),
                    score,
                    match_kind: if !query.is_empty() && item.data.text.contains(query) {
                        FuzzyMatchKind::Contiguous
                    } else {
                        FuzzyMatchKind::Fuzzy
                    },
                    match_ranges: match_ranges(&item.data.source_ranges, &mut indices),
                }
            })
            .collect();

        // Nucleo's snapshot is already score-sorted. A stable sort preserves that order
        // while guaranteeing that an exact contiguous occurrence always outranks a gapped match.
        items.sort_by(|a, b| {
            b.match_kind
                .cmp(&a.match_kind)
                .then_with(|| b.score.cmp(&a.score))
        });
        items.truncate(limit);

        SearchOutput {
            items,
            is_complete,
            total_matched,
        }
    }

    /// 清空索引
    pub fn clear(&self) {
        let mut nucleo = self.nucleo.lock().unwrap();
        nucleo.restart(true);
    }

    /// 当前索引总条目数
    pub fn total_items(&self) -> u32 {
        let nucleo = self.nucleo.lock().unwrap();
        nucleo.snapshot().item_count()
    }
}

fn match_ranges(source_ranges: &[TextMatchRange], indices: &mut Vec<u32>) -> Vec<TextMatchRange> {
    indices.sort_unstable();
    indices.dedup();

    let mut ranges: Vec<TextMatchRange> = Vec::new();
    for index in indices.iter().copied() {
        let Some(range) = source_ranges.get(index as usize).copied() else {
            continue;
        };

        if let Some(previous) = ranges.last_mut() {
            if previous.end == range.start {
                previous.end = range.end;
                continue;
            }
        }
        ranges.push(range);
    }

    ranges
}

impl<T: Searchable> Default for FuzzyIndex<T> {
    fn default() -> Self {
        Self::new()
    }
}
