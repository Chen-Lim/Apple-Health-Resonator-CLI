
> **变更说明**：本版本基于 v1.0 + Code Review 意见 + Reaction.md 确认修订输出。
> 所有改动均已标注 `[v1.1 修订]`，未标注部分与 v1.0 保持一致。

---

## **1. 项目定位**

这是一个 **纯 Rust 的本地数据引擎**，职责只有三层：

1. **Ingest**：把 Apple Health export.xml / export.zip 流式导入
2. **Store**：写入 SQLite，形成稳定 schema
3. **Expose**：通过 CLI 提供 inspect / stats / query 给用户和 Agent 调用

它不是 GUI app，不是分析平台，也不是可视化工具。

---

## **2. Repo Skeleton**

```
apple-health-resonator/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
├── docs/
│   └── PRD-v1.0.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   │
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs
│   │   ├── ingest.rs
│   │   ├── inspect.rs
│   │   ├── stats.rs
│   │   └── query.rs
│   │
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── record.rs
│   │   ├── workout.rs
│   │   ├── ingest_run.rs
│   │   ├── raw.rs
│   │   └── types.rs
│   │
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── input.rs
│   │   ├── xml_reader.rs
│   │   ├── extractor.rs
│   │   └── normalizer.rs
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── connection.rs
│   │   ├── schema.rs
│   │   ├── writer.rs
│   │   └── query.rs
│   │
│   ├── app/
│   │   ├── mod.rs
│   │   ├── ingest_service.rs
│   │   ├── inspect_service.rs
│   │   ├── stats_service.rs
│   │   └── query_service.rs
│   │
│   ├── output/
│   │   ├── mod.rs
│   │   └── json.rs
│   │
│   └── infra/
│       ├── mod.rs
│       ├── logging.rs
│       ├── hashing.rs
│       ├── time.rs
│       └── error.rs
│
└── tests/
    ├── ingest_basic.rs
    ├── ingest_zip.rs
    ├── dedupe.rs
    ├── inspect.rs
    ├── stats.rs
    ├── query_json.rs
    └── fixtures/
        ├── export-small.xml
        ├── export-dup.xml
        └── export-small.zip
```

---

## **3. Crate 选型**

### **3.1 MVP 必选 crates**

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
quick-xml = "0.37"           # [v1.1 修订] 锁定当前稳定版，不超前锁未发布版本
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indicatif = "0.17"
zip = "2"
sha2 = "0.10"
hex = "0.4"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

> **[v1.1 修订]** PRD 锁技术选择，不锁过细版本。具体小版本在实现阶段由 Cargo.toml 确认。XML parser 要求为 quick-xml，且必须使用 streaming reader 模式。

### **3.2 dev-dependencies**

```toml
[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## **4. 为什么选这些 crate**

### quick-xml
负责 **流式解析 XML**。这是 ingest 核心。必须使用 streaming reader 模式，禁止 DOM / 全文件加载。

### rusqlite
负责 SQLite 持久化。加 `bundled` 是为了减少用户机器上 SQLite 环境差异。

### clap
负责 CLI 参数解析。

### anyhow
统一错误处理，先保证开发速度和清晰度。

### chrono
统一时间解析和 UTC/RFC3339 转换。

### serde / serde_json
用于 CLI JSON 输出。

### indicatif
进度条和 ingest 过程状态显示。

### zip
支持直接 ingest export.zip。

### sha2 + hex
生成 dedupe_key。

### tracing + tracing-subscriber
日志基础设施。

---

## **5. 明确不纳入 MVP 的 crates**

以下不要在 v1.0 里加入：

- polars
- rayon
- sqlx
- tokio
- axum
- tantivy
- 向量数据库相关 crates

原因：当前项目不是服务端系统，也不是并发网络应用。先把 **单机流式导入 + SQLite 查询** 做稳定。

---

## **6. 模块边界**

---

### **6.1 main.rs**

**责任**
- 初始化日志
- 解析 CLI
- 调用对应 command handler
- 退出码控制

**不负责**
- 不负责 XML 解析
- 不负责 SQL
- 不负责 JSON 业务组装
- 不直接拼接 parser + SQL 细节

```rust
fn main() -> anyhow::Result<()> {
    // init logging
    // parse args
    // dispatch subcommand
}
```

---

### **6.2 lib.rs**

**责任**
- 统一暴露 crate 内部模块
- 方便 tests 和未来二次复用

```rust
pub mod app;
pub mod cli;
pub mod domain;
pub mod infra;
pub mod output;
pub mod parser;
pub mod storage;
```

---

### **6.3 cli/**

这一层只管 **命令定义与分发**。

**cli/args.rs**

定义顶层 CLI 结构：

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Ingest(IngestArgs),
    Inspect(InspectArgs),
    Stats(StatsArgs),
    Query(QueryArgs),
}
```

**cli/ingest.rs**

定义 `ahr ingest` 参数。建议字段：
- `path`
- `--db`
- `--batch-size`（可选）
- `--quiet`（可选）

**cli/inspect.rs**

定义 `ahr inspect --db <path>`

**cli/stats.rs**

定义 `ahr stats --db <path>`

**cli/query.rs**

定义 `ahr query --db <path> --sql "<SQL>" --limit N`

**cli/mod.rs**

统一 re-export。

**边界**

> **[v1.1 修订]** CLI 只产出参数对象，不做业务逻辑。**CLI 层必须负责将 CLI args 转换为 app config object，不得将 CLI args 类型直接传入 app 层。**
>
> 违反此规则会导致 `app` 层对 `cli` 层产生依赖，破坏既定依赖方向。

---

### **6.4 domain/**

这是 **领域模型层**，只放核心数据结构。

**domain/raw.rs**

用于表示从 XML 里刚抽出来、还没规范化的对象：

```rust
pub struct RawRecord {
    pub record_type: Option<String>,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub source_name: Option<String>,
    pub source_version: Option<String>,
    pub device: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

pub struct RawWorkout {
    pub workout_type: Option<String>,
    pub duration: Option<String>,
    pub duration_unit: Option<String>,
    pub total_distance: Option<String>,
    pub total_energy_burned: Option<String>,
    pub source_name: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
```

因为 XML 原始字段并不干净，不能直接写库。

**domain/record.rs**

规范化后的 Record：

```rust
pub struct Record {
    pub record_type: String,
    pub value_text: Option<String>,
    pub value_num: Option<f64>,
    pub unit: Option<String>,
    pub source_name: Option<String>,
    pub source_version: Option<String>,
    pub device: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub dedupe_key: String,
}
```

**domain/workout.rs**

规范化后的 Workout。

> **[v1.1 修订] Workout scope 限制**：MVP 仅支持 Workout 顶层属性。不支持 `WorkoutEvent`、`WorkoutRoute`、`MetadataEntry` 等嵌套子元素。xml_reader 遇到这些子元素时必须安全跳过，不得报 fatal error。

**domain/ingest_run.rs**

导入任务元数据：

```rust
pub struct IngestRun {
    pub started_at: String,
    pub finished_at: Option<String>,
    pub input_path: String,
    pub records_inserted: i64,
    pub workouts_inserted: i64,
    pub records_skipped: i64,
    pub errors_count: i64,
    pub schema_version: String,
}
```

**domain/types.rs**

放公共类型和别名：

```rust
pub type JsonValue = serde_json::Value;

pub enum ParsedEntity {
    Record(Record),
    Workout(Workout),
}
```

> **[v1.1 新增]** 各命令的 app config struct 定义于此，供 app 层使用：
>
> ```rust
> pub struct IngestConfig {
>     pub input_path: PathBuf,
>     pub db_path: PathBuf,
>     pub batch_size: usize,
>     pub quiet: bool,
> }
>
> pub struct InspectConfig { pub db_path: PathBuf }
> pub struct StatsConfig   { pub db_path: PathBuf }
>
> pub struct QueryConfig {
>     pub db_path: PathBuf,
>     pub sql: String,
>     pub limit: usize,
> }
> ```
>
> CLI 层做转换，app 层只接受 config。

**domain/mod.rs**

统一导出。

---

### **6.5 parser/**

这是 ingest 的核心层。只负责：**读 XML → 抽字段 → 规范化**。

---

#### **parser/input.rs** ⚠️ [v1.1 修订]

> **[v1.1 修订]** 原 v1.0 建议的 `Box<dyn BufRead>` 接口被废弃。
>
> **原因**：`ZipFile` 生命周期绑定在 `ZipArchive` 上，无法作为独立 trait object 返回，会导致 Rust borrow checker 编译失败。

**新设计：enum ownership model**

```rust
pub enum InputSource {
    Xml(std::io::BufReader<std::fs::File>),
    Zip {
        archive: zip::ZipArchive<std::fs::File>,
        entry_index: usize,
    },
}
```

**责任**
- 判断输入路径是 `.zip` 还是 `.xml`
- 若是 zip，定位其中 `export.xml` 的 entry index
- 构造并返回 `InputSource`，由调用方持有其生命周期
- `xml_reader` 接受 `&mut InputSource`，由 `InputSource` 自身提供读取能力

**建议接口**

```rust
pub fn open_input(path: &Path) -> anyhow::Result<InputSource>;
```

**不允许的做法**

```rust
// ❌ 禁止：ZipFile 无法脱离 archive 独立作为 trait object 返回
pub fn open_input(path: &Path) -> anyhow::Result<Box<dyn std::io::BufRead>>;
```

---

#### **parser/xml_reader.rs**

对 quick-xml 的封装。

**责任**
- 流式读取 XML event（必须 streaming，禁止全文件加载）
- 只识别当前 MVP 需要的 tag：`Record`、`Workout`
- 不做字段类型转换
- 遇到 `WorkoutEvent`、`WorkoutRoute`、`MetadataEntry` 等子元素，安全跳过

**输出**

输出"原始属性集合"给 extractor：

```rust
pub enum XmlEntity {
    Record(Vec<(String, String)>),
    Workout(Vec<(String, String)>),
}

pub fn next_entity(source: &mut InputSource) -> anyhow::Result<Option<XmlEntity>>;
```

---

#### **parser/extractor.rs**

**责任**
- 从 XML attribute key/value 中取出有用字段
- 做最轻量的存在性提取
- 转成 `RawRecord` / `RawWorkout`
- 不做时间标准化
- 不做 dedupe

---

#### **parser/normalizer.rs** ⚠️ [v1.1 修订]

这是最重要的业务转换层。

**责任**
- 时间转换为 UTC RFC3339
- 数值字符串转 `f64`
- 缺失字段容错
- 生成 `dedupe_key`（**必须在所有字段规范化完成后**）
- 产出 `Record` / `Workout`

> **[v1.1 修订] dedupe_key 生成顺序约束（硬性规则）**
>
> `dedupe_key` 必须基于 **规范化后的字段** 生成：
> - `start_date` 和 `end_date` 必须已转换为 UTC RFC3339，再参与 hash
> - **禁止使用原始 XML 时间字符串直接生成 `dedupe_key`**
>
> 错误的顺序会导致同一条记录因时区表示不同而产生不同 hash，导致去重失效。
>
> 正确生成顺序：
> ```
> 1. parse raw fields
> 2. normalize datetime → UTC RFC3339
> 3. normalize numeric fields
> 4. generate dedupe_key from normalized fields
> 5. return Record / Workout
> ```

**边界**
- 不直接访问数据库
- 不负责读取 XML

**建议接口**

```rust
pub fn normalize_record(raw: RawRecord) -> anyhow::Result<Record>;
pub fn normalize_workout(raw: RawWorkout) -> anyhow::Result<Workout>;
```

---

#### **parser/mod.rs**

统一导出 parser 模块。

---

### **6.6 storage/**

这是 SQLite 层。

---

#### **storage/connection.rs**

**责任**
- 打开 SQLite 连接
- 初始化 pragma（`journal_mode`、`synchronous`、`foreign_keys` 等）
- 返回 `rusqlite::Connection`

```rust
pub fn open_db(path: &Path) -> anyhow::Result<rusqlite::Connection>;
```

---

#### **storage/schema.rs**

**责任**
- 创建表
- 创建索引
- schema version 管理

```rust
pub fn init_schema(conn: &Connection) -> anyhow::Result<()>;
pub fn schema_version() -> &'static str;
```

---

#### **storage/writer.rs** ⚠️ [v1.1 修订]

**责任**
- 批量写入 records
- 批量写入 workouts
- 事务提交
- 处理 unique constraint 导致的重复写入跳过

> **[v1.1 修订] BatchWriter 不得拥有 Connection 所有权**
>
> **原因**：若 `BatchWriter::new(conn)` 将 `Connection` move 进去，`ingest_service` 在 flush 完成后将无法继续使用同一连接写入 `ingest_runs`，导致编译失败或逻辑断裂。
>
> **必须改为借用方式**：

```rust
pub struct BatchWriter<'a> {
    conn: &'a mut Connection,
    batch_size: usize,
    records_inserted: i64,
    workouts_inserted: i64,
    records_skipped: i64,
    // ...
}

impl<'a> BatchWriter<'a> {
    pub fn new(conn: &'a mut Connection, batch_size: usize) -> anyhow::Result<Self>;
    pub fn write_record(&mut self, record: &Record) -> anyhow::Result<()>;
    pub fn write_workout(&mut self, workout: &Workout) -> anyhow::Result<()>;
    pub fn flush(&mut self) -> anyhow::Result<()>;
}
```

**ingest_service 的正确编排顺序**：

```
1. open_db(path) → conn
2. init_schema(&conn)
3. { BatchWriter::new(&mut conn, ...) → 写入数据 → flush }
4. 用同一 conn 写入 ingest_runs   ← BatchWriter 已释放借用，此处合法
```

---

#### **storage/query.rs**

**责任**
- 执行 inspect 所需查询
- 执行 stats 所需查询
- 执行受控只读 SQL 查询
- 将结果变成中间数据结构

**边界**
- 不处理 CLI 参数
- 不直接决定最终 JSON 格式

```rust
pub fn fetch_inspect_summary(conn: &Connection) -> anyhow::Result<InspectSummary>;
pub fn fetch_stats_summary(conn: &Connection) -> anyhow::Result<StatsSummary>;
pub fn run_select_query(conn: &Connection, sql: &str, limit: usize) -> anyhow::Result<Vec<RowMap>>;
```

---

#### **storage/mod.rs**

统一导出。

---

### **6.7 app/**

这是 **应用服务层**，把 CLI、parser、storage 串起来。没有这一层，`main.rs` 会迅速变胖。

---

#### **app/ingest_service.rs**

**责任**：编排完整 ingest pipeline：

1. 接受 `IngestConfig`（不是 CLI args）
2. 打开输入（`InputSource`）
3. 初始化 DB
4. 初始化 schema
5. 流式读取 XML
6. extract → normalize
7. batch write
8. 记录 `ingest_runs`
9. 返回结果摘要

```rust
pub struct IngestReport {
    pub records_inserted: i64,
    pub workouts_inserted: i64,
    pub records_skipped: i64,
    pub errors_count: i64,
    pub elapsed_ms: u128,
}

// [v1.1 修订] 接受 IngestConfig，不接受 CLI IngestArgs
pub fn run_ingest(config: IngestConfig) -> anyhow::Result<IngestReport>;
```

---

#### **app/inspect_service.rs**

调用 `storage::query`，返回 `InspectSummary` DTO。

---

#### **app/stats_service.rs**

调用 `storage::query`，返回 `StatsSummary` DTO。

---

#### **app/query_service.rs** ⚠️ [v1.1 修订]

做 SQL 安全检查，只允许单条只读 SELECT，然后调用 `storage::query`。

> **[v1.1 修订] query 安全边界升级（硬性要求）**
>
> 原 v1.0 的"字符串前缀判断"（`starts_with("SELECT")`）不足，可被 `SELECT 1; DROP TABLE records` 绕过。
>
> **v1.1 要求两层限制**：
>
> **第一层：禁止多语句**
> - SQL 输入经解析后，不能存在第二条可执行 statement
> - 任何包含 `;` 后跟有效语句的输入必须被拒绝
>
> **第二层：statement 级别只读校验**
> - 使用 `rusqlite::Connection::prepare()` 解析 SQL
> - 通过 statement 元信息判断是否为只读 SELECT
> - **禁止以字符串猜测替代 statement 级别校验**
> - 拒绝：`PRAGMA`、`ATTACH`、任何 DDL（`CREATE`、`DROP`、`ALTER`）、任何 DML（`INSERT`、`UPDATE`、`DELETE`）
>
> 这是安全边界，不是风格选择。

---

### **6.8 output/**

这一层专门保证 **输出稳定**。

**output/json.rs**

**责任**
- 把 DTO 转成稳定 JSON
- 控制字段名和输出结构
- 保证 Agent 调用时返回一致
- 支持 compact / pretty 两种模式，未来可在此统一字段命名风格

```rust
pub fn to_pretty_json<T: serde::Serialize>(value: &T) -> anyhow::Result<String>;
pub fn to_compact_json<T: serde::Serialize>(value: &T) -> anyhow::Result<String>;
```

"查什么"和"怎么输出"是两件事，所以这层保持独立。

---

### **6.9 infra/**

基础设施层。

---

#### **infra/logging.rs**

初始化 tracing：

```rust
pub fn init_logging() -> anyhow::Result<()>;
```

---

#### **infra/hashing.rs**

生成 dedupe key：

```rust
pub fn sha256_hex(input: &str) -> String;
```

---

#### **infra/time.rs** ⚠️ [v1.1 修订]

统一时间解析和 UTC/RFC3339 格式化。

> **[v1.1 修订] Apple Health 时间格式处理为实现要求（不再只是注意事项）**
>
> Apple Health 的时间格式为：
> ```
> 2024-01-15 08:30:00 +0800
> ```
> 这不是标准 RFC3339（使用空格而非 `T` 分隔，时区为偏移量）。
>
> **`chrono::DateTime::parse_from_rfc3339` 会直接失败**，不能使用。
>
> **必须**：
> - 使用 `chrono::DateTime::parse_from_str` + 自定义 format string
> - 解析成功后转换为 UTC
> - 输出统一为 RFC3339 格式（`%Y-%m-%dT%H:%M:%SZ`）

```rust
// 格式参考："%Y-%m-%d %H:%M:%S %z"
pub fn parse_apple_health_datetime(input: &str) -> anyhow::Result<String>;
```

---

#### **infra/error.rs**

如果后面需要更细的错误类型，在这里集中定义。MVP 阶段先只用 `anyhow`，此文件可先保留为空壳。

---

## **7. 数据流**

```
CLI args
  ↓
CLI 层转换为 IngestConfig        ← [v1.1 修订] CLI args 不穿透到 app 层
  ↓
app::ingest_service
  ↓
parser::input (InputSource enum) ← [v1.1 修订] enum ownership，不返回 trait object
  ↓
parser::xml_reader
  ↓
parser::extractor
  ↓
parser::normalizer
  ├─ 1. normalize datetime → UTC RFC3339
  ├─ 2. normalize numeric fields
  └─ 3. generate dedupe_key      ← [v1.1 修订] 必须在规范化之后
  ↓
storage::writer (BatchWriter<'a>) ← [v1.1 修订] 借用连接，不拥有
  ↓
SQLite
  ↓
storage::query
  ↓
app::{inspect,stats,query}_service
  └─ query_service 做 statement 级别安全校验  ← [v1.1 修订]
  ↓
output::json
  ↓
stdout
```

---

## **8. 模块调用规则**

### **允许的依赖方向**

```
cli      -> app
app      -> parser / storage / output / infra / domain
parser   -> domain / infra
storage  -> domain / infra
output   -> domain
infra    -> (尽量不依赖业务模块)
domain   -> 不依赖其他业务层
```

### **禁止的依赖方向**

- `domain` -> `storage`
- `domain` -> `parser`
- `parser` -> `cli`
- `storage` -> `cli`
- `output` -> `storage`
- `app` -> `cli`（**[v1.1 新增]**：app 层不得使用 cli 类型）
- `main.rs` 直接拼接 parser + SQL 细节

> **依赖必须向内收敛，不能横向乱连。**

---

## **9. Cargo.toml 建议版本**

```toml
[package]
name = "apple-health-resonator"
version = "0.1.0"
edition = "2021"
description = "High-performance Rust CLI for ingesting Apple Health export into SQLite"
license = "MIT"

[[bin]]
name = "ahr"
path = "src/main.rs"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
quick-xml = "0.37"           # [v1.1 修订] 锁当前稳定版
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indicatif = "0.17"
zip = "2"
sha2 = "0.10"
hex = "0.4"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## **10. 首批核心数据结构**

### InspectSummary

```rust
#[derive(serde::Serialize)]
pub struct InspectSummary {
    pub tables: Vec<String>,
    pub record_count: i64,
    pub workout_count: i64,
    pub date_range: DateRange,
    pub sources: Vec<String>,
    pub record_types: Vec<String>,
}
```

### StatsSummary

```rust
#[derive(serde::Serialize)]
pub struct StatsSummary {
    pub total_records: i64,
    pub total_workouts: i64,
    pub top_types: Vec<TypeCount>,
    pub top_sources: Vec<SourceCount>,
    pub recent_activity: bool,
}
```

### DateRange

```rust
#[derive(serde::Serialize)]
pub struct DateRange {
    pub start: Option<String>,
    pub end: Option<String>,
}
```

### IngestConfig ⚠️ [v1.1 新增]

```rust
pub struct IngestConfig {
    pub input_path: PathBuf,
    pub db_path: PathBuf,
    pub batch_size: usize,
    pub quiet: bool,
}

pub struct InspectConfig { pub db_path: PathBuf }
pub struct StatsConfig   { pub db_path: PathBuf }

pub struct QueryConfig {
    pub db_path: PathBuf,
    pub sql: String,
    pub limit: usize,
}
```

---

## **11. 命令边界**

### ahr ingest
- 可读 zip/xml（通过 `InputSource` enum 处理）
- 只负责导入
- 输出 ingest report
- 不返回数据行

### ahr inspect
- 返回 schema 和数据概览
- 给 Agent 建立上下文

### ahr stats
- 返回摘要统计
- 给 Agent 优先调用

### ahr query
- **[v1.1 修订]** 只允许**单条只读 SELECT 语句**
- 通过 statement 级别能力判断，不依赖字符串前缀
- 拒绝多语句、PRAGMA、DDL、DML
- 默认附加 LIMIT（默认 1000）
- 输出 JSON array

---

## **12. 测试边界**

建议第一批测试只做集成测试，不要过早沉迷微小单元测试。

### 必测

**tests/ingest_basic.rs**
- 小 XML 能成功导入
- records/workouts 数量正确

**tests/ingest_zip.rs**
- zip 能找到 export.xml
- 导入成功

**tests/dedupe.rs**
- 重复记录不会重复入库
- **[v1.1 新增]** 同一条记录在不同时区表示下（如 `+0800` vs UTC）仍能正确去重

**tests/inspect.rs**
- inspect JSON 字段完整稳定

**tests/stats.rs**
- stats 输出 `top_types` / `top_sources` 合理

**tests/query_json.rs**
- query 只允许 SELECT
- **[v1.1 新增]** 多语句输入被拒绝（如 `SELECT 1; DROP TABLE records`）
- LIMIT 生效
- JSON 输出为 array

---

## **13. 开发顺序建议**

### Phase 1
- `Cargo.toml`
- `main.rs`
- `cli/args.rs`
- `infra/logging.rs`

### Phase 2
- `storage/connection.rs`
- `storage/schema.rs`
- `domain/*`（含所有 config structs）

### Phase 3

> **[v1.1 修订]** `infra/time.rs` 前置到 Phase 3 最前，因为 normalizer 依赖它。

- `infra/time.rs`（Apple Health datetime parser，**先做**）
- `parser/input.rs`（`InputSource` enum）
- `parser/xml_reader.rs`
- `parser/extractor.rs`
- `parser/normalizer.rs`

### Phase 4
- `storage/writer.rs`（`BatchWriter<'a>`）
- `app/ingest_service.rs`

### Phase 5
- `storage/query.rs`
- `app/inspect_service.rs`
- `app/stats_service.rs`
- `app/query_service.rs`（含 statement 级别安全校验）
- `output/json.rs`

### Phase 6
- integration tests
- CLI polish
- README

---

## **14. 一句话总结每层职责**

| 层 | 职责 |
|---|---|
| **cli** | 接命令，转换为 config，不穿透 args 到 app |
| **app** | 编排流程，只接受 config |
| **parser** | 读 XML 并规范化（input 用 enum；normalizer 在规范化后生成 dedupe_key）|
| **domain** | 定义核心数据结构和 config |
| **storage** | 写库和查库（writer 借用连接不拥有；query 做 statement 级别安全校验）|
| **output** | 稳定输出 JSON |
| **infra** | 日志、时间（含 Apple Health 格式专用 parser）、哈希等基础设施 |

---

## **附录：v1.0 → v1.1 修订一览**

| 编号 | 位置 | 修订内容 | 级别 |
|---|---|---|---|
| A | `parser/input.rs` | 废弃 `Box<dyn BufRead>`，改为 `InputSource` enum ownership model | 🔴 必须 |
| B | `storage/writer.rs` | `BatchWriter` 改为借用 `&'a mut Connection`，不拥有 | 🔴 必须 |
| C | `app/query_service.rs` | 安全校验从字符串前缀升级为 statement 级别两层限制 | 🔴 必须 |
| D | `parser/normalizer.rs` | dedupe_key 必须在字段规范化（含 UTC 转换）之后生成 | 🟡 必须 |
| E | `domain/workout.rs` | 明确 WorkoutEvent 等嵌套子元素不在 MVP scope，必须安全跳过 | 🟡 必须 |
| F | `infra/time.rs` | Apple Health 时间格式处理从注意事项升级为实现要求 | 🟡 必须 |
| G | `cli/` + `app/` | CLI args 与 app config 解耦，新增各命令 config struct | 🟢 建议 |
| H | `Cargo.toml` | `quick-xml` 版本号调整为当前稳定版 `0.37` | 🟢 建议 |
| I | 模块调用规则 | 禁止依赖方向新增：`app` 不得依赖 `cli` 类型 | 🟢 建议 |
| J | 开发顺序 | `infra/time.rs` 前置到 Phase 3 最前 | 🟢 建议 |
| K | `tests/dedupe.rs` | 新增时区归一化去重测试用例 | 🟢 建议 |
| L | `tests/query_json.rs` | 新增多语句拒绝测试用例 | 🟢 建议 |
