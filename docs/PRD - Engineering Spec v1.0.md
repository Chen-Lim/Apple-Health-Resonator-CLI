 ## **0. Spec Freeze 状态**

  

当前版本为 **Spec Freeze v1.0**：

- 不再新增功能范围
    
- 允许实现层优化，但**不允许改变接口行为**
    
- 所有开发必须严格遵循本 PRD
    

---

## **1. 项目定义**

  

### **1.1 一句话定义**

  

> 一个使用 Rust 构建的高性能 CLI 工具，将 Apple Health XML 转换为 SQLite，并提供 Agent 可调用的数据访问接口。

---

### **1.2 核心价值**

- 解决 **10GB+ XML 无法高效处理的问题**
    
- 提供 **结构化健康数据底座**
    
- 成为 **本地 AI / Agent 的数据工具层**
    

---

## **2. 范围（Scope）**

  

### **In Scope（必须实现）**

- XML → SQLite ingest pipeline
    
- CLI interface（ingest / inspect / stats / query）
    
- 去重机制
    
- 基础 schema（records / workouts / ingest_runs）
    
- JSON 输出接口
    
- 进度与日志
    

---

### **Out of Scope（明确禁止）**

- GUI（PyQt / Web UI 等）
    
- 数据可视化
    
- 云同步
    
- 向量数据库 / embedding
    
- 自动 SQL 生成
    
- 实时数据同步
    

---

## **3. 系统架构（必须遵守）**

```
Input Layer
  └── zip/xml reader

Parsing Layer
  └── quick-xml (streaming)

Normalize Layer
  └── type mapping / datetime parsing / dedupe key

Buffer Layer
  └── batch buffer (in-memory)

Storage Layer
  └── SQLite (transaction)

Interface Layer
  └── CLI commands (clap)

Consumer
  └── Agent / LLM
```

---

## **4. 技术栈（Spec Locked）**

  

### **必须使用**

|**组件**|**技术**|
|---|---|
|Language|Rust 2021|
|XML Parser|quick-xml (streaming only)|
|DB|rusqlite|
|CLI|clap|
|Error|anyhow|

---

### **推荐**

- chrono
    
- serde
    
- serde_json
    
- indicatif
    

---

### **明确不在 MVP 中**

- ❌ Polars
    
- ❌ Rayon（除非性能瓶颈确认）
    

  

👉 原则：**先稳定，再优化**

---

## **5. CLI 规范（必须严格一致）**

  

### **Binary Name**

```
ahr
```

---

## **5.1 ingest**

```
ahr ingest <path> [--db <path>]
```

### **行为**

- 自动识别 zip/xml
    
- 默认输出：./health_data.db
    
- 显示：
    
    - 进度条
        
    - 已处理记录数
        
    - 错误数
        
    - 总耗时
        
    

---

## **5.2 inspect**

```
ahr inspect --db <path>
```

### **输出（必须稳定）**

```
{
  "tables": [...],
  "record_count": ...,
  "workout_count": ...,
  "date_range": {
    "start": "...",
    "end": "..."
  },
  "sources": [...],
  "record_types": [...]
}
```

---

## **5.3 stats**

```
ahr stats --db <path>
```

### **输出（面向 Agent，必须紧凑）**

```
{
  "total_records": ...,
  "total_workouts": ...,
  "top_types": [...],
  "top_sources": [...],
  "recent_activity": true
}
```

---

## **5.4 query**

```
ahr query --db <path> --sql "<SQL>" [--limit N]
```

### **规则**

- 只允许 SELECT
    
- 自动附加 LIMIT（默认 1000）
    
- 输出 JSON array
    
- 禁止返回 schema 信息
    

---

## **6. 数据库设计（必须严格实现）**

---

## **6.1 records 表**

```
CREATE TABLE records (
  id INTEGER PRIMARY KEY,
  record_type TEXT NOT NULL,
  value_text TEXT,
  value_num REAL,
  unit TEXT,
  source_name TEXT,
  source_version TEXT,
  device TEXT,
  creation_date TEXT,
  start_date TEXT NOT NULL,
  end_date TEXT NOT NULL,
  dedupe_key TEXT UNIQUE
);
```

---

## **6.2 workouts 表**

```
CREATE TABLE workouts (
  id INTEGER PRIMARY KEY,
  workout_type TEXT NOT NULL,
  duration REAL,
  duration_unit TEXT,
  total_distance REAL,
  total_energy_burned REAL,
  source_name TEXT,
  creation_date TEXT,
  start_date TEXT NOT NULL,
  end_date TEXT NOT NULL,
  dedupe_key TEXT UNIQUE
);
```

---

## **6.3 ingest_runs 表**

```
CREATE TABLE ingest_runs (
  id INTEGER PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  input_path TEXT NOT NULL,
  records_inserted INTEGER,
  workouts_inserted INTEGER,
  records_skipped INTEGER,
  errors_count INTEGER,
  schema_version TEXT NOT NULL
);
```

---

## **6.4 索引（必须）**

```
CREATE INDEX idx_records_type_date ON records(record_type, start_date);
CREATE INDEX idx_records_source_date ON records(source_name, start_date);
CREATE INDEX idx_workouts_type_date ON workouts(workout_type, start_date);
```

---

## **7. 数据处理规则（关键实现）**

---

## **7.1 XML 解析**

- 使用 quick-xml Reader
    
- **必须 streaming**
    
- 禁止 DOM / load 全文件
    

---

## **7.2 字段映射**

|**XML Attribute**|**DB Column**|
|---|---|
|type|record_type|
|value|value_text / value_num|
|unit|unit|
|sourceName|source_name|
|startDate|start_date|
|endDate|end_date|

---

## **7.3 时间标准**

- 全部转换为 UTC
    
- 格式：RFC 3339（ISO 8601）
    

---

## **7.4 去重逻辑**

  

生成：

```
dedupe_key = hash(
  type + source + start_date + end_date + value + unit
)
```

---

## **7.5 批量写入**

- 每 10,000 条 commit 一次
    
- 使用 transaction
    
- 使用 prepared statement
    

---

## **8. 性能约束（必须达标）**

|**指标**|**要求**|
|---|---|
|内存|不随文件大小线性增长|
|文件|支持 ≥10GB XML|
|稳定性|不崩溃|
|容错|单条错误不影响整体|

---

## **9. 错误处理**

  

必须分类：

- Parse Error
    
- DB Error
    
- IO Error
    

  

CLI 输出：

```
Errors: 123 (skipped)
```

---

## **10. 日志与输出**

  

### **控制台**

- 进度条
    
- 实时统计
    

  

### **可选（非必须）**

- log file
    

---

## **11. 代码结构建议（给工程团队）**

```
src/
 ├── main.rs
 ├── cli/
 ├── ingest/
 │    ├── parser.rs
 │    ├── normalizer.rs
 │    ├── writer.rs
 ├── db/
 │    ├── schema.rs
 │    ├── connection.rs
 ├── query/
 ├── utils/
```

---

## **12. 版本定义**

---

## **MVP（必须完成）**

- ingest
    
- inspect
    
- stats
    
- query
    
- SQLite schema
    
- dedupe
    
- streaming parse
    

---

## **V1.1（不在当前 Sprint）**

- 更强统计
    
- 增量导入
    
- Parquet export
    

---

## **13. 验收标准（Definition of Done）**

  

项目完成必须满足：

- 成功导入 10GB Apple Health 数据
    
- 内存稳定（无爆炸增长）
    
- SQLite 可查询
    
- CLI 输出符合 Spec
    
- Agent 可调用 query/stats
    
- 无 fatal crash
    

---

## 14. 开发约束

  

### **必须遵守**

- 不允许引入 GUI
    
- 不允许引入复杂依赖（如 Polars）
    
- 不允许破坏 CLI 输出结构
    
- 不允许一次性加载 XML
    
