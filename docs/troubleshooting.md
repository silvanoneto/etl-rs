# 🔧 Guia de Troubleshooting

## Visão Geral

Este guia aborda os problemas mais comuns encontrados ao usar a biblioteca ETLRS, suas causas raízes e soluções detalhadas. Inclui técnicas de debugging, ferramentas de diagnóstico e procedimentos de resolução para diferentes cenários de falha.

## Problemas Comuns

### 1. Problemas de Performance

#### Pipeline Lento
**Sintomas:**
- Processamento demorado de dados
- Alto uso de CPU ou memória
- Timeouts frequentes

**Diagnóstico:**
```rust
// src/diagnostics/performance.rs
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

pub struct PerformanceDiagnostics {
    start_time: Instant,
    checkpoints: Vec<(String, Instant)>,
}

impl PerformanceDiagnostics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            checkpoints: Vec::new(),
        }
    }
    
    pub fn checkpoint(&mut self, name: &str) {
        self.checkpoints.push((name.to_string(), Instant::now()));
        
        if let Some((prev_name, prev_time)) = self.checkpoints.iter().rev().nth(1) {
            let duration = self.checkpoints.last().unwrap().1.duration_since(*prev_time);
            info!("Performance checkpoint: {} -> {} took {:?}", prev_name, name, duration);
            
            if duration > Duration::from_secs(5) {
                warn!("Slow operation detected: {} -> {} took {:?}", prev_name, name, duration);
            }
        }
    }
    
    pub fn report(&self) {
        let total_duration = self.start_time.elapsed();
        info!("Total pipeline duration: {:?}", total_duration);
        
        for (i, (name, time)) in self.checkpoints.iter().enumerate() {
            let duration = if i == 0 {
                time.duration_since(self.start_time)
            } else {
                time.duration_since(self.checkpoints[i-1].1)
            };
            info!("Step {}: {} took {:?}", i+1, name, duration);
        }
    }
}

// Uso no pipeline
pub async fn diagnose_pipeline_performance() -> Result<(), Box<dyn std::error::Error>> {
    let mut diagnostics = PerformanceDiagnostics::new();
    
    diagnostics.checkpoint("start");
    
    // Extract phase
    let data = extract_data().await?;
    diagnostics.checkpoint("extract_complete");
    
    // Transform phase
    let transformed = transform_data(data).await?;
    diagnostics.checkpoint("transform_complete");
    
    // Load phase
    load_data(transformed).await?;
    diagnostics.checkpoint("load_complete");
    
    diagnostics.report();
    Ok(())
}
```

**Soluções:**

1. **Otimização de Batch Size:**
```rust
// config/performance.toml
[pipeline]
batch_size = 1000  # Teste diferentes valores: 500, 1000, 5000
max_parallel_tasks = 4  # Baseado no número de cores CPU

[memory]
buffer_size = 1024000  # 1MB buffer
max_memory_usage = 2147483648  # 2GB limit
```

2. **Paralelização:**
```rust
use rayon::prelude::*;
use tokio::task::JoinSet;

pub async fn parallel_transform(data: Vec<DataRecord>) -> Result<Vec<TransformedRecord>, Error> {
    let chunk_size = data.len() / num_cpus::get();
    let chunks: Vec<_> = data.chunks(chunk_size).collect();
    
    let mut join_set = JoinSet::new();
    
    for chunk in chunks {
        let chunk = chunk.to_vec();
        join_set.spawn(async move {
            chunk.into_par_iter()
                .map(|record| transform_record(record))
                .collect::<Result<Vec<_>, _>>()
        });
    }
    
    let mut results = Vec::new();
    while let Some(chunk_result) = join_set.join_next().await {
        results.extend(chunk_result??);
    }
    
    Ok(results)
}
```

#### Vazamentos de Memória
**Sintomas:**
- Uso de memória crescendo constantemente
- OOM (Out of Memory) kills
- Degradação de performance ao longo do tempo

**Diagnóstico:**
```rust
// src/diagnostics/memory.rs
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn};

pub struct MemoryTracker {
    allocations: AtomicUsize,
    peak_usage: AtomicUsize,
    current_usage: AtomicUsize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            allocations: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            current_usage: AtomicUsize::new(0),
        }
    }
    
    pub fn allocate(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed) + size;
        
        // Update peak usage if necessary
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
        
        if current > 1024 * 1024 * 1024 { // 1GB warning
            warn!("High memory usage: {} MB", current / 1024 / 1024);
        }
    }
    
    pub fn deallocate(&self, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
    }
    
    pub fn report(&self) {
        let current = self.current_usage.load(Ordering::Relaxed);
        let peak = self.peak_usage.load(Ordering::Relaxed);
        let allocations = self.allocations.load(Ordering::Relaxed);
        
        info!("Memory Report:");
        info!("  Current usage: {} MB", current / 1024 / 1024);
        info!("  Peak usage: {} MB", peak / 1024 / 1024);
        info!("  Total allocations: {}", allocations);
    }
}

// Wrapper para Vec que tracked memory
pub struct TrackedVec<T> {
    inner: Vec<T>,
    tracker: Arc<MemoryTracker>,
}

impl<T> TrackedVec<T> {
    pub fn new(tracker: Arc<MemoryTracker>) -> Self {
        Self {
            inner: Vec::new(),
            tracker,
        }
    }
    
    pub fn push(&mut self, item: T) {
        let size = std::mem::size_of::<T>();
        self.tracker.allocate(size);
        self.inner.push(item);
    }
}

impl<T> Drop for TrackedVec<T> {
    fn drop(&mut self) {
        let size = self.inner.len() * std::mem::size_of::<T>();
        self.tracker.deallocate(size);
    }
}
```

**Soluções:**

1. **Streaming Processing:**
```rust
use futures::stream::{Stream, StreamExt};

pub fn process_stream<S>(stream: S) -> impl Stream<Item = Result<ProcessedRecord, Error>>
where
    S: Stream<Item = Result<InputRecord, Error>>,
{
    stream
        .map(|result| async move {
            match result {
                Ok(record) => process_record(record).await,
                Err(e) => Err(e),
            }
        })
        .buffer_unordered(100) // Processa até 100 records em paralelo
        .chunks(1000) // Agrupa em batches de 1000
        .map(|chunk| {
            // Processa batch e libera memória
            let results = chunk.into_iter().collect::<Result<Vec<_>, _>>();
            results
        })
}
```

2. **Memory Pooling:**
```rust
use object_pool::{Pool, Reusable};

pub struct RecordPool {
    pool: Pool<Vec<u8>>,
}

impl RecordPool {
    pub fn new() -> Self {
        Self {
            pool: Pool::new(100, || Vec::with_capacity(1024)),
        }
    }
    
    pub fn get_buffer(&self) -> Reusable<Vec<u8>> {
        let mut buffer = self.pool.try_pull().unwrap_or_else(|| {
            Reusable::new(&self.pool, Vec::with_capacity(1024))
        });
        buffer.clear();
        buffer
    }
}
```

### 2. Problemas de Conectividade

#### Falhas de Conexão com Banco de Dados
**Sintomas:**
- Connection timeouts
- Connection pool exhausted
- Intermittent database errors

**Diagnóstico:**
```rust
// src/diagnostics/connection.rs
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::{info, warn, error};

pub struct ConnectionDiagnostics {
    pool: sqlx::PgPool,
}

impl ConnectionDiagnostics {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await?;
            
        Ok(Self { pool })
    }
    
    pub async fn test_connection(&self) -> Result<(), sqlx::Error> {
        let start = std::time::Instant::now();
        
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => {
                let duration = start.elapsed();
                info!("Database connection test successful: {:?}", duration);
                Ok(())
            }
            Err(e) => {
                error!("Database connection test failed: {}", e);
                Err(e)
            }
        }
    }
    
    pub async fn test_transaction(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        
        sqlx::query("CREATE TEMPORARY TABLE test_table (id INT)")
            .execute(&mut tx)
            .await?;
            
        sqlx::query("INSERT INTO test_table VALUES (1)")
            .execute(&mut tx)
            .await?;
            
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test_table")
            .fetch_one(&mut tx)
            .await?;
            
        tx.rollback().await?;
        
        if count.0 == 1 {
            info!("Transaction test successful");
            Ok(())
        } else {
            error!("Transaction test failed: unexpected count {}", count.0);
            Err(sqlx::Error::RowNotFound)
        }
    }
    
    pub fn get_pool_status(&self) -> (u32, u32) {
        (self.pool.size(), self.pool.num_idle())
    }
}
```

**Soluções:**

1. **Connection Pooling Otimizado:**
```toml
# config/database.toml
[connection_pool]
max_connections = 20
min_connections = 5
acquire_timeout = 30
idle_timeout = 600
max_lifetime = 1800
test_before_acquire = true

[retry_policy]
max_retries = 3
base_delay = 1000  # milliseconds
max_delay = 10000  # milliseconds
backoff_multiplier = 2.0
```

```rust
use tokio_retry::{strategy::ExponentialBackoff, Retry};

pub async fn execute_with_retry<T, F, Fut>(
    operation: F,
    max_retries: usize,
) -> Result<T, sqlx::Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let retry_strategy = ExponentialBackoff::from_millis(1000)
        .max_delay(Duration::from_secs(10))
        .take(max_retries);
    
    Retry::spawn(retry_strategy, || async {
        operation().await.map_err(|e| {
            warn!("Database operation failed, retrying: {}", e);
            e
        })
    }).await
}
```

2. **Circuit Breaker:**
```rust
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure_time: std::sync::Mutex<Option<Instant>>,
    is_open: AtomicBool,
    failure_threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure_time: std::sync::Mutex::new(None),
            is_open: AtomicBool::new(false),
            failure_threshold,
            timeout,
        }
    }
    
    pub async fn call<T, F, Fut>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        if self.is_open.load(Ordering::Relaxed) {
            if let Ok(last_failure) = self.last_failure_time.lock() {
                if let Some(last_failure) = *last_failure {
                    if last_failure.elapsed() < self.timeout {
                        return Err(CircuitBreakerError::CircuitOpen);
                    }
                    // Try to close circuit
                    self.is_open.store(false, Ordering::Relaxed);
                    self.failure_count.store(0, Ordering::Relaxed);
                }
            }
        }
        
        match operation().await {
            Ok(result) => {
                self.failure_count.store(0, Ordering::Relaxed);
                Ok(result)
            }
            Err(e) => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if failures >= self.failure_threshold {
                    self.is_open.store(true, Ordering::Relaxed);
                    if let Ok(mut last_failure) = self.last_failure_time.lock() {
                        *last_failure = Some(Instant::now());
                    }
                }
                
                Err(CircuitBreakerError::OperationFailed(e))
            }
        }
    }
}
```

### 3. Problemas de Dados

#### Dados Corrompidos
**Sintomas:**
- Parsing errors
- Validation failures
- Inconsistent data formats

**Diagnóstico:**
```rust
// src/diagnostics/data_integrity.rs
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Serialize, Deserialize)]
pub struct DataIntegrityReport {
    pub total_records: usize,
    pub valid_records: usize,
    pub invalid_records: usize,
    pub corrupted_records: usize,
    pub checksum: String,
    pub errors: Vec<DataError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataError {
    pub record_index: usize,
    pub error_type: String,
    pub message: String,
    pub record_sample: Option<String>,
}

pub struct DataValidator {
    errors: Vec<DataError>,
    checksums: Vec<String>,
}

impl DataValidator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            checksums: Vec::new(),
        }
    }
    
    pub fn validate_record(&mut self, index: usize, record: &str) -> bool {
        // Calculate checksum for integrity
        let mut hasher = Sha256::new();
        hasher.update(record.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());
        self.checksums.push(checksum);
        
        // Validate JSON format
        if let Err(e) = serde_json::from_str::<serde_json::Value>(record) {
            self.errors.push(DataError {
                record_index: index,
                error_type: "JSON_PARSE_ERROR".to_string(),
                message: e.to_string(),
                record_sample: Some(record.chars().take(100).collect()),
            });
            return false;
        }
        
        // Validate required fields
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(record) {
            if !value.is_object() {
                self.errors.push(DataError {
                    record_index: index,
                    error_type: "INVALID_STRUCTURE".to_string(),
                    message: "Record is not a JSON object".to_string(),
                    record_sample: Some(record.chars().take(100).collect()),
                });
                return false;
            }
            
            // Check for required fields
            let required_fields = ["id", "timestamp", "data"];
            for field in &required_fields {
                if !value.get(field).is_some() {
                    self.errors.push(DataError {
                        record_index: index,
                        error_type: "MISSING_FIELD".to_string(),
                        message: format!("Missing required field: {}", field),
                        record_sample: Some(record.chars().take(100).collect()),
                    });
                    return false;
                }
            }
        }
        
        true
    }
    
    pub fn generate_report(&self, total_records: usize) -> DataIntegrityReport {
        let invalid_records = self.errors.len();
        let valid_records = total_records - invalid_records;
        
        // Calculate overall checksum
        let mut hasher = Sha256::new();
        for checksum in &self.checksums {
            hasher.update(checksum.as_bytes());
        }
        let overall_checksum = format!("{:x}", hasher.finalize());
        
        DataIntegrityReport {
            total_records,
            valid_records,
            invalid_records,
            corrupted_records: self.errors.iter()
                .filter(|e| e.error_type == "CORRUPTED_DATA")
                .count(),
            checksum: overall_checksum,
            errors: self.errors.clone(),
        }
    }
}
```

**Soluções:**

1. **Data Sanitization:**
```rust
use regex::Regex;

pub struct DataSanitizer {
    email_regex: Regex,
    phone_regex: Regex,
    date_regex: Regex,
}

impl DataSanitizer {
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            email_regex: Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")?,
            phone_regex: Regex::new(r"^\+?[\d\s\-\(\)]{10,}$")?,
            date_regex: Regex::new(r"^\d{4}-\d{2}-\d{2}$")?,
        })
    }
    
    pub fn sanitize_email(&self, email: &str) -> Option<String> {
        let trimmed = email.trim().to_lowercase();
        if self.email_regex.is_match(&trimmed) {
            Some(trimmed)
        } else {
            None
        }
    }
    
    pub fn sanitize_phone(&self, phone: &str) -> Option<String> {
        let cleaned = phone.chars()
            .filter(|c| c.is_digit(10) || *c == '+')
            .collect::<String>();
            
        if self.phone_regex.is_match(&cleaned) {
            Some(cleaned)
        } else {
            None
        }
    }
    
    pub fn sanitize_date(&self, date: &str) -> Option<chrono::NaiveDate> {
        if self.date_regex.is_match(date) {
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
        } else {
            None
        }
    }
}
```

2. **Schema Validation:**
```rust
use jsonschema::{JSONSchema, Draft};
use serde_json::Value;

pub struct SchemaValidator {
    schema: JSONSchema,
}

impl SchemaValidator {
    pub fn new(schema_json: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema_value: Value = serde_json::from_str(schema_json)?;
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)?;
            
        Ok(Self { schema })
    }
    
    pub fn validate(&self, data: &Value) -> Result<(), Vec<String>> {
        match self.schema.validate(data) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                Err(error_messages)
            }
        }
    }
}

// Uso
const USER_SCHEMA: &str = r#"
{
  "type": "object",
  "properties": {
    "id": { "type": "integer", "minimum": 1 },
    "name": { "type": "string", "minLength": 1, "maxLength": 100 },
    "email": { "type": "string", "format": "email" },
    "age": { "type": "integer", "minimum": 0, "maximum": 150 },
    "created_at": { "type": "string", "format": "date-time" }
  },
  "required": ["id", "name", "email"],
  "additionalProperties": false
}
"#;
```

### 4. Problemas de Configuração

#### Configuração Inválida
**Sintomas:**
- Application startup failures
- Runtime configuration errors
- Environment variable issues

**Diagnóstico:**
```rust
// src/diagnostics/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigValidationReport {
    pub is_valid: bool,
    pub errors: Vec<ConfigError>,
    pub warnings: Vec<ConfigWarning>,
    pub environment_check: EnvironmentCheck,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigError {
    pub field: String,
    pub message: String,
    pub current_value: Option<String>,
    pub expected_format: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigWarning {
    pub field: String,
    pub message: String,
    pub recommendation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentCheck {
    pub required_vars: HashMap<String, bool>,
    pub optional_vars: HashMap<String, bool>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub available_memory: u64,
    pub cpu_cores: usize,
    pub disk_space: u64,
    pub os: String,
}

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate_config(config: &crate::Config) -> ConfigValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Validate database URL
        if config.database_url.is_empty() {
            errors.push(ConfigError {
                field: "database_url".to_string(),
                message: "Database URL cannot be empty".to_string(),
                current_value: None,
                expected_format: "postgresql://user:password@host:port/database".to_string(),
            });
        } else if !config.database_url.starts_with("postgresql://") {
            errors.push(ConfigError {
                field: "database_url".to_string(),
                message: "Invalid database URL format".to_string(),
                current_value: Some(config.database_url.clone()),
                expected_format: "postgresql://user:password@host:port/database".to_string(),
            });
        }
        
        // Validate batch size
        if config.batch_size == 0 {
            errors.push(ConfigError {
                field: "batch_size".to_string(),
                message: "Batch size cannot be zero".to_string(),
                current_value: Some(config.batch_size.to_string()),
                expected_format: "positive integer".to_string(),
            });
        } else if config.batch_size > 100000 {
            warnings.push(ConfigWarning {
                field: "batch_size".to_string(),
                message: "Very large batch size may cause memory issues".to_string(),
                recommendation: "Consider using a smaller batch size (1000-10000)".to_string(),
            });
        }
        
        // Validate thread count
        let cpu_cores = num_cpus::get();
        if config.max_threads > cpu_cores * 2 {
            warnings.push(ConfigWarning {
                field: "max_threads".to_string(),
                message: "Thread count exceeds recommended limit".to_string(),
                recommendation: format!("Consider using {} threads (2x CPU cores)", cpu_cores * 2),
            });
        }
        
        let environment_check = Self::check_environment();
        
        ConfigValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            environment_check,
        }
    }
    
    fn check_environment() -> EnvironmentCheck {
        let required_vars = [
            "DATABASE_URL",
            "RUST_LOG",
        ].iter()
        .map(|var| (var.to_string(), std::env::var(var).is_ok()))
        .collect();
        
        let optional_vars = [
            "REDIS_URL",
            "JWT_SECRET",
            "API_KEY",
        ].iter()
        .map(|var| (var.to_string(), std::env::var(var).is_ok()))
        .collect();
        
        let system_info = SystemInfo {
            available_memory: Self::get_available_memory(),
            cpu_cores: num_cpus::get(),
            disk_space: Self::get_disk_space(),
            os: std::env::consts::OS.to_string(),
        };
        
        EnvironmentCheck {
            required_vars,
            optional_vars,
            system_info,
        }
    }
    
    fn get_available_memory() -> u64 {
        // Platform-specific memory detection
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemAvailable:") {
                        if let Some(value) = line.split_whitespace().nth(1) {
                            return value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
        
        0 // Default if unable to detect
    }
    
    fn get_disk_space() -> u64 {
        // Simplified disk space check
        match std::fs::metadata(".") {
            Ok(_) => 1024 * 1024 * 1024, // Return 1GB as placeholder
            Err(_) => 0,
        }
    }
}
```

**Soluções:**

1. **Config Auto-correction:**
```rust
pub struct ConfigCorrector;

impl ConfigCorrector {
    pub fn auto_correct(config: &mut crate::Config) -> Vec<String> {
        let mut corrections = Vec::new();
        
        // Auto-correct batch size
        if config.batch_size == 0 {
            config.batch_size = 1000;
            corrections.push("Set batch_size to default value: 1000".to_string());
        } else if config.batch_size > 100000 {
            config.batch_size = 10000;
            corrections.push("Reduced batch_size to recommended value: 10000".to_string());
        }
        
        // Auto-correct thread count
        let cpu_cores = num_cpus::get();
        if config.max_threads == 0 {
            config.max_threads = cpu_cores;
            corrections.push(format!("Set max_threads to CPU core count: {}", cpu_cores));
        } else if config.max_threads > cpu_cores * 2 {
            config.max_threads = cpu_cores * 2;
            corrections.push(format!("Reduced max_threads to recommended value: {}", cpu_cores * 2));
        }
        
        // Set default log level if not specified
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info");
            corrections.push("Set RUST_LOG to default value: info".to_string());
        }
        
        corrections
    }
}
```

## Ferramentas de Debugging

### 1. Logger Estruturado

```rust
// src/diagnostics/logger.rs
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};
use tracing_appender::{rolling, non_blocking};

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let file_appender = rolling::daily("logs", "etlrs.log");
    let (non_blocking_file, _guard) = non_blocking(file_appender);
    
    let (non_blocking_stdout, _guard2) = non_blocking(std::io::stdout());
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    Registry::default()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking_stdout)
                .with_timer(UtcTime::rfc_3339())
                .with_target(true)
                .with_line_number(true)
                .with_file(true)
                .json()
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking_file)
                .with_timer(UtcTime::rfc_3339())
                .with_ansi(false)
                .json()
        )
        .init();
    
    Ok(())
}

// Macro para logging estruturado
#[macro_export]
macro_rules! log_operation {
    ($level:ident, $operation:expr, $($field:tt)*) => {
        tracing::$level!(
            operation = $operation,
            $($field)*
        );
    };
}

// Uso
log_operation!(info, "data_extraction", 
    records_count = 1000,
    source = "database",
    duration_ms = 250
);
```

### 2. Profiler Integrado

```rust
// src/diagnostics/profiler.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

pub struct Profiler {
    measurements: Arc<Mutex<HashMap<String, ProfileMeasurement>>>,
}

#[derive(Debug, Clone)]
pub struct ProfileMeasurement {
    pub call_count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn measure<T>(&self, name: &str, operation: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        
        if let Ok(mut measurements) = self.measurements.lock() {
            let measurement = measurements.entry(name.to_string())
                .or_insert(ProfileMeasurement {
                    call_count: 0,
                    total_duration: Duration::ZERO,
                    min_duration: Duration::MAX,
                    max_duration: Duration::ZERO,
                    avg_duration: Duration::ZERO,
                });
            
            measurement.call_count += 1;
            measurement.total_duration += duration;
            measurement.min_duration = measurement.min_duration.min(duration);
            measurement.max_duration = measurement.max_duration.max(duration);
            measurement.avg_duration = measurement.total_duration / measurement.call_count as u32;
        }
        
        result
    }
    
    pub async fn measure_async<T, F>(&self, name: &str, operation: F) -> T 
    where 
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation.await;
        let duration = start.elapsed();
        
        if let Ok(mut measurements) = self.measurements.lock() {
            let measurement = measurements.entry(name.to_string())
                .or_insert(ProfileMeasurement {
                    call_count: 0,
                    total_duration: Duration::ZERO,
                    min_duration: Duration::MAX,
                    max_duration: Duration::ZERO,
                    avg_duration: Duration::ZERO,
                });
            
            measurement.call_count += 1;
            measurement.total_duration += duration;
            measurement.min_duration = measurement.min_duration.min(duration);
            measurement.max_duration = measurement.max_duration.max(duration);
            measurement.avg_duration = measurement.total_duration / measurement.call_count as u32;
        }
        
        result
    }
    
    pub fn report(&self) -> HashMap<String, ProfileMeasurement> {
        self.measurements.lock().unwrap().clone()
    }
    
    pub fn reset(&self) {
        if let Ok(mut measurements) = self.measurements.lock() {
            measurements.clear();
        }
    }
}

// Macro para profiling
#[macro_export]
macro_rules! profile {
    ($profiler:expr, $name:expr, $operation:block) => {
        $profiler.measure($name, || $operation)
    };
}
```

## Scripts de Diagnóstico

### 1. Health Check Script

```bash
#!/bin/bash
# scripts/health-check.sh

set -e

echo "🔍 ETLRS Health Check"
echo "===================="

# Check system resources
echo "📊 System Resources:"
echo "CPU Cores: $(nproc)"
echo "Memory: $(free -h | grep '^Mem:' | awk '{print $2}')"
echo "Disk Space: $(df -h . | tail -1 | awk '{print $4}')"
echo ""

# Check environment variables
echo "🔧 Environment Variables:"
required_vars=("DATABASE_URL" "RUST_LOG")
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "❌ $var: Not set"
    else
        echo "✅ $var: Set"
    fi
done
echo ""

# Check database connectivity
echo "🗄️  Database Connectivity:"
if command -v psql &> /dev/null; then
    if psql "$DATABASE_URL" -c "SELECT 1;" &> /dev/null; then
        echo "✅ Database: Connected"
    else
        echo "❌ Database: Connection failed"
    fi
else
    echo "⚠️  psql not found, skipping database check"
fi
echo ""

# Check Redis connectivity (if configured)
echo "🔴 Redis Connectivity:"
if [ -n "$REDIS_URL" ]; then
    if command -v redis-cli &> /dev/null; then
        if redis-cli -u "$REDIS_URL" ping &> /dev/null; then
            echo "✅ Redis: Connected"
        else
            echo "❌ Redis: Connection failed"
        fi
    else
        echo "⚠️  redis-cli not found, skipping Redis check"
    fi
else
    echo "⚠️  REDIS_URL not set, skipping Redis check"
fi
echo ""

# Check application health endpoint
echo "🏥 Application Health:"
if command -v curl &> /dev/null; then
    if curl -f http://localhost:8080/health &> /dev/null; then
        echo "✅ Health endpoint: OK"
    else
        echo "❌ Health endpoint: Failed"
    fi
else
    echo "⚠️  curl not found, skipping health endpoint check"
fi

echo ""
echo "Health check completed!"
```

### 2. Log Analysis Script

```bash
#!/bin/bash
# scripts/analyze-logs.sh

LOG_FILE=${1:-"logs/etlrs.log"}
LAST_HOURS=${2:-1}

echo "📊 Log Analysis for last $LAST_HOURS hours"
echo "========================================"

if [ ! -f "$LOG_FILE" ]; then
    echo "❌ Log file not found: $LOG_FILE"
    exit 1
fi

# Calculate timestamp for filtering
if command -v date &> /dev/null; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        SINCE=$(date -u -v-${LAST_HOURS}H +"%Y-%m-%dT%H:%M:%S")
    else
        # Linux
        SINCE=$(date -u -d "$LAST_HOURS hours ago" +"%Y-%m-%dT%H:%M:%S")
    fi
    
    echo "Analyzing logs since: $SINCE"
    echo ""
    
    # Filter logs by timestamp
    RECENT_LOGS=$(awk -v since="$SINCE" '$0 >= since' "$LOG_FILE")
else
    # Fallback: analyze last 1000 lines
    RECENT_LOGS=$(tail -1000 "$LOG_FILE")
    echo "Analyzing last 1000 log lines"
    echo ""
fi

# Error analysis
echo "🚨 Error Summary:"
ERROR_COUNT=$(echo "$RECENT_LOGS" | grep -c '"level":"ERROR"' || true)
WARN_COUNT=$(echo "$RECENT_LOGS" | grep -c '"level":"WARN"' || true)
echo "Errors: $ERROR_COUNT"
echo "Warnings: $WARN_COUNT"
echo ""

if [ "$ERROR_COUNT" -gt 0 ]; then
    echo "Recent Errors:"
    echo "$RECENT_LOGS" | grep '"level":"ERROR"' | tail -5 | jq -r '.message' 2>/dev/null || echo "$RECENT_LOGS" | grep '"level":"ERROR"' | tail -5
    echo ""
fi

# Performance analysis
echo "⚡ Performance Metrics:"
echo "$RECENT_LOGS" | grep -o '"duration_ms":[0-9]*' | cut -d':' -f2 | awk '
{
    sum += $1; 
    count++; 
    if ($1 > max) max = $1; 
    if (min == 0 || $1 < min) min = $1
} 
END {
    if (count > 0) {
        printf "Operation Count: %d\n", count
        printf "Avg Duration: %.2f ms\n", sum/count
        printf "Min Duration: %d ms\n", min
        printf "Max Duration: %d ms\n", max
    } else {
        print "No performance metrics found"
    }
}'
echo ""

# Connection analysis
echo "🔗 Connection Events:"
CONN_ESTABLISHED=$(echo "$RECENT_LOGS" | grep -c "connection.*established" || true)
CONN_FAILED=$(echo "$RECENT_LOGS" | grep -c "connection.*failed" || true)
echo "Connections Established: $CONN_ESTABLISHED"
echo "Connection Failures: $CONN_FAILED"
echo ""

echo "Log analysis completed!"
```

Este guia de troubleshooting fornece uma base sólida para identificar, diagnosticar e resolver problemas comuns na biblioteca ETLRS, com ferramentas automatizadas e procedimentos estruturados.
