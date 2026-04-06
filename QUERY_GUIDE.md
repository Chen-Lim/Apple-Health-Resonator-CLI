# Apple Health Resonator Query Guide for AI Agents

This document is for AI agents that need to use the `ahr` CLI safely and predictably.

All statements below were verified against the current Rust implementation and test suite in this repository on 2026-04-07.

## 1. Tool Contract

CLI name:

```bash
ahr
```

If the binary is not installed globally, run from the repository root with:

```bash
cargo run -- <subcommand> ...
```

Subcommands:

```bash
ahr ingest <PATH> [--db <DB>] [--batch-size <N>] [--quiet]
ahr inspect --db <DB>
ahr stats --db <DB>
ahr query --db <DB> --sql "<SQL>" [--limit <N>]
```

Behavior:

- `ingest` imports `export.xml` or `export.zip` into SQLite.
- `inspect` returns stable pretty JSON for schema-level orientation.
- `stats` returns compact JSON for fast summary checks.
- `query` returns a compact JSON array of row objects.

## 2. Required Agent Workflow

Use this sequence unless the user explicitly asks otherwise:

1. Confirm or choose an explicit SQLite path.
2. If the DB does not exist yet, run `ingest`.
3. Run `inspect` once to confirm table presence and high-level coverage.
4. Run `stats` when you need record volumes, top types, or recent-activity context.
5. Run `query` with a bounded, read-only SQL statement.

Do not start with an unbounded `SELECT *`.

## 3. Stable Output Shapes

### 3.1 `inspect`

Command:

```bash
ahr inspect --db ./health_data.db
```

Returns pretty JSON with this shape:

```json
{
  "tables": ["ingest_runs", "records", "workouts"],
  "record_count": 2,
  "workout_count": 1,
  "date_range": {
    "start": "2024-01-15T00:00:00Z",
    "end": "2024-01-16T01:00:00Z"
  },
  "sources": ["Apple Watch", "iPhone"],
  "record_types": ["HKQuantityTypeIdentifierStepCount"]
}
```

Semantics:

- `tables`: non-internal SQLite tables, sorted by name.
- `record_count`: `COUNT(*)` from `records`.
- `workout_count`: `COUNT(*)` from `workouts`.
- `date_range`: min `start_date` and max `end_date` across `records` and `workouts`.
- `sources`: distinct non-empty `source_name` values from `records` and `workouts`.
- `record_types`: distinct `record_type` values from `records`.

### 3.2 `stats`

Command:

```bash
ahr stats --db ./health_data.db
```

Returns compact JSON with this shape:

```json
{"total_records":2,"total_workouts":1,"top_types":[{"record_type":"HKQuantityTypeIdentifierStepCount","count":1}],"top_sources":[{"source_name":"Apple Watch","count":2}],"recent_activity":false}
```

Semantics:

- `top_types`: top 10 `records.record_type` values by count.
- `top_sources`: top 10 combined `source_name` values from `records` and `workouts`.
- `recent_activity`: whether any `start_date` exists within the last 30 days from current runtime time.

### 3.3 `query`

Command:

```bash
ahr query --db ./health_data.db --sql "SELECT record_type, value_num FROM records ORDER BY id" --limit 5
```

Returns compact JSON:

```json
[{"record_type":"HKQuantityTypeIdentifierStepCount","value_num":1234.0}]
```

Semantics:

- Output is always a JSON array.
- Each row is a JSON object keyed by the SQL result column name or alias.
- SQLite `NULL` becomes JSON `null`.
- SQLite integer becomes JSON integer.
- SQLite real becomes JSON number.
- SQLite text becomes JSON string.
- SQLite blob becomes lowercase hex string.

## 4. Actual Database Schema

Use these exact columns. Do not invent fields.

### 4.1 `records`

```text
id INTEGER PRIMARY KEY
record_type TEXT NOT NULL
value_text TEXT
value_num REAL
unit TEXT
source_name TEXT
source_version TEXT
device TEXT
creation_date TEXT
start_date TEXT NOT NULL
end_date TEXT NOT NULL
dedupe_key TEXT UNIQUE
```

Useful index-backed filters:

- `(record_type, start_date)`
- `(source_name, start_date)`

### 4.2 `workouts`

```text
id INTEGER PRIMARY KEY
workout_type TEXT NOT NULL
duration REAL
duration_unit TEXT
total_distance REAL
total_energy_burned REAL
source_name TEXT
creation_date TEXT
start_date TEXT NOT NULL
end_date TEXT NOT NULL
dedupe_key TEXT UNIQUE
```

Useful index-backed filter:

- `(workout_type, start_date)`

### 4.3 `ingest_runs`

```text
id INTEGER PRIMARY KEY
started_at TEXT NOT NULL
finished_at TEXT
input_path TEXT NOT NULL
records_inserted INTEGER
workouts_inserted INTEGER
records_skipped INTEGER
errors_count INTEGER
schema_version TEXT NOT NULL
```

## 5. SQL Safety Rules Enforced by the CLI

`ahr query` validates SQL before execution.

Allowed:

- A single read-only statement.
- Leading keyword `SELECT`.
- Leading keyword `WITH`, but only when it resolves to a `SELECT`.

Rejected:

- Empty SQL.
- Multiple statements such as `SELECT 1; DROP TABLE records`.
- Any non-read-only SQL.
- Statements whose first keyword is not `SELECT` or `WITH`.

Important operational detail:

- `--limit` truncates rows in CLI code after SQLite starts returning rows.
- Therefore, you should still put an explicit SQL `LIMIT` inside the query for performance and predictability.

Another important detail:

- The CLI does not support bind parameters.
- Write complete SQL with inline literals.

## 6. Query Writing Rules for Agents

Prefer these practices:

- Select only the columns needed for the answer.
- Always add SQL `LIMIT`, even if `--limit` is also set.
- Add `ORDER BY` whenever “latest”, “earliest”, “top”, or “recent” matters.
- Use explicit date predicates on `start_date` or `end_date`.
- Alias computed columns to stable names.
- Query `inspect` or `stats` first if table coverage is unclear.

Avoid these patterns:

- `SELECT *` on large scans unless the user explicitly needs full rows.
- Unbounded queries without SQL `LIMIT`.
- Assuming nested workout details exist. Current schema stores only workout top-level fields.
- Assuming joins to tables that do not exist.

## 7. Recommended Query Patterns

Latest records of one type:

```sql
SELECT record_type, value_num, unit, start_date, end_date, source_name
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
ORDER BY start_date DESC
LIMIT 20
```

Daily aggregate:

```sql
SELECT substr(start_date, 1, 10) AS day, SUM(value_num) AS total_steps
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
GROUP BY substr(start_date, 1, 10)
ORDER BY day DESC
LIMIT 30
```

Recent workouts:

```sql
SELECT workout_type, duration, duration_unit, total_distance, total_energy_burned, start_date
FROM workouts
ORDER BY start_date DESC
LIMIT 20
```

Import audit:

```sql
SELECT id, started_at, finished_at, input_path, records_inserted, workouts_inserted, records_skipped, errors_count, schema_version
FROM ingest_runs
ORDER BY id DESC
LIMIT 10
```

## 8. Error Handling Guidance

If `query` fails:

- On `multiple SQL statements are not allowed`: remove everything after the first statement.
- On `only read-only SELECT statements are allowed`: rewrite as pure `SELECT` or `WITH ... SELECT`.
- On `only SELECT statements are allowed`: remove DDL, DML, `PRAGMA`, or attachment logic.
- On `invalid SQL`: simplify the statement and verify table and column names from this guide.

If a result is unexpectedly empty:

- Check `inspect` for counts and date range.
- Check `stats` for available `record_type` values indirectly via `top_types`.
- Loosen filters before assuming data is absent.

## 9. Time and Data Assumptions

Imported datetimes are normalized to UTC RFC3339 strings such as:

```text
2024-01-15T00:30:00Z
```

When filtering by date, compare against UTC timestamps or ISO date prefixes intentionally.

Current ingestion scope:

- `records`: supported.
- `workouts`: top-level workout fields only.
- Nested `WorkoutEvent`, `WorkoutRoute`, and `MetadataEntry`: not stored as separate queryable tables.
