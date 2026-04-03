//! Database layer for Synap.
//!
//! 这一层的目标不是把 `redb` 原样暴露出去，而是提供一组更稳定的
//! “领域友好型存储原语”：
//!
//! - `KvStore<K, V>`: 类型化的 KV 存储，负责 value codec
//! - `SetStore<K>`: 只存 key 的集合
//! - `OneToMany<K, V>`: 一对多关系
//! - `DagStore`: 基于双向 `OneToMany` 的有向图关系
//! - `VectorStore<V>`: 向量索引
//!
//! ## 设计约定
//!
//! db 层里大多数存储原语都分成两类对象：
//!
//! - `*Store`: 静态蓝图，描述“表叫什么、key/value 类型是什么”，并承载写操作
//! - `*Reader`: 运行时读视图，绑定到某个 `ReadTransaction`，并承载读操作
//!
//! 这套分工不是“读写对称拆一半”，而是：
//!
//! - `Store` 默认负责 schema、初始化、局部写操作
//! - `Reader` 默认负责正式的读取能力
//! - 只有少数“写流程里的短生命周期辅助读取”才会留在 `Store` 上
//!
//! 这种拆分不只是为了“给迭代器提供生命周期”，它还同时解决了几个问题：
//!
//! 1. 把“静态 schema 定义”和“事务期 runtime 资源”分开。
//!    `Store` 本身可以做成 `const`，适合作为模块级静态定义；
//!    `Reader` 才持有真正打开过的 table / cursor。
//! 2. 让多个读操作天然共享同一个 snapshot。
//!    一个 `Reader` 里的所有查询都绑定到同一个 `ReadTransaction`，
//!    这样组合查询时不会意外跨 snapshot。
//! 3. 让惰性迭代器有稳定 owner。
//!    redb 的 range / multimap cursor 都会借用 table；
//!    如果在函数内部临时打开 table 再把 iterator 往外返回，就会立刻悬空。
//!    `Reader` 的职责就是稳稳持有 table，给这些惰性 iterator 兜底。
//! 4. 让读路径和写路径的资源管理更清晰。
//!    读路径适合长期持有 `Reader`；
//!    写路径则应在局部作用域中“打开 table -> 操作 -> 立即释放”，
//!    因此写操作天然更适合留在 `Store` 本体上。
//! 5. 给 codec / envelope 留一个稳定的落点。
//!    现在 `KvStore` 的 value 编解码统一放在 `db::codec`，
//!    上层模型不应该再私自对底层 bytes 做 `postcard` 读写。
//!
//! ## 使用规范
//!
//! ### 1. `Store` 是写入口和 schema 蓝图，不是读会话
//!
//! `Store` 负责描述表定义和提供基本操作，但它本身不持有 transaction。
//! 如果你的读逻辑会返回惰性 iterator，或者会在同一 snapshot 下做多次查询，
//! 就应该先创建 `Reader`，再从 `Reader` 上继续读。
//!
//! 可以把它理解成：
//!
//! - `Store`: “这张表怎么建、怎么写”
//! - `Reader`: “这张表在这次 read transaction 里怎么读”
//!
//! ### 2. 惰性迭代器必须从 `Reader` 借出
//!
//! 推荐：
//!
//! ```ignore
//! let rtx = db.begin_read()?;
//! let reader = NOTE_STORE.reader(&rtx)?;
//! let iter = reader.iter()?;
//! ```
//!
//! 不推荐：
//!
//! ```ignore
//! fn bad(tx: &ReadTransaction) -> impl Iterator<...> {
//!     let table = tx.open_table(...)?;
//!     table.range(..)
//! }
//! ```
//!
//! 后者的问题是 iterator 借用了局部 `table`，函数返回时 owner 已经销毁。
//!
//! ### 3. 写路径优先“短借用”，所以默认放在 `Store`
//!
//! 写操作尽量在方法内部局部打开 table，用完即放掉。
//! 如果写流程里需要“先读 typed value 再写”，应该提供像
//! `KvStore::get_in_write()` 这样的 helper，而不是在模型层手写
//! `open_table + postcard::from_bytes`。
//!
//! 这类 `*_in_write` helper 是例外，不是主路径。
//! 它们存在的目的，是为了让写流程里的辅助读取仍然遵守 db 层的 codec 和类型约束。
//!
//! ### 4. `KvStore` 的 value bytes 不能绕过 codec
//!
//! `KvStore` 现在已经接入 envelope/压缩 codec：
//!
//! - 读：自动兼容 legacy 明文和新 envelope
//! - 写：按默认 profile 输出新格式
//!
//! 所以对 `KvStore` 的 value 来说，下面这种旁路现在应该视为禁用：
//!
//! ```ignore
//! let table = tx.open_table(STORE.table_def())?;
//! let raw = table.get(key)?;
//! let value: V = postcard::from_bytes(raw.value())?;
//! ```
//!
//! 因为底层 bytes 已经不再保证是裸 `postcard`。
//! 正确方式是走 `KvStore::put` / `KvReader::get` / `KvReader::iter`
//! / `KvStore::get_in_write`。
//!
//! ### 5. 跨多张表的业务读取，应该再包一层领域 Reader
//!
//! 比如 `NoteReader`、`DagReader` 这种类型，它们不是多余抽象，
//! 而是“同一读事务下的领域视图”：
//!
//! - 可以一次性持有多张表的 reader
//! - 可以把多步查询组合成稳定 API
//! - 可以继续向外返回安全的惰性 iterator
//!
//! ## 什么时候可以直连 redb？
//!
//! 只有在下面这几种情况才值得直接碰原始 table：
//!
//! - 操作的不是 `KvStore` value，而是 `SetStore` / `OneToMany` 这类原生结构
//! - 做底层迁移、调试、校验、测试
//! - codec 层本身需要处理原始 bytes
//!
//! 即便如此，也应该把原始 bytes 的解释权尽量留在 db 层，不要把它泄漏到模型层。
mod codec;
pub mod dagstorage;
pub mod kvstore;
pub mod onetomany;
pub mod setstore;
pub mod types;
pub mod vector;
