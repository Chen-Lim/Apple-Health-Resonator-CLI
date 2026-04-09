# AGENTS.md

Guidance for AI agents working with the `ahr` CLI in this repository.

## Scope

Use `ahr` when the task is to import Apple Health exports, inspect a generated SQLite database, summarize its contents, or answer questions with read-only SQL.

`ahr` is a local CLI. It imports `export.xml` or `export.zip` into SQLite and exposes bounded read-only commands for agent use.

## Ground Rules

- Always use an explicit `--db` path.
- If the database does not exist, ingest before querying.
- Run `inspect` once before exploratory querying.
- Prefer `stats` when you only need quick counts or top categories.
- Use bounded read-only SQL only.
- Never start with an unbounded `SELECT *` on an unfamiliar database.
- Always include an SQL `LIMIT`, even if `--limit` is also set.
- Do not assume tables or columns beyond those documented here.
- Do not assume nested workout data exists.

## Standard Workflow

1. Confirm the SQLite database path.
2. If needed, run `ahr ingest`.
3. Run `ahr inspect` for orientation.
4. Run `ahr stats` if summary metrics are useful.
5. Run `ahr query` with a narrow, read-only SQL statement.

## Command Contract

```bash
ahr ingest <PATH> [--db <DB>] [--log <PATH>] [--batch-size <N>] [--quiet]
ahr inspect --db <DB>
ahr stats --db <DB>
ahr query --db <DB> --sql "<SQL>" [--limit <N>]
```

If `ahr` is not installed globally, run from the repo root:

```bash
cargo run -- <subcommand> ...
```

Useful checks:

```bash
ahr --help
ahr --version
```

## Output Expectations

- `inspect`: pretty JSON for orientation.
- `stats`: compact JSON summary.
- `query`: compact JSON array of row objects.

`query` always returns a JSON array. `NULL` becomes `null`. Text stays text. Numeric values stay numeric.

## Schema Summary

### `records`

General health records.

Columns:

`id`, `record_type`, `value_text`, `value_num`, `unit`, `source_name`, `source_version`, `device`, `creation_date`, `start_date`, `end_date`, `dedupe_key`

Useful filters:

- `record_type`
- `source_name`
- `start_date`

### `workouts`

Top-level workout sessions only.

Columns:

`id`, `workout_type`, `duration`, `duration_unit`, `total_distance`, `total_energy_burned`, `source_name`, `creation_date`, `start_date`, `end_date`, `dedupe_key`

Useful filters:

- `workout_type`
- `start_date`

### `ingest_runs`

Import history and counters.

Columns:

`id`, `started_at`, `finished_at`, `input_path`, `records_inserted`, `workouts_inserted`, `records_skipped`, `errors_count`, `schema_version`

## SQL Safety

`ahr query` accepts:

- a single `SELECT`
- `WITH ... SELECT`

`ahr query` rejects:

- multiple statements
- mutating SQL such as `INSERT`, `UPDATE`, `DELETE`, `DROP`
- operational statements such as `PRAGMA` and `ATTACH`
- empty SQL

The CLI does not support bind parameters. Write complete SQL with inline literals.

## Query Conventions

- Select only the columns needed.
- Add `ORDER BY` whenever recency, ranking, or chronology matters.
- Use explicit date predicates on `start_date` or `end_date`.
- Alias computed columns to stable names.
- Prefer UTC RFC3339 timestamps in predicates, for example `2024-03-01T00:00:00Z`.

Example:

```sql
SELECT record_type, value_num, unit, start_date
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
ORDER BY start_date DESC
LIMIT 20
```
