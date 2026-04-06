---
name: apple-health-resonator-cli
description: Use when an AI agent needs to ingest Apple Health export files into SQLite or answer questions from an existing Apple Health Resonator database through the local `ahr` CLI. Follow the repository query guide, inspect the database first, and write bounded read-only SQL for `ahr query`.
---

# Apple Health Resonator CLI

Use this skill when working inside this repository and the task involves:

- importing `export.xml` or `export.zip`
- inspecting an existing Apple Health SQLite database
- answering user questions with `ahr query`
- validating whether a proposed query is compatible with the actual CLI restrictions

## Read First

Open this file first:

- [QUERY_GUIDE.md](../../../QUERY_GUIDE.md)

That guide is the source of truth for:

- command syntax
- stable output formats
- actual schema
- query safety constraints
- recommended SQL patterns

## Execution Rules

Run commands from the repository root.

Prefer:

```bash
cargo run -- <subcommand> ...
```

Use the installed `ahr` binary only if it is already known to be available and points to this repo's current implementation.

Always pass an explicit `--db` path for repeatable work.

## Default Workflow

1. If no SQLite database exists yet, run `ingest`.
2. Run `inspect` before making schema assumptions.
3. Run `stats` when you need quick orientation on counts, top record types, or recent activity.
4. Write a bounded `query` with explicit `ORDER BY` and SQL `LIMIT`.
5. Parse the JSON result and answer from that output, not from assumptions.

## Command Templates

Ingest:

```bash
cargo run -- ingest /path/to/export.xml --db /path/to/health.db --quiet
```

Inspect:

```bash
cargo run -- inspect --db /path/to/health.db
```

Stats:

```bash
cargo run -- stats --db /path/to/health.db
```

Query:

```bash
cargo run -- query --db /path/to/health.db --sql "SELECT ..." --limit 50
```

## Query Discipline

Follow these rules:

- Only write one read-only `SELECT` or `WITH ... SELECT` statement.
- Add SQL `LIMIT` yourself; do not rely only on CLI `--limit`.
- Select only needed columns.
- Use aliases for computed columns.
- Use UTC-aware date filters against `start_date` or `end_date`.

Do not do these things:

- send multiple SQL statements
- use `INSERT`, `UPDATE`, `DELETE`, DDL, `PRAGMA`, or `ATTACH`
- assume bind parameters are supported
- assume tables beyond `records`, `workouts`, and `ingest_runs`
- assume nested workout entities are queryable

## Recovery Strategy

If a query fails, rewrite it instead of pushing harder:

- invalid SQL: simplify syntax and re-check column names
- multiple statements: keep only one statement
- non-read-only SQL: rewrite as pure `SELECT`
- empty result: inspect coverage first, then broaden filters

## Output Expectations

Expect:

- `inspect`: pretty JSON object
- `stats`: compact JSON object
- `query`: compact JSON array of objects

Treat `query` output as the only evidence for the final answer. If the user asks for trends or summaries, compute them from returned rows or write an aggregate SQL query.
