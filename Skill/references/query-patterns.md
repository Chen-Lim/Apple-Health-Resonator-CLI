# Query Patterns Reference

Extended query examples for common Apple Health analysis tasks.

---

## Health Records

### Latest N readings of any metric
```sql
SELECT record_type, value_num, unit, start_date, source_name
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierRestingHeartRate'
ORDER BY start_date DESC
LIMIT 30
```

### Daily step count (last 30 days)
```sql
SELECT substr(start_date, 1, 10) AS day, SUM(value_num) AS total_steps
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierStepCount'
GROUP BY substr(start_date, 1, 10)
ORDER BY day DESC
LIMIT 30
```

### Weekly average heart rate
```sql
SELECT
  strftime('%Y-W%W', start_date) AS week,
  ROUND(AVG(value_num), 1) AS avg_bpm,
  COUNT(*) AS readings
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierHeartRate'
GROUP BY strftime('%Y-W%W', start_date)
ORDER BY week DESC
LIMIT 12
```

### Records within a specific date range
```sql
SELECT record_type, value_num, unit, start_date
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierActiveEnergyBurned'
  AND start_date >= '2024-01-01T00:00:00Z'
  AND start_date <  '2024-04-01T00:00:00Z'
ORDER BY start_date DESC
LIMIT 100
```

### Min/max/avg of a metric over all time
```sql
SELECT
  MIN(value_num) AS min_val,
  MAX(value_num) AS max_val,
  ROUND(AVG(value_num), 2) AS avg_val,
  COUNT(*) AS total_readings
FROM records
WHERE record_type = 'HKQuantityTypeIdentifierBodyMass'
```

### All distinct record types in the database
```sql
SELECT record_type, COUNT(*) AS count
FROM records
GROUP BY record_type
ORDER BY count DESC
LIMIT 50
```

### Records from a specific source
```sql
SELECT record_type, value_num, unit, start_date
FROM records
WHERE source_name = 'Apple Watch'
  AND record_type = 'HKQuantityTypeIdentifierHeartRateVariabilitySDNN'
ORDER BY start_date DESC
LIMIT 20
```

---

## Workouts

### Recent workouts with key stats
```sql
SELECT workout_type, duration, duration_unit, total_distance, total_energy_burned, start_date
FROM workouts
ORDER BY start_date DESC
LIMIT 20
```

### Workout summary by type
```sql
SELECT
  workout_type,
  COUNT(*) AS sessions,
  ROUND(SUM(duration), 1) AS total_duration,
  ROUND(AVG(duration), 1) AS avg_duration,
  ROUND(SUM(total_energy_burned), 0) AS total_calories
FROM workouts
GROUP BY workout_type
ORDER BY sessions DESC
```

### Workouts in a date range
```sql
SELECT workout_type, duration, total_distance, start_date
FROM workouts
WHERE start_date >= '2024-01-01T00:00:00Z'
  AND start_date <  '2024-04-01T00:00:00Z'
ORDER BY start_date DESC
LIMIT 50
```

### Monthly workout frequency
```sql
SELECT
  strftime('%Y-%m', start_date) AS month,
  COUNT(*) AS sessions,
  workout_type
FROM workouts
GROUP BY strftime('%Y-%m', start_date), workout_type
ORDER BY month DESC
LIMIT 30
```

---

## Import Audit

### Most recent ingest runs
```sql
SELECT id, started_at, finished_at, input_path, records_inserted, workouts_inserted, records_skipped, errors_count
FROM ingest_runs
ORDER BY id DESC
LIMIT 10
```

### Check for ingest errors
```sql
SELECT id, started_at, input_path, errors_count
FROM ingest_runs
WHERE errors_count > 0
ORDER BY id DESC
LIMIT 10
```

---

## Common HKQuantityTypeIdentifier Values

Reference for writing `WHERE record_type = '...'` predicates:

| Identifier | Meaning |
|---|---|
| `HKQuantityTypeIdentifierStepCount` | Steps |
| `HKQuantityTypeIdentifierHeartRate` | Heart rate (bpm) |
| `HKQuantityTypeIdentifierRestingHeartRate` | Resting heart rate |
| `HKQuantityTypeIdentifierHeartRateVariabilitySDNN` | HRV |
| `HKQuantityTypeIdentifierActiveEnergyBurned` | Active calories |
| `HKQuantityTypeIdentifierBasalEnergyBurned` | Resting calories |
| `HKQuantityTypeIdentifierDistanceWalkingRunning` | Walking/running distance |
| `HKQuantityTypeIdentifierFlightsClimbed` | Floors climbed |
| `HKQuantityTypeIdentifierBodyMass` | Body weight |
| `HKQuantityTypeIdentifierBodyMassIndex` | BMI |
| `HKQuantityTypeIdentifierOxygenSaturation` | Blood oxygen (%) |
| `HKQuantityTypeIdentifierRespiratoryRate` | Breaths per minute |
| `HKQuantityTypeIdentifierSleepAnalysis` | Sleep analysis (value_text) |
| `HKQuantityTypeIdentifierMindfulSession` | Mindfulness minutes |

> Actual types present in your DB depend on what data was recorded. Always run `ahr inspect` or `ahr stats` first, or query `SELECT DISTINCT record_type FROM records` to see what's available.
