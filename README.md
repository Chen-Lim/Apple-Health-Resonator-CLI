# Apple Health Resonator

一个使用 Rust 构建的本地 CLI 工具，用于将 Apple Health 导出的 `export.xml` 或 `export.zip` 转换为 SQLite，并提供适合 Agent / LLM 调用的结构化查询接口。

## 项目目标

本项目聚焦三件事：

- `ingest`：流式导入 Apple Health 导出文件
- `store`：写入 SQLite，形成稳定 schema
- `expose`：通过 CLI 提供 `inspect / stats / query`

不包含以下能力：

- GUI
- 数据可视化
- 云同步
- embedding / 向量数据库
- 自动 SQL 生成

## 当前特性

- 支持输入 `export.xml` 和 `export.zip`
- 使用 `quick-xml` 进行 streaming XML 解析
- 使用 SQLite 作为本地数据存储
- 支持记录去重
- 支持导入运行记录 `ingest_runs`
- 支持稳定 JSON 输出的 `inspect`
- 支持紧凑 JSON 输出的 `stats`
- 支持受控只读 SQL 的 `query`

## 技术栈

- Rust 2021
- `quick-xml`
- `rusqlite`
- `clap`
- `anyhow`
- `chrono`
- `serde / serde_json`
- `indicatif`
- `tracing`

## 安装与构建

要求：

- Rust 工具链
- `cargo`

构建：

```bash
cargo build --release
```

运行测试：

```bash
cargo test
```

可执行文件名：

```bash
ahr
```

查看 CLI 帮助与版本：

```bash
ahr --help
ahr --version
```

如果使用源码直接运行：

```bash
cargo run -- <subcommand>
```

## 快速开始

### 1. 导入 Apple Health 数据

```bash
cargo run -- ingest /path/to/export.xml
```

或：

```bash
cargo run -- ingest /path/to/export.zip
```

默认数据库文件为当前目录下的 `./health_data.db`。

也可以显式指定：

```bash
cargo run -- ingest /path/to/export.xml --db /path/to/health.db
```

### 2. 查看数据库摘要

```bash
cargo run -- inspect --db ./health_data.db
```

示例输出：

```json
{
  "tables": ["ingest_runs", "records", "workouts"],
  "record_count": 123,
  "workout_count": 8,
  "date_range": {
    "start": "2024-01-01T00:00:00Z",
    "end": "2024-03-31T23:59:59Z"
  },
  "sources": ["Apple Watch", "iPhone"],
  "record_types": ["HKQuantityTypeIdentifierStepCount"]
}
```

### 3. 查看统计信息

```bash
cargo run -- stats --db ./health_data.db
```

示例输出：

```json
{"total_records":123,"total_workouts":8,"top_types":[...],"top_sources":[...],"recent_activity":true}
```

### 4. 执行只读查询

```bash
cargo run -- query --db ./health_data.db --sql "SELECT record_type, value_num, start_date FROM records ORDER BY start_date DESC" --limit 20
```

示例输出：

```json
[{"record_type":"HKQuantityTypeIdentifierStepCount","value_num":1234.0,"start_date":"2024-01-15T00:00:00Z"}]
```

## CLI 说明

### `ahr ingest`

```bash
ahr ingest <path> [--db <path>] [--batch-size <n>] [--quiet]
```

说明：

- 自动识别输入是 `.xml` 还是 `.zip`
- 默认数据库路径：`./health_data.db`
- 默认批量写入大小：`10000`
- `--quiet` 可关闭进度条

### `ahr inspect`

```bash
ahr inspect --db <path>
```

输出为稳定的 pretty JSON，适合人工阅读和程序解析。

### `ahr stats`

```bash
ahr stats --db <path>
```

输出为紧凑 JSON，适合 Agent 调用。

### `ahr query`

```bash
ahr query --db <path> --sql "<SQL>" [--limit N]
```

限制：

- 只允许单条只读查询
- 不允许多语句
- 不允许 DDL / DML / `PRAGMA` / `ATTACH`
- 默认 `limit = 1000`

## 数据库 Schema

当前 MVP 包含三张表：

- `records`
- `workouts`
- `ingest_runs`

其中：

- `records` 存储 Apple Health Record
- `workouts` 存储 Workout 顶层属性
- `ingest_runs` 存储每次导入的元信息

同时创建以下索引：

- `idx_records_type_date`
- `idx_records_source_date`
- `idx_workouts_type_date`

## 数据处理规则

### 时间标准化

Apple Health 原始时间格式类似：

```text
2024-01-15 08:30:00 +0800
```

导入时会统一转换为 UTC RFC3339：

```text
2024-01-15T00:30:00Z
```

### 去重规则

去重键基于规范化后的字段生成，核心字段包括：

- type
- source
- start_date
- end_date
- value
- unit

这意味着同一条记录即使原始时区表示不同，只要标准化后相同，也会被正确去重。

### Workout 范围

当前仅支持 Workout 顶层属性，不处理以下嵌套内容：

- `WorkoutEvent`
- `WorkoutRoute`
- `MetadataEntry`

遇到这些子元素时会安全跳过，不会导致 fatal error。

## 关于 zip 导入实现

当前 zip 导入不是先解压到临时文件，而是：

- 在 `InputSource` 中持有 `ZipArchive<File>`
- 在 `ingest_service` 的同一作用域内借用 `export.xml` entry
- 通过泛型 `BufRead` 解析逻辑直接交给 `quick-xml`

这样做的好处是：

- 避免额外磁盘写入
- 避免临时文件管理
- 不需要 `unsafe`
- 更适合后续大体积导入

## 项目结构

```text
src/
├── app/        # 应用服务层，编排 ingest / inspect / stats / query
├── cli/        # 命令定义与参数转换
├── domain/     # 领域模型与 DTO
├── infra/      # 日志、时间、哈希等基础设施
├── output/     # JSON 输出
├── parser/     # 输入、XML 读取、提取、规范化
└── storage/    # SQLite 连接、schema、写入、查询
```

## 已验证内容

当前已有集成测试覆盖：

- 基础 XML 导入
- zip 导入
- 去重
- `inspect`
- `stats`
- `query` JSON 输出
- `query` 拒绝多语句和修改性 SQL

运行：

```bash
cargo test
```

## 适用场景

- 将 Apple Health 导出转成可查询数据库
- 为本地 Agent 提供结构化健康数据访问层
- 为后续数据分析或上层工具提供稳定底座

## Roadmap


- 增量导入
- 导出 Parquet
- 更细粒度的错误报告
