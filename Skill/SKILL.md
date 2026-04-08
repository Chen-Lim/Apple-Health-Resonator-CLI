---
name: apple-health-resonator
description: >
  Use this skill whenever a user wants to work with Apple Health data using the `ahr` CLI tool
  (Apple Health Resonator). This includes: importing Apple Health export files (export.xml or
  export.zip) into a local SQLite database, inspecting or summarizing what health data exists,
  querying records or workouts with SQL, or analyzing any Apple Health metrics such as steps,
  heart rate, workouts, sleep, or other HealthKit data types.

  Trigger this skill whenever you see: an Apple Health export file, references to `ahr` or
  `health_data.db`, requests to analyze personal health data from Apple devices, or any task
  involving HKQuantityTypeIdentifier or Apple Watch health metrics. Always use this skill before
  writing any `ahr` commands — it contains the exact schema, output formats, and safety rules
  enforced by the CLI.
---

# Apple Health Resonator CLI Skill

`ahr` is a local Rust CLI that imports Apple Health exports into SQLite and exposes a
read-only query interface designed for Agent use.

**Binary name:** `ahr`  
**If not installed globally:** `cargo run -- <subcommand>` from the repo root  
**Verify:** `ahr --help` / `ahr --version`

---

## Required Workflow

Always follow this sequence unless the user explicitly asks to skip a step:

1. **Confirm the DB path** — establish an explicit `--db` path before doing anything else.
2. **Ingest** — if the DB doesn't exist yet, run `ahr ingest` on the export file.
3. **Inspect** — run `ahr inspect` once to confirm what data is present and the date range.
4. **Stats** (optional) — run `ahr stats` when you need record volumes or top types quickly.
5. **Query** — run `ahr query` with a bounded, read-only SQL statement.

Never start with an unbounded `SELECT *` on an unfamiliar database.

---

## Subcommands

### `ahr ingest`

```bash
ahr ingest <PATH> [--db <DB>] [--batch-size <N>] [--quiet]
```

- Accepts `export.xml` or `export.zip` (auto-detected).
- Default DB path: `./health_data.db`
- Default batch size: `10000`
- `--quiet` suppresses the progress bar.

### `ahr inspect`

```bash
ahr inspect --db <PATH>
```

Returns **pretty JSON** — use for orientation. See [Output Shapes](#output-shapes) below.

### `ahr stats`

```bash
ahr stats --db <PATH>
```

Returns **compact JSON** — use for fast summary checks before querying.

### `ahr query`

```bash
ahr query --db <PATH> --sql "<SQL>" [--limit <N>]
```

- Returns a compact **JSON array** of row objects.
- Default limit: `1000`. Always add an explicit SQL `LIMIT` too — `--limit` truncates *after* SQLite starts returning rows, so a SQL `LIMIT` is better for performance.
- No bind parameters — write inline literals.
- Read-only enforced. See [SQL Safety Rules](#sql-safety-rules).

---

## Output Shapes

### `inspect` output

```json
{
  "tables": ["ingest_runs", "records", "workouts"],
  "record_count": 123,
  "workout_count": 8,
  "date_range": { "start": "2024-01-01T00:00:00Z", "end": "2024-03-31T23:59:59Z" },
  "sources": ["Apple Watch", "iPhone"],
  "record_types": ["HKQuantityTypeIdentifierStepCount"]
}
```

### `stats` output

```json
{
  "total_records": 123,
  "total_workouts": 8,
  "top_types": [{"record_type": "HKQuantityTypeIdentifierStepCount", "count": 100}],
  "top_sources": [{"source_name": "Apple Watch", "count": 110}],
  "recent_activity": true
}
```

- `top_types` / `top_sources`: top 10 each.
- `recent_activity`: `true` if any `start_date` is within the last 30 days.

### `query` output

```json
[{"record_type": "HKQuantityTypeIdentifierStepCount", "value_num": 1234.0, "start_date": "2024-01-15T00:30:00Z"}]
```

- Always a JSON array (empty array `[]` for no results, never `null`).
- `NULL` → `null`, integer → JSON integer, real → JSON number, text → JSON string, blob → lowercase hex.

---

## Database Schema

### `records`

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `record_type` | TEXT NOT NULL | HKQuantityTypeIdentifier... |
| `value_text` | TEXT | for non-numeric values |
| `value_num` | REAL | for numeric values |
| `unit` | TEXT | e.g. `count`, `km`, `bpm` |
| `source_name` | TEXT | e.g. `Apple Watch` |
| `source_version` | TEXT | |
| `device` | TEXT | |
| `creation_date` | TEXT | UTC RFC3339 |
| `start_date` | TEXT NOT NULL | UTC RFC3339 |
| `end_date` | TEXT NOT NULL | UTC RFC3339 |
| `dedupe_key` | TEXT UNIQUE | |

**Index-backed filters:** `(record_type, start_date)`, `(source_name, start_date)`

### `workouts`

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `workout_type` | TEXT NOT NULL | e.g. `HKWorkoutActivityTypeRunning` |
| `duration` | REAL | |
| `duration_unit` | TEXT | e.g. `min` |
| `total_distance` | REAL | |
| `total_energy_burned` | REAL | |
| `source_name` | TEXT | |
| `creation_date` | TEXT | UTC RFC3339 |
| `start_date` | TEXT NOT NULL | UTC RFC3339 |
| `end_date` | TEXT NOT NULL | UTC RFC3339 |
| `dedupe_key` | TEXT UNIQUE | |

**Index-backed filter:** `(workout_type, start_date)`

> ⚠️ Only top-level workout fields are stored. `WorkoutEvent`, `WorkoutRoute`, and `MetadataEntry` are not queryable.

### `ingest_runs`

Tracks import history: `id`, `started_at`, `finished_at`, `input_path`, `records_inserted`, `workouts_inserted`, `records_skipped`, `errors_count`, `schema_version`.

---

## SQL Safety Rules

The CLI enforces these rules **before** execution. Violating them returns an error, not silence.

**Allowed:**
- Single `SELECT` statement
- `WITH ... SELECT` (CTE resolving to a SELECT)

**Rejected:**
- Multiple statements (`SELECT 1; DROP TABLE records`)
- Non-read-only SQL (INSERT, UPDATE, DELETE, DROP, PRAGMA, ATTACH)
- Empty SQL
- Anything whose first keyword is not `SELECT` or `WITH`

---

## Query Patterns

For more examples and edge cases, read → [`references/query-patterns.md`](references/query-patterns.md)

**Latest records of one type:**
```sql
SELECT record_type, value_num, unit, start_date, source_name
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
ORDER BY start_date DESC
LIMIT 20
```

**Daily aggregate:**
```sql
SELECT substr(start_date, 1, 10) AS day, SUM(value_num) AS total_steps
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
GROUP BY substr(start_date, 1, 10)
ORDER BY day DESC
LIMIT 30
```

**Recent workouts:**
```sql
SELECT workout_type, duration, duration_unit, total_distance, total_energy_burned, start_date
FROM workouts
ORDER BY start_date DESC
LIMIT 20
```

**Date-filtered query:**
```sql
SELECT record_type, value_num, unit, start_date
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierHeartRate'
  AND start_date >= '2024-03-01T00:00:00Z'
  AND start_date <  '2024-04-01T00:00:00Z'
ORDER BY start_date DESC
LIMIT 100
```

---

## Time Format

All datetimes in the DB are **UTC RFC3339**: `2024-01-15T00:30:00Z`

Apple Health's original timezone-offset format (`2024-01-15 08:30:00 +0800`) is normalized on ingest.

When writing date predicates, compare against UTC timestamps or ISO date prefixes (`substr(start_date, 1, 10)`).

---

## Error Recovery

| Error message | Fix |
|---|---|
| `multiple SQL statements are not allowed` | Remove everything after the first `;` |
| `only read-only SELECT statements are allowed` | Rewrite as pure `SELECT` or `WITH … SELECT` |
| `only SELECT statements are allowed` | Remove DDL, DML, PRAGMA, or ATTACH |
| `invalid SQL` | Verify column/table names against schema above |
| Empty result | Run `inspect` to check counts and date range; loosen date filters before assuming data is absent |

---

## What `ahr` Does NOT Do

- No GUI, no visualization, no cloud sync.
- No automatic SQL generation — you write the SQL.
- No embeddings or vector search.
- No nested workout details (WorkoutEvent, WorkoutRoute, MetadataEntry).
- No bind parameters in queries — use inline literals.
