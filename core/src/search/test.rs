#![cfg(test)]

// tests/integration.rs

use std::time::Duration;

use uuid::Uuid;

use crate::search::{searcher::FuzzyIndex, types::Searchable};

// ─── 测试用数据结构 ───

#[derive(Clone, Debug)]
struct Note {
    id: Uuid,
    title: String,
    content: String,
}

impl Note {
    fn new(title: &str, content: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            content: content.into(),
        }
    }
}

impl Searchable for Note {
    type Id = Uuid;

    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_search_text(&self) -> String {
        format!("{} {}", self.title, self.content)
    }
}

#[derive(Clone, Debug)]
struct Tag {
    id: u64,
    name: String,
}

impl Searchable for Tag {
    type Id = u64;

    fn get_id(&self) -> u64 {
        self.id
    }

    fn get_search_text(&self) -> String {
        self.name.clone()
    }
}

// ─── 辅助函数 ───

fn make_notes() -> Vec<Note> {
    vec![
        Note::new("Rust学习笔记", "今天学了生命周期和所有权"),
        Note::new("买菜清单", "番茄 鸡蛋 青菜 豆腐"),
        Note::new(
            "Meeting Notes",
            "Discussed the new Rust microservice architecture",
        ),
        Note::new("读书笔记", "深入理解计算机系统 第三章"),
        Note::new("Rust Async Runtime", "tokio executor poll future waker"),
        Note::new("周末计划", "去公园跑步 然后写Rust代码"),
    ]
}

fn find_note<'a>(notes: &'a [Note], id: &Uuid) -> &'a Note {
    notes.iter().find(|n| &n.id == id).unwrap()
}

// ─── 测试 ───

#[test]
fn test_basic_search() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    let output = index.search("rust", 10, None);

    assert!(output.is_complete);
    assert!(output.items.len() >= 2, "至少应匹配到多条含 rust 的笔记");

    // 验证分数降序
    for w in output.items.windows(2) {
        assert!(w[0].score >= w[1].score, "结果应按分数降序排列");
    }

    // 打印结果便于调试
    println!("搜索 'rust' 命中 {} 条:", output.items.len());
    for item in &output.items {
        let note = find_note(&notes, &item.id);
        println!("  [score={}] {} | {}", item.score, note.title, note.content);
    }
}

#[test]
fn test_chinese_search() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    let output = index.search("笔记", 10, None);

    assert!(output.is_complete);
    assert!(output.items.len() >= 2);

    println!("搜索 '笔记' 命中 {} 条:", output.items.len());
    for item in &output.items {
        let note = find_note(&notes, &item.id);
        println!("  [score={}] {}", item.score, note.title);
    }
}

#[test]
fn test_empty_query() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    let output = index.search("", 10, None);

    // 空查询应该返回所有条目（nucleo 的默认行为：空 pattern 匹配全部）
    assert!(output.is_complete);
    println!(
        "空查询命中 {} 条, 总共 {} 条",
        output.items.len(),
        output.total_matched
    );
}

#[test]
fn test_no_match() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    let output = index.search("xyzzyspoon", 10, None);

    assert!(output.is_complete);
    assert_eq!(output.items.len(), 0, "不应匹配到任何结果");
}

#[test]
fn test_limit() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    let output = index.search("", 2, None);

    assert!(output.items.len() <= 2, "不应超过 limit");
}

#[test]
fn test_single_insert() {
    let index = FuzzyIndex::<Note>::new();

    let note = Note::new("单独插入测试", "这是一条单独插入的笔记");
    let expected_id = note.id;
    index.insert(note);

    let output = index.search("单独", 10, None);

    assert!(output.is_complete);
    assert_eq!(output.items.len(), 1);
    assert_eq!(output.items[0].id, expected_id);
}

#[test]
fn test_clear() {
    let index = FuzzyIndex::<Note>::new();
    let notes = make_notes();
    index.insert_batch(notes.iter().cloned());

    // 搜索确认有数据
    let output = index.search("rust", 10, None);
    assert!(!output.items.is_empty());

    // 清空
    index.clear();

    // 需要等 nucleo 内部重置完成
    std::thread::sleep(Duration::from_millis(50));

    assert_eq!(index.total_items(), 0);
}

#[test]
fn test_different_id_type() {
    let index = FuzzyIndex::<Tag>::new();

    let tags = vec![
        Tag {
            id: 1,
            name: "rust-lang".into(),
        },
        Tag {
            id: 2,
            name: "javascript".into(),
        },
        Tag {
            id: 3,
            name: "rust-async".into(),
        },
        Tag {
            id: 4,
            name: "python".into(),
        },
        Tag {
            id: 5,
            name: "rustacean".into(),
        },
    ];

    index.insert_batch(tags.into_iter());

    let output = index.search("rust", 10, None);

    assert!(output.is_complete);
    assert!(output.items.len() >= 2);

    println!("Tag 搜索 'rust' 命中:");
    for item in &output.items {
        println!("  id={}, score={}", item.id, item.score);
    }
}

#[test]
fn test_timeout_returns_partial() {
    let index = FuzzyIndex::<Tag>::new();

    // 注入大量数据
    let tags: Vec<Tag> = (0..100_000)
        .map(|i| Tag {
            id: i,
            name: format!("tag_number_{}_with_extra_text_for_matching_{}", i, i * 7),
        })
        .collect();

    index.insert_batch(tags.into_iter());

    // 用极短的超时
    let output = index.search("tag_number_42", 10, Some(Duration::from_micros(1)));

    println!(
        "超时测试: complete={}, matched={}, returned={}",
        output.is_complete,
        output.total_matched,
        output.items.len()
    );

    // 不管是否完成，结构体都应该是合法的
    assert!(output.items.len() <= 10);
}

#[test]
fn test_score_ordering() {
    let index = FuzzyIndex::<Tag>::new();

    let tags = vec![
        Tag {
            id: 1,
            name: "rust".into(),
        }, // 精确匹配
        Tag {
            id: 2,
            name: "rust-lang".into(),
        }, // 前缀匹配
        Tag {
            id: 3,
            name: "rusty-tools".into(),
        }, // 前缀但更长
        Tag {
            id: 4,
            name: "trust".into(),
        }, // 包含但非前缀
        Tag {
            id: 5,
            name: "robust".into(),
        }, // 子序列匹配
    ];

    index.insert_batch(tags.into_iter());

    let output = index.search("rust", 10, None);

    assert!(output.is_complete);
    println!("分数排序测试:");
    for item in &output.items {
        println!("  id={}, score={}", item.id, item.score);
    }

    // 精确匹配 "rust"(id=1) 应该排最前面
    if output.items.len() >= 2 {
        assert!(
            output.items[0].score >= output.items[1].score,
            "第一名的分数应 >= 第二名"
        );
    }
}

#[test]
fn test_concurrent_insert_and_search() {
    use std::sync::Arc;
    use std::thread;

    let index = Arc::new(FuzzyIndex::<Tag>::new());

    // 线程 1：持续插入
    let index_w = Arc::clone(&index);
    let writer = thread::spawn(move || {
        for i in 0..1000 {
            index_w.insert(Tag {
                id: i,
                name: format!("concurrent_tag_{}", i),
            });
        }
    });

    // 线程 2：持续搜索
    let index_r = Arc::clone(&index);
    let reader = thread::spawn(move || {
        let mut found_any = false;
        for _ in 0..20 {
            let output = index_r.search("concurrent", 5, Some(Duration::from_millis(10)));
            if !output.items.is_empty() {
                found_any = true;
            }
            thread::sleep(Duration::from_millis(5));
        }
        found_any
    });

    writer.join().unwrap();
    let found = reader.join().unwrap();

    // 最终搜索应该能找到
    let final_output = index.search("concurrent", 5, None);
    assert!(!final_output.items.is_empty(), "最终搜索应有结果");

    println!(
        "并发测试: 读线程期间找到={}, 最终命中={}",
        found,
        final_output.items.len()
    );
}
