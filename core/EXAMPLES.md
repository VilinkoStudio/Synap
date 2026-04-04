# Synap Core - 使用示例

## 目录

- [基础操作](#基础操作)
- [知识图谱](#知识图谱)
- [标签管理](#标签管理)
- [查询功能](#查询功能)
- [统计功能](#统计功能)
- [完整示例](#完整示例)

## 基础操作

### 打开数据库

```rust
use synap_core::SynapService;

// 文件数据库
let service = SynapService::open("my_notes.db")?;

// 内存数据库（测试用）
let service = SynapService::open_memory()?;
```

### 创建和读取笔记

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建笔记
let note = service.create_note("我的第一篇笔记".to_string())?;
println!("创建笔记: {}", note.id);
println!("内容: {}", note.content);

// 读取笔记
let retrieved = service.get_note(note.id)?;
println!("读取到的笔记: {}", retrieved.content);
```

### 更新和删除笔记

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;
let note = service.create_note("原始内容".to_string())?;

// 更新笔记
let updated = service.update_note(note.id, "更新后的内容".to_string())?;
println!("更新时间: {:?}", updated.updated_at);

// 软删除笔记
service.delete_note(note.id)?;

// 笔记仍然可以通过 ID 访问，但不会出现在列表中
let deleted_note = service.get_note(note.id)?;
assert!(deleted_note.deleted);

let all_notes = service.list_notes()?;
assert_eq!(all_notes.len(), 0); // 不包含已删除的笔记
```

## 知识图谱

### 建立笔记关系

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建多个笔记
let idea1 = service.create_note("基础概念".to_string())?;
let idea2 = service.create_note("衍生想法 A".to_string())?;
let idea3 = service.create_note("衍生想法 B".to_string())?;
let idea4 = service.create_note("更深入的想法".to_string())?;

// 建立 DAG 关系
service.link_notes(idea1.id, idea2.id)?;  // idea1 -> idea2
service.link_notes(idea1.id, idea3.id)?;  // idea1 -> idea3
service.link_notes(idea2.id, idea4.id)?;  // idea2 -> idea4

// 查询子节点（从这个想法衍生出了哪些想法）
let children = service.get_children(idea1.id)?;
println!("从 {} 衍生了 {} 个想法", idea1.id, children.len());
for child in children {
    println!("  - {}", child.content);
}

// 查询父节点（这个想法来自哪些想法）
let parents = service.get_parents(idea4.id)?;
println!("{} 来自 {} 个想法", idea4.id, parents.len());
for parent in parents {
    println!("  - {}", parent.content);
}
```

### 遍历知识图谱

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建知识树
let root = service.create_note("编程语言".to_string())?;
let rust = service.create_note("Rust".to_string())?;
let python = service.create_note("Python".to_string())?;
let ownership = service.create_note("所有权".to_string())?;
let decorators = service.create_note("装饰器".to_string())?;

service.link_notes(root.id, rust.id)?;
service.link_notes(root.id, python.id)?;
service.link_notes(rust.id, ownership.id)?;
service.link_notes(python.id, decorators.id)?;

// 获取完整图谱（无深度限制）
let graph = service.get_graph(root.id, 0)?;
println!("知识图谱:");
for (note, depth) in graph {
    let indent = "  ".repeat(depth);
    println!("{}- {}", indent, note.content);
}

// 输出:
// 知识图谱:
// - 编程语言
//   - Rust
//     - 所有权
//   - Python
//     - 装饰器
```

### 循环检测

```rust
use synap_core::{CoreError, SynapService};

let service = SynapService::open_memory()?;

let note1 = service.create_note("Note 1".to_string())?;
let note2 = service.create_note("Note 2".to_string())?;

service.link_notes(note1.id, note2.id)?;

// 尝试创建循环会失败
match service.link_notes(note2.id, note1.id) {
    Ok(_) => println!("不应该成功"),
    Err(CoreError::CycleDetected { .. }) => {
        println!("成功阻止了循环创建！");
    }
    Err(e) => println!("其他错误: {}", e),
}
```

## 标签管理

### 添加和删除标签

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;
let note = service.create_note("Rust 学习笔记".to_string())?;

// 添加标签
service.add_tag(note.id, "rust".to_string())?;
service.add_tag(note.id, "学习".to_string())?;
service.add_tag(note.id, "编程".to_string())?;

// 查看笔记的标签
let tags = service.get_tags_for_note(note.id)?;
println!("标签: {:?}", tags); // ["rust", "学习", "编程"]

// 删除标签
service.remove_tag(note.id, "学习")?;

let updated_tags = service.get_tags_for_note(note.id)?;
println!("更新后的标签: {:?}", updated_tags); // ["rust", "编程"]
```

### 按标签浏览

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建不同主题的笔记
let note1 = service.create_note("Rust 所有权系统".to_string())?;
let note2 = service.create_note("Python 装饰器".to_string())?;
let note3 = service.create_note("Rust 生命周期".to_string())?;

service.add_tag(note1.id, "rust".to_string())?;
service.add_tag(note1.id, "进阶".to_string())?;
service.add_tag(note2.id, "python".to_string())?;
service.add_tag(note3.id, "rust".to_string())?;

// 按标签查询
let rust_notes = service.get_notes_by_tag("rust", None, Some(20))?;
println!("Rust 相关笔记:");
for note in rust_notes {
    println!("  - {}", note.content);
}

// 获取所有标签
let all_tags = service.get_all_tags()?;
println!("\n所有标签:");
for tag in all_tags {
    let count = service.get_notes_by_tag(&tag, None, Some(20)).unwrap().len();
    println!("  - {} ({} 篇)", tag, count);
}
```

### 标签统计

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建大量笔记并添加标签
for i in 1..=10 {
    let note = service.create_note(format!("笔记 {}", i))?;
    if i % 2 == 0 {
        service.add_tag(note.id, "偶数".to_string())?;
    }
    if i % 3 == 0 {
        service.add_tag(note.id, "三的倍数".to_string())?;
    }
}

// 获取统计信息
let stats = service.get_stats()?;
println!("总笔记数: {}", stats.total_notes);
println!("总关系数: {}", stats.total_edges);
println!("热门标签:");
for (tag, count) in &stats.top_tags {
    println!("  - {}: {} 次", tag, count);
}
```

## 查询功能

### 文本搜索

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

service.create_note("Rust 编程语言".to_string())?;
service.create_note("Python 数据科学".to_string())?;
service.create_note("JavaScript Web 开发".to_string())?;

// 搜索包含特定关键词的笔记
let rust_results = service.search_notes("rust")?;
println!("找到 {} 篇 Rust 相关笔记", rust_results.len());

let web_results = service.search_notes("web")?;
println!("找到 {} 篇 Web 相关笔记", web_results.len());

// 搜索是大小写不敏感的
let results1 = service.search_notes("PYTHON")?;
let results2 = service.search_notes("python")?;
assert_eq!(results1.len(), results2.len());
```

### 分页浏览

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建 100 条笔记
for i in 1..=100 {
    service.create_note(format!("笔记 #{}", i))?;
}

// 分页查询（每页 10 条）
let page_size = 10;
let total_pages = (100 + page_size - 1) / page_size;

for page in 0..total_pages {
    let offset = page * page_size;
    let notes = service.list_notes_paginated(offset, page_size)?;

    println!("第 {} 页 ({} 条):", page + 1, notes.len());
    for note in notes {
        println!("  - {}", note.content);
    }
}
```

### 组合查询

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建带标签的笔记
for topic in &["rust", "python", "javascript"] {
    let note = service.create_note(format!("{} 教程", topic))?;
    service.add_tag(note.id, topic.to_string())?;
    service.add_tag(note.id, "教程".to_string())?;
}

// 查询带"教程"标签的笔记
let tutorial_notes = service.get_notes_by_tag("教程", None, Some(20))?;

// 在这些笔记中搜索特定内容
let results: Vec<_> = tutorial_notes
    .into_iter()
    .filter(|n| n.content.to_lowercase().contains("rust"))
    .collect();

println!("Rust 教程: {} 篇", results.len());
```

## 统计功能

### 获取数据库统计

```rust
use synap_core::SynapService;

let service = SynapService::open_memory()?;

// 创建一些笔记和关系
let note1 = service.create_note("Note 1".to_string())?;
let note2 = service.create_note("Note 2".to_string())?;
let note3 = service.create_note("Note 3".to_string())?;

service.add_tag(note1.id, "important".to_string())?;
service.add_tag(note2.id, "important".to_string())?;
service.add_tag(note3.id, "draft".to_string())?;

service.link_notes(note1.id, note2.id)?;
service.link_notes(note1.id, note3.id)?;

// 获取统计信息
let stats = service.get_stats()?;
println!("=== 数据库统计 ===");
println!("总笔记数: {}", stats.total_notes);
println!("总关系数: {}", stats.total_edges);
println!("\n热门标签:");
for (tag, count) in stats.top_tags {
    println!("  - {}: {} 篇笔记", tag, count);
}
```

## 完整示例

### 构建个人知识库

```rust
use synap_core::{SynapService, Result};

fn main() -> Result<()> {
    let service = SynapService::open("knowledge_base.db")?;

    // 创建主题分类
    let programming = create_category(&service, "编程")?;
    let rust_lang = create_topic(&service, "Rust 语言", &programming)?;
    let python_lang = create_topic(&service, "Python 语言", &programming)?;

    // Rust 相关笔记
    let ownership = create_note(&service, "所有权是 Rust 的核心概念", &rust_lang)?;
    let borrowing = create_note(&service, "借用允许临时使用值", &rust_lang)?;
    let lifetime = create_note(&service, "生命周期确保引用有效", &rust_lang)?;
    create_note(&service, "Rust 的所有权系统", &ownership)?;

    // Python 相关笔记
    let decorators = create_note(&service, "装饰器用于修改函数行为", &python_lang)?;
    let async_py = create_note(&service, "async/await 用于异步编程", &python_lang)?;

    // 添加技术标签
    service.add_tag(ownership.id, "基础".to_string())?;
    service.add_tag(borrowing.id, "基础".to_string())?;
    service.add_tag(lifetime.id, "进阶".to_string())?;
    service.add_tag(decorators.id, "高级特性".to_string())?;
    service.add_tag(async_py.id, "异步".to_string())?;

    // 浏览知识库
    println!("\n=== 知识库结构 ===\n");
    print_graph(&service, programming.id, 0)?;

    // 按标签浏览
    println!("\n=== 按标签浏览 ===\n");
    let tags = service.get_all_tags()?;
    for tag in tags {
        let notes = service.get_notes_by_tag(&tag, None, Some(20))?;
        println!("{} ({} 篇):", tag, notes.len());
        for note in notes {
            println!("  - {}", note.content);
        }
    }

    Ok(())
}

fn create_category(service: &SynapService, name: &str) -> Result<ulid::Ulid> {
    let note = service.create_note(format!("📚 {}", name))?;
    service.add_tag(note.id, "category".to_string())?;
    Ok(note.id)
}

fn create_topic(service: &SynapService, name: &str, parent: &ulid::Ulid) -> Result<ulid::Ulid> {
    let note = service.create_note(format!("📖 {}", name))?;
    service.add_tag(note.id, "topic".to_string())?;
    service.link_notes(*parent, note.id)?;
    Ok(note.id)
}

fn create_note(service: &SynapService, content: &str, parent: &ulid::Ulid) -> Result<ulid::Ulid> {
    let note = service.create_note(content.to_string())?;
    service.link_notes(*parent, note.id)?;
    Ok(note.id)
}

fn print_graph(service: &SynapService, root_id: ulid::Ulid, max_depth: usize) -> Result<()> {
    let graph = service.get_graph(root_id, max_depth)?;
    for (note, depth) in graph {
        let indent = "  ".repeat(depth);
        println!("{}{}", indent, note.content);
    }
    Ok(())
}
```

### 笔记应用集成

```rust
use synap_core::{SynapService, Note, Result};

struct NoteApp {
    service: SynapService,
}

impl NoteApp {
    fn new(db_path: &str) -> Result<Self> {
        Ok(Self {
            service: SynapService::open(db_path)?,
        })
    }

    // 创建新笔记
    fn create_note(&self, content: String, tags: Vec<String>) -> Result<Note> {
        let note = self.service.create_note(content)?;
        for tag in tags {
            self.service.add_tag(note.id, tag)?;
        }
        Ok(note)
    }

    // 搜索并过滤
    fn search(&self, query: &str, tag_filter: Option<&str>) -> Result<Vec<Note>> {
        let mut notes = if let Some(tag) = tag_filter {
            self.service.get_notes_by_tag(tag, None, Some(20))?
        } else {
            self.service.list_notes()?
        };

        if !query.is_empty() {
            notes.retain(|n| n.content.to_lowercase().contains(&query.to_lowercase()));
        }

        Ok(notes)
    }

    // 获取相关笔记
    fn get_related_notes(&self, note_id: ulid::Ulid) -> Result<Vec<Note>> {
        let children = self.service.get_children(note_id)?;
        let parents = self.service.get_parents(note_id)?;

        let mut related = Vec::new();
        related.extend(children);
        related.extend(parents);
        Ok(related)
    }

    // 获取统计摘要
    fn get_summary(&self) -> Result<String> {
        let stats = self.service.get_stats()?;
        Ok(format!(
            "📊 统计摘要\n\
             总笔记数: {}\n\
             总关系数: {}\n\
             标签数: {}",
            stats.total_notes,
            stats.total_edges,
            stats.top_tags.len()
        ))
    }
}
```

## 错误处理最佳实践

```rust
use synap_core::{CoreError, SynapService, Result};

fn handle_operations(service: &SynapService) -> Result<()> {
    // 方式 1: 使用 ? 操作符
    let note = service.create_note("内容".to_string())?;

    // 方式 2: 匹配特定错误
    match service.get_note(note.id) {
        Ok(note) => println!("找到笔记"),
        Err(CoreError::NoteNotFound(id)) => {
            println!("笔记 {} 不存在", id);
        }
        Err(e) => return Err(e.into()),
    }

    // 方式 3: 提供默认值
    let note = service.get_note(note.id)
        .unwrap_or_else(|_| {
            // 创建默认笔记
            service.create_note("默认内容".to_string()).unwrap()
        });

    Ok(())
}
```

## 性能优化建议

1. **使用分页** - 大数据集使用 `list_notes_paginated`
2. **利用标签** - 按标签过滤比全表扫描快
3. **批量操作** - 考虑使用事务进行批量写入
4. **内存数据库** - 测试和临时数据使用 `open_memory()`

希望这些示例帮助你更好地使用 Synap Core！
