# Synap Core Module

基于 KV 数据库的核心逻辑层，支持 Markdown 笔记的存储、检索和管理。

## 特性

- ✅ **统一服务接口** - 通过 `SynapService` 访问所有功能
- ✅ **CRUD 操作** - 创建、读取、更新、删除笔记
- ✅ **知识图谱** - 支持 DAG（有向无环图）关系
- ✅ **标签索引** - 高效的标签查询和管理
- ✅ **查询功能** - 文本搜索和分页查询
- ✅ **软删除** - 安全的数据删除机制
- ✅ **持久化** - 基于 redb 的 ACID 事务支持
- ✅ **统计功能** - 获取数据库统计信息

## 快速开始

```rust
use synap_core::SynapService;

// 打开数据库
let service = SynapService::open("my_notes.db").unwrap();

// 创建笔记
let note = service.create_note("我的第一篇笔记".to_string()).unwrap();

// 添加标签
service.add_tag(note.id, "rust".to_string()).unwrap();

// 创建关系
let related = service.create_note("相关笔记".to_string()).unwrap();
service.link_notes(note.id, related.id).unwrap();

// 查询子节点
let children = service.get_children(note.id).unwrap();
println!("有 {} 个相关笔记", children.len());
```

## API 概览

### 数据库操作

```rust
// 文件数据库
let service = SynapService::open("path/to/db")?;

// 内存数据库（测试用）
let service = SynapService::open_memory()?;
```

### 笔记操作

```rust
// 创建
let note = service.create_note("内容".to_string())?;

// 读取
let note = service.get_note(note_id)?;

// 更新
let note = service.update_note(note_id, "新内容".to_string())?;

// 删除（软删除）
service.delete_note(note_id)?;

// 列出所有
let notes = service.list_notes()?;

// 分页
let page = service.list_notes_paginated(0, 10)?;

// 搜索
let results = service.search_notes("关键词")?;
```

### 知识图谱

```rust
// 建立关系
service.link_notes(parent_id, child_id)?;

// 删除关系
service.unlink_notes(parent_id, child_id)?;

// 获取子节点
let children = service.get_children(note_id)?;

// 获取父节点
let parents = service.get_parents(note_id)?;

// 获取完整图谱
let graph = service.get_graph(root_id, max_depth)?;
```

### 标签管理

```rust
// 添加标签
service.add_tag(note_id, "标签名".to_string())?;

// 删除标签
service.remove_tag(note_id, "标签名")?;

// 按标签查询（支持 cursor + limit）
let notes = service.get_notes_by_tag("rust", None, Some(20))?;

// 获取所有标签
let tags = service.get_all_tags()?;

// 获取笔记的标签
let tags = service.get_tags_for_note(note_id)?;
```

### 统计功能

```rust
let stats = service.get_stats()?;
println!("总笔记数: {}", stats.total_notes);
println!("总关系数: {}", stats.total_edges);
println!("热门标签: {:?}", stats.top_tags);
```

## 核心概念

### Note（笔记）

```rust
pub struct Note {
    pub id: Ulid,              // 唯一标识
    pub content: String,        // Markdown 内容
    pub updated_at: SystemTime, // 更新时间
    pub created_at: SystemTime, // 创建时间
    pub deleted: bool,          // 软删除标记
    pub tags: Vec<String>,      // 标签列表
}
```

### DAG 关系

笔记之间可以建立有向无环图（DAG）关系：

- **link_notes(parent, child)** - 从父节点指向子节点
- **循环检测** - 自动阻止创建循环
- **双向查询** - 可以查询子节点和父节点

### 标签系统

- 标签用于分类和组织笔记
- 支持高效的按标签查询
- 标签不能包含空字节或为空

## 使用示例

### 构建知识图谱

```rust
use synap_core::SynapService;

let service = SynapService::open("knowledge.db")?;

// 创建根节点
let rust = service.create_note("Rust 编程语言".to_string()).unwrap();
service.add_tag(rust.id, "编程".to_string()).unwrap();

// 创建子主题
let ownership = service.create_note("所有权系统".to_string()).unwrap();
let borrowing = service.create_note("借用机制".to_string()).unwrap();

// 建立关系
service.link_notes(rust.id, ownership.id).unwrap();
service.link_notes(rust.id, borrowing.id).unwrap();

// 查询图谱
let graph = service.get_graph(rust.id, 0).unwrap();
for (note, depth) in graph {
    println!("{}{}", "  ".repeat(depth), note.content);
}
```

### 按标签浏览

```rust
// 获取所有 Rust 相关的笔记
let rust_notes = service.get_notes_by_tag("rust", None, Some(20)).unwrap();

// 获取所有标签
let tags = service.get_all_tags().unwrap();
for tag in tags {
    let count = service.get_notes_by_tag(&tag, None, Some(20)).unwrap().len();
    println!("{}: {} 篇笔记", tag, count);
}
```

### 搜索和分页

```rust
// 搜索笔记
let results = service.search_notes("async").unwrap();
for note in results {
    println!("找到: {}", note.content);
}

// 分页浏览
let page1 = service.list_notes_paginated(0, 20).unwrap();
println!("第 1 页: {} 条笔记", page1.len());
```

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_service_creation

# 显示测试输出
cargo test -- --nocapture
```

## 性能

- 创建笔记: < 1ms
- 读取笔记: < 1ms
- 列出 1000 条笔记: < 100ms
- 搜索 1000 条笔记: < 50ms

## 错误处理

```rust
use synap_core::{CoreError, SynapService};

let service = SynapService::open_memory().unwrap();

match service.get_note(some_id) {
    Ok(note) => println!("找到笔记"),
    Err(CoreError::NoteNotFound(id)) => println!("笔记不存在"),
    Err(e) => eprintln!("其他错误: {}", e),
}
```

## 架构

```
SynapService (公共接口)
    ├── Note 操作
    ├── DAG 关系
    ├── 标签管理
    ├── 查询功能
    └── 统计分析
            ↓
    SynapDb (私有实现)
    ├── redb 数据库
    ├── 4 张 KV 表
    └── 事务管理
```

更多示例请参阅 [EXAMPLES.md](EXAMPLES.md)。

## 许可证

MIT License
