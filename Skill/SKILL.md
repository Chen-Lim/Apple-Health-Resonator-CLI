---
name: apple-health-resonator
description: >
  Use this skill whenever a user wants to work with Apple Health data using the `ahr` CLI tool
  (Apple Health Resonator). This includes: importing Apple Health export files (`export.xml`,
  `export.zip`, or SimpleHealthExportCSV bundles) into a local DuckDB database, inspecting or
  summarizing what health data exists, querying records or workouts with SQL, or analyzing any
  Apple Health metrics such as steps, heart rate, workouts, sleep, or other HealthKit data types.

  Trigger this skill whenever you see: an Apple Health export file, a SimpleHealthExportCSV zip,
  references to `ahr` or `health_data.db`, requests to analyze personal health data from Apple
  devices, or any task involving HKQuantityTypeIdentifier or Apple Watch health metrics. Always
  use this skill before writing any `ahr` commands — it contains the exact schema, output
  formats, and safety rules enforced by the CLI.
---

# Apple Health Resonator CLI Skill

`ahr` is a local Rust CLI that imports Apple Health data into **DuckDB** and exposes a read-only
query interface designed for Agent use. Since v1.0.0 it also accepts SimpleHealthExportCSV
bundles and performs incremental ingest.

**Binary name:** `ahr`
**If not installed globally:** `cargo run -- <subcommand>` from the repo root
**Verify:** `ahr --help` / `ahr --version`

---

## Required Workflow

Always follow this sequence unless the user explicitly asks to skip a step:

1. **Confirm the DB path** — establish an explicit `--db` path before doing anything else.
2. **Ingest** — if the DB doesn't exist yet (or the user has a new bundle to import), run `ahr ingest` on the export file or bundle.
3. **Inspect** — run `ahr inspect` once to confirm what data is present and the date range.
4. **Stats** (optional) — run `ahr stats` when you need record volumes or top types quickly.
5. **Query** — run `ahr query` with a bounded, read-only SQL statement.

Never start with an unbounded `SELECT *` on an unfamiliar database.

---

## Subcommands

### `ahr ingest`

```bash
ahr ingest <PATH> [--db <DB>] [--log <PATH>] [--batch-size <N>] [--quiet] [--force]
```

`<PATH>` is auto-detected:

| Input shape | Detection | Path |
|---|---|---|
| `*.xml` | extension | Apple Health XML stream |
| `*.zip` containing `export.xml` / `导出.xml` | scans entries | Apple Health zip |
| `*.zip` containing `*_SimpleHealthExportCSV.csv` | scans entries | SimpleHealthExportCSV bundle (incremental) |
| Directory containing `*_SimpleHealthExportCSV.csv` | filesystem scan | Unpacked SimpleHealthExportCSV (incremental) |

Flags:

- `--db` default `./health_data.db`
- `--log` overrides the ingest-error JSONL path (default sits next to the DB; `--log /dev/null` suppresses on Unix)
- `--batch-size` default `10000`
- `--quiet` suppresses the progress spinner
- `--force` (CSV path only) bypasses the bundle-level "already ingested" guard

**Incremental cascade for SimpleHealthExportCSV input** — three layers, all derived from data already in the DB:

1. **Bundle level**: matches the input basename in `ingest_runs`; on hit the whole archive is skipped in milliseconds.
2. **Row-level watermark**: `MAX(end_date)` per `record_type` / `workout_type`; rows ≤ watermark are dropped before the writer.
3. **`dedupe_key` PK safety net**: surviving rows go through staging → `INSERT WHERE NOT EXISTS`.

XML / Apple Health zip paths are **not** affected by the watermark layer — they always go straight to the dedupe-by-key path.

CSV-mode runs print one extra summary line:

```
Files: 108 | Rows skipped (watermark): 250477
```

When the bundle-level guard fires you'll see this on stderr:

```
Archive already imported (matched a prior successful ingest_runs entry). Pass --force to re-import.
```

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
- `--limit` default `1000`. Always add an explicit SQL `LIMIT` too — `--limit` truncates *after* DuckDB starts streaming rows, so a SQL `LIMIT` is better for performance.
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

## Database Schema (DuckDB)

### `records`

| Column | Type | Notes |
|---|---|---|
| `record_type` | VARCHAR NOT NULL | e.g. `HKQuantityTypeIdentifierStepCount` |
| `value_text` | VARCHAR | for non-numeric values (e.g. category text) |
| `value_num` | DOUBLE | for numeric values |
| `unit` | VARCHAR | e.g. `count`, `km`, `bpm` |
| `source_name` | VARCHAR | e.g. `Apple Watch` |
| `source_version` | VARCHAR | |
| `device` | VARCHAR | |
| `creation_date` | VARCHAR | UTC RFC3339 |
| `start_date` | VARCHAR NOT NULL | UTC RFC3339 |
| `end_date` | VARCHAR NOT NULL | UTC RFC3339 |
| `dedupe_key` | VARCHAR PRIMARY KEY | sha256 of `(type, source, start, end, value, unit)` |

**Index-backed filters:** `(record_type, start_date)`, `(source_name, start_date)`

### `workouts`

| Column | Type | Notes |
|---|---|---|
| `workout_type` | VARCHAR NOT NULL | e.g. `HKWorkoutActivityTypeRunning` |
| `duration` | DOUBLE | |
| `duration_unit` | VARCHAR | e.g. `min`, `sec` |
| `total_distance` | DOUBLE | numeric only — units carry separately |
| `total_energy_burned` | DOUBLE | |
| `source_name` | VARCHAR | |
| `creation_date` | VARCHAR | UTC RFC3339 |
| `start_date` | VARCHAR NOT NULL | UTC RFC3339 |
| `end_date` | VARCHAR NOT NULL | UTC RFC3339 |
| `dedupe_key` | VARCHAR PRIMARY KEY | |

**Index-backed filter:** `(workout_type, start_date)`

> ⚠️ Only top-level workout fields are stored. `WorkoutEvent`, `WorkoutRoute`, and `MetadataEntry` are not queryable.

### `ingest_runs`

Tracks import history: `started_at`, `finished_at`, `input_path`, `records_inserted`, `workouts_inserted`, `records_skipped`, `errors_count`, `schema_version`. The CSV bundle-level guard queries this table by `input_path` basename.

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

**Most recent ingest_runs (useful for confirming an incremental import landed):**
```sql
SELECT started_at, input_path, records_inserted, workouts_inserted, records_skipped, errors_count
FROM ingest_runs
ORDER BY started_at DESC
LIMIT 5
```

---

## Time Format

All datetimes in the DB are **UTC RFC3339**: `2024-01-15T00:30:00Z`

Apple Health's original timezone-offset format (`2024-01-15 08:30:00 +0800`) is normalized on ingest. SimpleHealthExportCSV uses the same wire format and is normalized identically, so XML and CSV rows are directly comparable.

When writing date predicates, compare against UTC timestamps or ISO date prefixes (`substr(start_date, 1, 10)`). DuckDB also supports `strftime` and `date_trunc` if you prefer those.

---

## Error Recovery

| Error message | Fix |
|---|---|
| `multiple SQL statements are not allowed` | Remove everything after the first `;` |
| `only read-only SELECT statements are allowed` | Rewrite as pure `SELECT` or `WITH … SELECT` |
| `only SELECT statements are allowed` | Remove DDL, DML, PRAGMA, or ATTACH |
| `invalid SQL` | Verify column/table names against schema above |
| `unsupported input format` | `ingest` only accepts `.xml`, `.zip`, or a directory of SimpleHealthExportCSV files |
| `zip archive ... contains neither an Apple Health export xml nor SimpleHealthExportCSV files` | The zip isn't a recognized bundle; verify its contents |
| stderr: `Archive already imported ...` | The CSV bundle was previously ingested. Add `--force` to re-import; otherwise this is the expected idempotent path |
| Empty result | Run `inspect` to check counts and date range; loosen date filters before assuming data is absent |

---

## What `ahr` Does NOT Do

- No GUI, no visualization, no cloud sync.
- No automatic SQL generation — you write the SQL.
- No embeddings or vector search.
- No nested workout details (WorkoutEvent, WorkoutRoute, MetadataEntry).
- No bind parameters in queries — use inline literals.
- No write-side SQL — there is no way to insert / update / delete via `ahr query`. Mutations only happen through `ahr ingest`.
