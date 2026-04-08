[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-2021-blue.svg)](https://www.rust-lang.org/)

Apple Health Resonator (`ahr`) is a local-first CLI for importing Apple Health exports into SQLite and querying them safely. It is designed for individual use as well as AI-agent workflows.

`ahr` converts `export.xml` or `export.zip` into a stable local database, keeps all processing on-device, and exposes read-only commands for inspection, summary, and SQL querying.

[中文说明](#中文说明) | [English](#english) | [AI Agent Guide](./AGENTS.md)

## 中文说明

### 项目简介

**Apple Health Resonator (`ahr`)** 是一个本地优先的命令行工具，用于将 Apple Health 导出的 `export.xml` 或 `export.zip` 导入 SQLite，并通过受控的只读接口进行检查、统计和查询。

适合两类场景：

- 个人在本地整理和分析健康数据
- AI Agent 在受控边界内读取和查询数据库

### 核心特性

- **本地处理**：不依赖云端服务，数据始终保留在本机。
- **高性能导入**：支持流式解析大型 `export.xml`，也可直接读取 `export.zip`。
- **稳定 Schema**：导入后生成可预测的 SQLite 结构，便于后续分析和自动化。
- **去重与时间标准化**：统一时间格式并避免重复导入。
- **面向 Agent 的输出**：`inspect` 提供格式化 JSON，`stats` 和 `query` 提供紧凑 JSON。
- **只读查询防护**：`query` 仅允许单条只读 SQL，阻止 `DROP`、`UPDATE`、`DELETE`、`ATTACH` 等语句。

### 安装

目前发布的预编译二进制面向 **Apple Silicon macOS**。

**Homebrew**

```bash
brew install Chen-Lim/tap/ahr
```

**手动下载**

从 [Releases](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/releases) 下载对应版本的二进制文件，并将其加入 `PATH`。

**源码编译**

```bash
cargo build --release
./target/release/ahr --help
```

### 快速开始

1. 导入 Apple Health 导出文件到本地 SQLite：

```bash
ahr ingest /path/to/export.zip --db ./health_data.db
```

2. 查看数据库摘要：

```bash
ahr inspect --db ./health_data.db
```

3. 执行只读 SQL 查询：

```bash
ahr query --db ./health_data.db --sql "SELECT record_type, value_num, start_date FROM records ORDER BY start_date DESC LIMIT 20" --limit 20
```

### CLI 概览

```bash
ahr ingest <PATH> [--db <DB>] [--batch-size <N>] [--quiet]
ahr inspect --db <DB>
ahr stats --db <DB>
ahr query --db <DB> --sql "<SQL>" [--limit <N>]
```

默认数据库路径为 `./health_data.db`。

### 数据模型

- `records`：一般健康记录，例如步数、心率、睡眠等。
- `workouts`：顶层 workout 会话数据。
- `ingest_runs`：每次导入的元数据与计数信息。

更完整的 Agent 使用约束、输出格式和 schema 说明见 [AGENTS.md](./AGENTS.md)。

## English

### Overview

**Apple Health Resonator (`ahr`)** is a local-first CLI for importing Apple Health exports (`export.xml` or `export.zip`) into SQLite and querying them through a controlled, read-only interface.

It is suitable for:

- individuals exploring their own health data locally
- AI agents that need a bounded, predictable database workflow

### Features

- **Local-first**: no cloud sync; data stays on the machine.
- **Fast ingestion**: stream-parses large `export.xml` files and can read `export.zip` directly.
- **Stable schema**: writes to a predictable SQLite structure for analysis and automation.
- **Deduplication and normalized timestamps**: avoids duplicate imports and standardizes time values.
- **Agent-oriented output**: `inspect` returns pretty JSON; `stats` and `query` return compact JSON.
- **Read-only SQL enforcement**: blocks mutating or unsafe statements such as `DROP`, `UPDATE`, `DELETE`, and `ATTACH`.

### Installation

Prebuilt binaries are currently provided for **Apple Silicon macOS**.

**Homebrew**

```bash
brew install Chen-Lim/tap/ahr
```

**Manual download**

Download the appropriate binary from [Releases](https://github.com/Chen-Lim/Apple-Health-Resonator-CLI/releases) and add it to your `PATH`.

**Build from source**

```bash
cargo build --release
./target/release/ahr --help
```

### Quick Start

1. Ingest an Apple Health export into a local SQLite database:

```bash
ahr ingest /path/to/export.zip --db ./health_data.db
```

2. Inspect database coverage:

```bash
ahr inspect --db ./health_data.db
```

3. Run a read-only SQL query:

```bash
ahr query --db ./health_data.db --sql "SELECT record_type, value_num, start_date FROM records ORDER BY start_date DESC LIMIT 20" --limit 20
```

### CLI Summary

```bash
ahr ingest <PATH> [--db <DB>] [--batch-size <N>] [--quiet]
ahr inspect --db <DB>
ahr stats --db <DB>
ahr query --db <DB> --sql "<SQL>" [--limit <N>]
```

The default database path is `./health_data.db`.

### Data Model

- `records`: general health records such as steps, heart rate, and sleep metrics.
- `workouts`: top-level workout session data.
- `ingest_runs`: metadata and counters for each ingestion run.

For exact agent operating rules, output shapes, SQL safety rules, and schema details, see [AGENTS.md](./AGENTS.md).

## AI Agent Guide

Agent-specific operating rules, output contracts, and query patterns are documented in [AGENTS.md](./AGENTS.md).
