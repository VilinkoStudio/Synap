use crate::nlp::tag::{NlpDocument, NlpTagIndex};
use std::time::{Duration, Instant};

fn doc(id: &str, content: &str, tags: &[&str]) -> NlpDocument {
    NlpDocument::new(
        id,
        content,
        tags.iter().map(|tag| (*tag).to_string()).collect(),
    )
}

fn sample_docs() -> Vec<NlpDocument> {
    vec![
        doc(
            "1",
            "Rust ownership and lifetimes for async services",
            &["rust", "async", "backend"],
        ),
        doc(
            "2",
            "Tokio runtime, future polling and async scheduling",
            &["rust", "async"],
        ),
        doc(
            "3",
            "Operating system memory management reading notes",
            &["os", "reading"],
        ),
        doc("4", "操作系统 内存管理 读书笔记", &["os", "读书"]),
        doc("5", "数据库索引与查询优化实践", &["database", "backend"]),
        doc(
            "6",
            "API design, request validation and service boundaries",
            &["backend", "api"],
        ),
    ]
}

fn make_perf_docs(count: usize) -> Vec<NlpDocument> {
    (0..count)
        .map(|idx| {
            let tags = vec![
                format!("topic_{}", idx % 32),
                format!("cluster_{}", idx % 12),
                format!("lang_{}", idx % 5),
            ];
            NlpDocument::new(
                format!("doc-{idx}"),
                format!(
                    "document {idx} covers rust async indexing memory graph topic {} cluster {}",
                    idx % 32,
                    idx % 12
                ),
                tags,
            )
        })
        .collect()
}

#[test]
fn suggest_tags_matches_english_topics() {
    let mut index = NlpTagIndex::new();
    index.build(sample_docs());

    let suggestions = index.suggest_tags("tokio async ownership", 3);
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();

    assert!(tags.contains(&"rust"));
    assert!(tags.contains(&"async"));
}

#[test]
fn suggest_tags_matches_chinese_topics() {
    let mut index = NlpTagIndex::new();
    index.build(sample_docs());

    let suggestions = index.suggest_tags("操作系统的内存管理整理", 3);
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();

    assert!(tags.contains(&"os"));
    assert!(tags.contains(&"读书"));
}

#[test]
fn suggest_tags_ignores_markdown_media_noise() {
    let mut index = NlpTagIndex::new();
    index.build(vec![doc("1", "hello rust async world", &["rust", "async"])]);

    let suggestions = index.suggest_tags(
        "![cover](data:image/png;base64,AAAA) rust async data:image/jpeg;base64,BBBB",
        3,
    );

    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0].tag, "async");
    assert!(suggestions.iter().any(|item| item.tag == "rust"));
}

#[test]
fn build_and_incremental_upsert_produce_same_ranking() {
    let docs = sample_docs();

    let mut built = NlpTagIndex::new();
    built.build(docs.clone());

    let mut incremental = NlpTagIndex::new();
    for doc in docs {
        incremental.upsert(doc);
    }

    let built_tags: Vec<String> = built
        .suggest_tags("memory management reading", 4)
        .into_iter()
        .map(|item| item.tag)
        .collect();
    let incremental_tags: Vec<String> = incremental
        .suggest_tags("memory management reading", 4)
        .into_iter()
        .map(|item| item.tag)
        .collect();

    assert_eq!(built_tags, incremental_tags);
}

#[test]
fn upsert_replaces_old_document_contribution() {
    let mut index = NlpTagIndex::new();
    index.upsert(doc("1", "tokio future runtime", &["async"]));

    let initial = index.suggest_tags("tokio runtime", 2);
    assert!(initial.iter().any(|item| item.tag == "async"));

    index.upsert(doc("1", "sql index join planner", &["database"]));

    let updated = index.suggest_tags("sql planner", 2);
    assert!(updated.iter().any(|item| item.tag == "database"));

    let old_query = index.suggest_tags("tokio runtime", 2);
    assert!(!old_query.iter().any(|item| item.tag == "async"));
}

#[test]
fn remove_reverses_document_contribution() {
    let mut index = NlpTagIndex::new();
    index.build(vec![
        doc("1", "tokio future runtime", &["async"]),
        doc("2", "structured logging and tracing", &["ops"]),
    ]);

    assert!(index.remove("1"));
    let suggestions = index.suggest_tags("tokio runtime", 3);

    assert!(suggestions.iter().all(|item| item.tag != "async"));
    assert_eq!(index.document_count(), 1);
}

#[test]
fn cooccurrence_boost_surfaces_related_tag() {
    let mut index = NlpTagIndex::new();
    index.build(vec![
        doc(
            "1",
            "tokio runtime future scheduling",
            &["async", "runtime"],
        ),
        doc(
            "2",
            "async task orchestration and queue handling",
            &["async", "runtime"],
        ),
        doc("3", "future combinators and async lifetimes", &["async"]),
    ]);

    let suggestions = index.suggest_tags("future runtime scheduling", 3);
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();

    assert!(tags.contains(&"async"));
    assert!(tags.contains(&"runtime"));
}

#[test]
fn inactive_or_empty_docs_do_not_enter_index() {
    let mut index = NlpTagIndex::new();
    index.upsert(doc("1", "   ", &["rust"]));
    index.upsert(doc("2", "tokio runtime", &[]));
    index.upsert(doc("3", "tokio runtime", &["async"]).with_active(false));

    assert_eq!(index.document_count(), 0);
    assert!(index.suggest_tags("tokio", 3).is_empty());
}

#[test]
#[ignore = "performance guard for local runs"]
fn test_build_realtime_small() {
    let docs = make_perf_docs(1_000);
    let mut index = NlpTagIndex::new();
    let started = Instant::now();

    index.build(docs);

    let elapsed = started.elapsed();
    eprintln!("build 1000 docs in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(3));
    assert!(index.document_count() >= 1_000);
}

#[test]
#[ignore = "performance guard for local runs"]
fn test_upsert_realtime_small() {
    let docs = make_perf_docs(500);
    let mut index = NlpTagIndex::new();
    index.build(docs);

    let started = Instant::now();
    for idx in 0..200 {
        index.upsert(NlpDocument::new(
            format!("hot-{idx}"),
            format!("realtime update {idx} for rust async service indexing"),
            vec!["rust".into(), "hot".into(), format!("bucket_{}", idx % 10)],
        ));
    }

    let elapsed = started.elapsed();
    eprintln!("200 upserts in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(2));
}

#[test]
#[ignore = "performance guard for local runs"]
fn test_suggest_realtime_small() {
    let docs = make_perf_docs(1_000);
    let mut index = NlpTagIndex::new();
    index.build(docs);

    let started = Instant::now();
    for _ in 0..200 {
        let results = index.suggest_tags("rust async indexing memory graph", 5);
        assert!(!results.is_empty());
    }

    let elapsed = started.elapsed();
    eprintln!("200 suggestions in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(2));
}
#[test]
fn suggest_tags_respects_limit_parameter() {
    let mut index = NlpTagIndex::new();
    index.build(vec![doc(
        "1",
        "comprehensive guide to rust tokio async futures memory threading",
        &["rust", "async", "tokio", "memory", "threading", "guide"],
    )]);

    // 请求限制为 2
    let suggestions_limit_2 = index.suggest_tags("rust tokio async memory", 2);
    assert_eq!(suggestions_limit_2.len(), 2);

    // 请求限制为 0，应该返回空
    let suggestions_limit_0 = index.suggest_tags("rust tokio", 0);
    assert!(suggestions_limit_0.is_empty());

    // 请求限制大于可用标签数
    let suggestions_limit_10 = index.suggest_tags("rust tokio async memory", 10);
    assert!(suggestions_limit_10.len() <= 6); // 总共只有 6 个 tag
}

#[test]
fn suggest_tags_is_case_insensitive() {
    let mut index = NlpTagIndex::new();
    index.build(vec![
        doc("1", "Rust Async Programming", &["rust", "async"]),
        doc("2", "DATABASE SYSTEM DESIGN", &["database", "design"]),
    ]);

    // 使用全大写或混合大小写查询
    let suggestions = index.suggest_tags("rUsT aSyNc DaTaBaSe", 5);
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();

    assert!(tags.contains(&"rust"));
    assert!(tags.contains(&"async"));
    assert!(tags.contains(&"database"));
}

#[test]
fn empty_or_whitespace_query_returns_no_suggestions() {
    let mut index = NlpTagIndex::new();
    index.build(sample_docs());

    // 空字符串
    assert!(index.suggest_tags("", 5).is_empty());

    // 纯空白字符（空格、制表符、换行）
    assert!(index.suggest_tags("   \n\t  ", 5).is_empty());
}

#[test]
fn remove_non_existent_document_returns_false() {
    let mut index = NlpTagIndex::new();
    index.build(sample_docs());
    let initial_count = index.document_count();

    // 删除不存在的 ID
    let removed = index.remove("non_existent_id_999");

    assert!(!removed);
    assert_eq!(index.document_count(), initial_count);
}

#[test]
fn upsert_is_idempotent() {
    let mut index = NlpTagIndex::new();
    let my_doc = doc("1", "idempotent async rust testing", &["rust", "testing"]);

    // 重复插入同一个文档多次
    for _ in 0..5 {
        index.upsert(my_doc.clone());
    }

    assert_eq!(index.document_count(), 1);

    let suggestions = index.suggest_tags("idempotent rust", 3);
    assert!(!suggestions.is_empty());

    // 确保分数不会因为重复插入而被异常放大（如果有绝对分数校验可以在此添加）
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();
    assert!(tags.contains(&"rust"));
    assert!(tags.contains(&"testing"));
}

#[test]
fn exact_tag_match_boost() {
    let mut index = NlpTagIndex::new();
    index.build(vec![
        doc("1", "some random content", &["docker"]),
        doc("2", "deep dive into container runtimes", &["container"]),
    ]);

    // 用户查询词本身就是一个 tag，但并没有在文档 content 中大量出现
    // 优秀的 NLP Tag 引擎应该能通过词汇网络或反向索引推荐出 "docker"
    let suggestions = index.suggest_tags("docker", 3);
    let tags: Vec<&str> = suggestions.iter().map(|item| item.tag.as_str()).collect();

    assert!(tags.contains(&"docker"));
}

#[test]
fn index_handles_special_characters_gracefully() {
    let mut index = NlpTagIndex::new();
    // 包含大量符号、标点、emoji
    index.build(vec![
        doc(
            "1",
            "C++ & Rust: 🚀 A match made in heaven! (v1.0.0)",
            &["cpp", "rust"],
        ),
        doc(
            "2",
            "{[\"json\": true]} => regex parsing*",
            &["json", "regex"],
        ),
    ]);

    let suggestions1 = index.suggest_tags("C++ 🚀 match", 3);
    let tags1: Vec<&str> = suggestions1.iter().map(|item| item.tag.as_str()).collect();
    assert!(tags1.contains(&"cpp") || tags1.contains(&"rust"));

    let suggestions2 = index.suggest_tags("json regex *", 3);
    let tags2: Vec<&str> = suggestions2.iter().map(|item| item.tag.as_str()).collect();
    assert!(tags2.contains(&"json"));
}
