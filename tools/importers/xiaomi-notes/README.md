# Xiaomi Notes Importer

实验性导入工具，用于把小米笔记 SQLite 数据迁移到 Synap 的 `.redb` 数据库。

## 用法

```bash
cargo run -p synap-xiaomi-notes-import -- --source note.db --target synap_database.redb --dry-run
cargo run -p synap-xiaomi-notes-import -- --source note.db --target synap_database.redb
```

## 说明

- 默认只导入普通文本笔记。
- 会保留小米笔记里的 `created_date` 作为 Synap 笔记时间。
- 附件目前只做统计，不会导入到 Synap。
- 不是幂等工具；重复执行会产生重复笔记。
- 导入前请先备份目标 `.redb` 数据库。
