# Apple Health Resonator — 数据库字段与查询指南

## 数据库表结构

### 表一：`records`（健康记录，主表）

| 列名 | 类型 | 是否必填 | 说明 |
|------|------|----------|------|
| `id` | INTEGER | 自动生成 | 主键 |
| `record_type` | TEXT | ✅ 必填 | 数据类型，见下方「常用 record_type」 |
| `value_text` | TEXT | 可为空 | 原始字符串值（类别类型用此字段，如睡眠阶段） |
| `value_num` | REAL | 可为空 | 数值（若 value_text 可解析为数字则自动填入，如心率、步数） |
| `unit` | TEXT | 可为空 | 单位（如 `count/min`、`km`、`kcal`） |
| `source_name` | TEXT | 可为空 | 数据来源（如 `Apple Watch`、`iPhone`） |
| `source_version` | TEXT | 可为空 | 来源 App 版本号 |
| `device` | TEXT | 可为空 | 设备字符串（完整设备描述） |
| `creation_date` | TEXT | 可为空 | 记录创建时间（UTC RFC3339） |
| `start_date` | TEXT | ✅ 必填 | 记录开始时间（UTC RFC3339） |
| `end_date` | TEXT | ✅ 必填 | 记录结束时间（UTC RFC3339） |
| `dedupe_key` | TEXT | 自动生成 | SHA256 去重键，重复导入不会产生重复数据 |

> **重要**：`value_text` 和 `value_num` 来自同一原始字段。
> - 若原始值是纯数字（如心率 `72`），则 `value_num = 72`，`value_text = "72"`
> - 若原始值是字符串（如睡眠阶段），则只有 `value_text` 有意义，`value_num` 为 NULL

---

### 表二：`workouts`（运动记录）

| 列名 | 类型 | 是否必填 | 说明 |
|------|------|----------|------|
| `id` | INTEGER | 自动生成 | 主键 |
| `workout_type` | TEXT | ✅ 必填 | 运动类型（如 `HKWorkoutActivityTypeRunning`） |
| `duration` | REAL | 可为空 | 运动时长（数值） |
| `duration_unit` | TEXT | 可为空 | 时长单位（通常为 `min`） |
| `total_distance` | REAL | 可为空 | 总距离（数值，单位见对应字段） |
| `total_energy_burned` | REAL | 可为空 | 消耗卡路里（数值） |
| `source_name` | TEXT | 可为空 | 数据来源 |
| `creation_date` | TEXT | 可为空 | 记录创建时间（UTC RFC3339） |
| `start_date` | TEXT | ✅ 必填 | 运动开始时间（UTC RFC3339） |
| `end_date` | TEXT | ✅ 必填 | 运动结束时间（UTC RFC3339） |
| `dedupe_key` | TEXT | 自动生成 | SHA256 去重键 |

> **注意**：`WorkoutEvent`、`WorkoutRoute`、`MetadataEntry` 等嵌套内容当前**不导入**，仅存储 Workout 顶层属性。

---

### 表三：`ingest_runs`（导入历史）

| 列名 | 类型 | 说明 |
|------|------|------|
| `id` | INTEGER | 主键 |
| `started_at` | TEXT | 导入开始时间 |
| `finished_at` | TEXT | 导入完成时间 |
| `input_path` | TEXT | 输入文件路径 |
| `records_inserted` | INTEGER | 本次插入的 records 数量 |
| `workouts_inserted` | INTEGER | 本次插入的 workouts 数量 |
| `records_skipped` | INTEGER | 因去重跳过的数量 |
| `errors_count` | INTEGER | 错误数量 |
| `schema_version` | TEXT | Schema 版本号 |

---

## 时间字段说明

所有时间均已**统一转为 UTC RFC3339**格式，例如：

```
2025-01-15T00:30:00Z
```

Apple Health 原始格式（如 `2025-01-15 08:30:00 +0800`）在导入时自动转换。

查询时推荐使用**固定 UTC 时间范围**，而不是 `datetime('now', '-N days')`（除非你的数据是最近的）：

```sql
-- 正确：固定日期范围
AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z'

-- 谨慎使用：依赖当前时间，历史数据可能查不到
AND start_date >= datetime('now', '-30 days')
```

---

## 常用 record_type 及查询方式

### 数值类型（用 `value_num` 查询）

| record_type | 说明 | 单位 |
|-------------|------|------|
| `HKQuantityTypeIdentifierHeartRate` | 心率 | count/min（BPM） |
| `HKQuantityTypeIdentifierStepCount` | 步数 | count |
| `HKQuantityTypeIdentifierDistanceWalkingRunning` | 步行/跑步距离 | km |
| `HKQuantityTypeIdentifierActiveEnergyBurned` | 活动消耗卡路里 | kcal |
| `HKQuantityTypeIdentifierBasalEnergyBurned` | 基础代谢卡路里 | kcal |
| `HKQuantityTypeIdentifierFlightsClimbed` | 爬楼层数 | count |
| `HKQuantityTypeIdentifierBodyMass` | 体重 | kg |
| `HKQuantityTypeIdentifierHeight` | 身高 | cm |
| `HKQuantityTypeIdentifierBloodPressureSystolic` | 收缩压 | mmHg |
| `HKQuantityTypeIdentifierBloodPressureDiastolic` | 舒张压 | mmHg |
| `HKQuantityTypeIdentifierOxygenSaturation` | 血氧饱和度 | % |
| `HKQuantityTypeIdentifierRespiratoryRate` | 呼吸频率 | count/min |
| `HKQuantityTypeIdentifierBodyTemperature` | 体温 | degC |
| `HKQuantityTypeIdentifierVO2Max` | 最大摄氧量 | mL/min·kg |
| `HKQuantityTypeIdentifierHeartRateVariabilitySDNN` | 心率变异性 HRV | ms |

### 类别类型（用 `value_text` 查询）

#### 睡眠分析 `HKCategoryTypeIdentifierSleepAnalysis`

| value_text | 说明 | 计入"实际睡眠"？ |
|------------|------|----------------|
| `HKCategoryValueSleepAnalysisAsleepCore` | 核心睡眠（浅睡） | ✅ 是 |
| `HKCategoryValueSleepAnalysisAsleepDeep` | 深度睡眠 | ✅ 是 |
| `HKCategoryValueSleepAnalysisAsleepREM` | REM 睡眠 | ✅ 是 |
| `HKCategoryValueSleepAnalysisAsleepUnspecified` | 未分类睡眠（旧版） | ✅ 是 |
| `HKCategoryValueSleepAnalysisAwake` | 清醒时段 | ❌ 否 |
| `HKCategoryValueSleepAnalysisInBed` | 在床上（不一定在睡） | ❌ 否 |

---

## 常用查询模板

### 心率

```bash
# 某月平均/最低/最高心率
cargo run -- query --db ./health_data.db \
  --sql "SELECT ROUND(AVG(value_num),1) as avg_hr, MIN(value_num) as min_hr, MAX(value_num) as max_hr FROM records WHERE record_type = 'HKQuantityTypeIdentifierHeartRate' AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z'"

# 按天查每日平均心率
cargo run -- query --db ./health_data.db \
  --sql "SELECT date(start_date) as day, ROUND(AVG(value_num),1) as avg_hr FROM records WHERE record_type = 'HKQuantityTypeIdentifierHeartRate' AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z' GROUP BY date(start_date) ORDER BY day" \
  --limit 31
```

### 睡眠

```bash
# 某月每日睡眠时长（小时）
cargo run -- query --db ./health_data.db \
  --sql "SELECT date(start_date) as day, ROUND(SUM((julianday(end_date)-julianday(start_date))*24),2) as sleep_hours FROM records WHERE record_type = 'HKCategoryTypeIdentifierSleepAnalysis' AND value_text IN ('HKCategoryValueSleepAnalysisAsleepCore','HKCategoryValueSleepAnalysisAsleepDeep','HKCategoryValueSleepAnalysisAsleepREM','HKCategoryValueSleepAnalysisAsleepUnspecified') AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z' GROUP BY date(start_date) ORDER BY day" \
  --limit 31

# 某月平均睡眠时长
cargo run -- query --db ./health_data.db \
  --sql "SELECT ROUND(AVG(daily_hours),2) as avg_sleep_hours FROM (SELECT date(start_date) as day, SUM((julianday(end_date)-julianday(start_date))*24) as daily_hours FROM records WHERE record_type = 'HKCategoryTypeIdentifierSleepAnalysis' AND value_text IN ('HKCategoryValueSleepAnalysisAsleepCore','HKCategoryValueSleepAnalysisAsleepDeep','HKCategoryValueSleepAnalysisAsleepREM','HKCategoryValueSleepAnalysisAsleepUnspecified') AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z' GROUP BY date(start_date))"
```

### 步数

```bash
# 按天查步数
cargo run -- query --db ./health_data.db \
  --sql "SELECT date(start_date) as day, CAST(SUM(value_num) AS INTEGER) as total_steps FROM records WHERE record_type = 'HKQuantityTypeIdentifierStepCount' AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z' GROUP BY date(start_date) ORDER BY day" \
  --limit 31
```

### 运动记录

```bash
# 查看所有运动类型
cargo run -- query --db ./health_data.db \
  --sql "SELECT workout_type, COUNT(*) as cnt FROM workouts GROUP BY workout_type ORDER BY cnt DESC"

# 查某月跑步记录（距离和消耗）
cargo run -- query --db ./health_data.db \
  --sql "SELECT date(start_date) as day, ROUND(total_distance,2) as km, ROUND(total_energy_burned,0) as kcal, ROUND(duration,1) as min FROM workouts WHERE workout_type = 'HKWorkoutActivityTypeRunning' AND start_date >= '2025-01-01T00:00:00Z' AND start_date < '2025-02-01T00:00:00Z' ORDER BY day" \
  --limit 31
```

### 探索数据

```bash
# 查看数据库中有哪些数据类型（按数量排序）
cargo run -- query --db ./health_data.db \
  --sql "SELECT record_type, COUNT(*) as cnt FROM records GROUP BY record_type ORDER BY cnt DESC" \
  --limit 50

# 查看数据的日期范围
cargo run -- query --db ./health_data.db \
  --sql "SELECT MIN(start_date) as earliest, MAX(start_date) as latest FROM records"

# 查看某个类型的原始样本
cargo run -- query --db ./health_data.db \
  --sql "SELECT * FROM records WHERE record_type = 'HKQuantityTypeIdentifierHeartRate' ORDER BY start_date DESC" \
  --limit 5
```
