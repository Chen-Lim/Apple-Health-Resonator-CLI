[![CI](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/actions/workflows/ci.yml?query=branch%3Amaster)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-2021-blue.svg)](https://www.rust-lang.org/)

If you are AI Agent, this is for you [AI Agent Guide](./AGENTS.md)

[中文说明](#中文说明) | [English](#english) | [AI Agent Guide](./AGENTS.md)

## 中文说明

### 项目简介

**Apple Health Resonator (`ahr`)** 是一个本地优先的命令行工具，用于将 Apple Health 数据导入 **DuckDB**，并通过受控的只读接口进行检查、统计和查询。它接受两类输入：

- Apple Health 官方导出的 `export.xml` / `export.zip`
- iOS app **SimpleHealthExportCSV** 导出的 zip 包及其解压目录（v1.0.0 起，自动识别并做增量导入）

适合两类场景：

- 个人在本地整理和分析健康数据
- AI Agent 在受控边界内读取和查询数据库

### 核心特性

- **本地处理**：不依赖云端服务，数据始终保留在本机。
- **高性能导入**：支持流式解析大型 `export.xml`，也可直接读取 `export.zip`。
- **稳定 Schema**：导入后生成可预测的 DuckDB 结构，便于后续分析和自动化。
- **去重与时间标准化**：统一时间格式并避免重复导入。
- **增量导入**：兼容 iOS app `SimpleHealthExportCSV` 导出的 zip / 解压目录，按 `record_type` 做高水位跳过，详见 [`docs/incremental-ingest.md`](./docs/incremental-ingest.md)。
- **面向 Agent 的输出**：`inspect` 提供格式化 JSON，`stats` 和 `query` 提供紧凑 JSON。
- **只读查询防护**：`query` 仅允许单条只读 SQL，阻止 `DROP`、`UPDATE`、`DELETE`、`ATTACH` 等语句。

### 安装

目前发布的预编译二进制面向 **Apple Silicon macOS**。

**Homebrew**

```bash
brew install Chen-Lim/tap/ahr
```

**手动下载**

从 [Releases](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/releases) 下载对应版本的 Apple Silicon 二进制文件，并将其加入 `PATH`。

**源码编译**

```bash
cargo build --release
./target/release/ahr --help
```

### 快速开始

1. 首次导入：用 Apple Health 官方 `export.zip` 一次性建库：

```bash
ahr ingest /path/to/export.zip --db ./health_data.db --log ./health_data.ingest-errors.jsonl
```

2. 日常增量：把 SimpleHealthExportCSV 导出的 zip 直接喂给同一个 DB，重复执行即可——已存在的数据会被自动跳过：

```bash
ahr ingest ./HealthAll_2026-04-26_xx-xx_SimpleHealthExportCSV.zip --db ./health_data.db
```

3. 查看数据库摘要：

```bash
ahr inspect --db ./health_data.db
```

4. 执行只读 SQL 查询：

```bash
ahr query --db ./health_data.db --sql "SELECT record_type, value_num, start_date FROM records ORDER BY start_date DESC LIMIT 20" --limit 20
```

### CLI 概览

```bash
ahr ingest <PATH> [--db <DB>] [--log <PATH>] [--batch-size <N>] [--quiet] [--force]
ahr inspect --db <DB>
ahr stats   --db <DB>
ahr query   --db <DB> --sql "<SQL>" [--limit <N>]
```

- `<PATH>` 接受：`*.xml`、`*.zip`（官方导出或 SimpleHealthExportCSV bundle）、或解压后的 SimpleHealthExportCSV 目录。
- `--force`：对 SimpleHealthExportCSV bundle 关闭"同名包已导入"检查（仍然走 watermark + dedupe_key 兜底）。
- 默认数据库路径为 `./health_data.db`，默认 batch size `10000`，`query` 默认 `--limit 1000`。
- 增量导入的细节见 [`docs/incremental-ingest.md`](./docs/incremental-ingest.md)。

### 数据模型

- `records`：一般健康记录，例如步数、心率、睡眠等。
- `workouts`：顶层 workout 会话数据。
- `ingest_runs`：每次导入的元数据与计数信息。

更完整的 Agent 使用约束、输出格式和 schema 说明见 [AGENTS.md](./AGENTS.md)。

## English

### Overview

**Apple Health Resonator (`ahr`)** is a local-first CLI for importing Apple Health data into DuckDB and querying it through a controlled, read-only interface. It accepts two kinds of input:

- Apple Health's official `export.xml` / `export.zip`
- The zip bundles (and unpacked directories) produced by the iOS app **SimpleHealthExportCSV** — auto-detected and ingested incrementally since v1.0.0

It is suitable for:

- individuals exploring their own health data locally
- AI agents that need a bounded, predictable database workflow

### Features

- **Local-first**: no cloud sync; data stays on the machine.
- **Fast ingestion**: stream-parses large `export.xml` files and can read `export.zip` directly.
- **Stable schema**: writes to a predictable DuckDB structure for analysis and automation.
- **Deduplication and normalized timestamps**: avoids duplicate imports and standardizes time values.
- **Incremental ingest**: also reads SimpleHealthExportCSV bundles (zip or unpacked directory) and skips already-imported rows by `record_type` watermark — see [`docs/incremental-ingest.md`](./docs/incremental-ingest.md).
- **Agent-oriented output**: `inspect` returns pretty JSON; `stats` and `query` return compact JSON.
- **Read-only SQL enforcement**: blocks mutating or unsafe statements such as `DROP`, `UPDATE`, `DELETE`, and `ATTACH`.

### Installation

Prebuilt binaries are currently provided for **Apple Silicon macOS**.

**Homebrew**

```bash
brew install Chen-Lim/tap/ahr
```

**Manual download**

Download the appropriate Apple Silicon binary from [Releases](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/releases) and add it to your `PATH`.

**Build from source**

```bash
cargo build --release
./target/release/ahr --help
```

### Quick Start

1. Backfill: ingest the official Apple Health `export.zip` once to seed the DB:

```bash
ahr ingest /path/to/export.zip --db ./health_data.db --log ./health_data.ingest-errors.jsonl
```

2. Daily incremental: feed SimpleHealthExportCSV bundles into the same DB — re-running the same command is safe; already-imported rows are skipped automatically:

```bash
ahr ingest ./HealthAll_2026-04-26_xx-xx_SimpleHealthExportCSV.zip --db ./health_data.db
```

3. Inspect database coverage:

```bash
ahr inspect --db ./health_data.db
```

4. Run a read-only SQL query:

```bash
ahr query --db ./health_data.db --sql "SELECT record_type, value_num, start_date FROM records ORDER BY start_date DESC LIMIT 20" --limit 20
```

### CLI Summary

```bash
ahr ingest <PATH> [--db <DB>] [--log <PATH>] [--batch-size <N>] [--quiet] [--force]
ahr inspect --db <DB>
ahr stats   --db <DB>
ahr query   --db <DB> --sql "<SQL>" [--limit <N>]
```

- `<PATH>` accepts: `*.xml`, `*.zip` (Apple Health official **or** SimpleHealthExportCSV bundle), or an unpacked SimpleHealthExportCSV directory.
- `--force`: disables the "this bundle was already imported" guard for SimpleHealthExportCSV input (watermark + `dedupe_key` still apply).
- Defaults: DB path `./health_data.db`, batch size `10000`, `query --limit 1000`.
- See [`docs/incremental-ingest.md`](./docs/incremental-ingest.md) for the incremental ingest design.

### Data Model

- `records`: general health records such as steps, heart rate, and sleep metrics.
- `workouts`: top-level workout session data.
- `ingest_runs`: metadata and counters for each ingestion run.

For exact agent operating rules, output shapes, SQL safety rules, and schema details, see [AGENTS.md](./AGENTS.md).

## AI Agent Guide

Agent-specific operating rules, output contracts, and query patterns are documented in [AGENTS.md](./AGENTS.md).
