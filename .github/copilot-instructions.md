# ETLRS AI Assistant Guidelines

## Project Overview
ETLRS is an enterprise-grade ETL framework in Rust focused on type-safety, performance, and observability. It uses async traits for Extract-Transform-Load operations with a fluent builder pattern.

## Architecture & Key Patterns

### Core Pipeline Pattern
```rust
// Always use builder pattern with type-safe transformations
let pipeline = Pipeline::builder()
    .extract(CsvExtractor::new("input.csv"))
    .transform(FilterTransform::new(|row| { /* logic */ }))
    .load(JsonLoader::new("output.json"))
    .enable_metrics(true)
    .build();
```

### Trait System
- **Extractor**: `async fn extract() -> Result<Vec<DataRow>>` with batch support via `extract_batch()` and `has_more()`
- **Transformer**: `async fn transform(Vec<DataRow>) -> Result<Vec<DataRow>>` with optional validation via `validate()`
- **Loader**: `async fn load(Vec<DataRow>) -> Result<PipelineResult>` with health checks via `health_check()`
- **Validator**: `async fn validate(&[DataRow]) -> Result<Vec<String>>` for data quality checks
- **Aggregator**: `async fn aggregate(Vec<DataRow>) -> Result<Vec<DataRow>>` for data aggregation

All traits are `Send + Sync` and use `#[async_trait]`.

### Error Handling
Use the structured error system in `src/error.rs`:
- `ETLError` with specific variants: `Extract`, `Transform`, `Load`, `Config`
- Always use `Result<T>` type alias, not `std::result::Result`
- Errors are contextual with `#[from]` derivations
- Handle pipeline failures gracefully with `PipelineResult.errors` collection
- Use `thiserror` for custom error types with `#[error]` attributes

### Data Types & Value Conversion
- `DataRow = HashMap<String, DataValue>`
- `DataValue` enum: `String`, `Integer`, `Float`, `Boolean`, `Null`, `Array`, `Object`, `Date`, `DateTime`, `Timestamp`
- **Type-safe conversions**: Use `.as_string()`, `.as_integer()`, `.as_float()`, `.as_boolean()` for safe extraction
- **Smart parsing**: CSV extractor auto-detects types (int → float → bool → string)
- **Flexible conversions**: String values parse booleans as "true"/"false"/"1"/"0"/"yes"/"no"
- **From impls**: Direct conversion from Rust primitives to `DataValue`
- **Array/Object support**: Nested structures with `Vec<DataValue>` and `HashMap<String, DataValue>`
- **Date/Time support**: `NaiveDate`, `NaiveDateTime`, and `DateTime<Utc>` types for temporal data

## Development Workflows

### Building & Testing
```bash
cargo build                    # Standard build
cargo test                     # Run all tests
cargo bench                    # Run benchmarks (in benches/)
cargo run --example basic_pipeline  # Run examples
cargo fmt --all -- --check     # Check formatting (CI requirement)
cargo clippy                   # Run lints
```

### Feature Flags & Dependencies
Optional dependencies in `Cargo.toml` (must be explicitly enabled):
- `csv` for CSV support (`CsvExtractor`)
- `parquet` for Parquet support (`ParquetExtractor`, `ParquetLoader`) - Currently using Arrow/Parquet 55.2.0
- `sqlx` for database operations  
- `aws-*` for cloud integrations
- `candle-*` for ML features
- `arrow` for columnar data processing

### Component Builder Patterns
```rust
// Extractors with configuration chaining
let extractor = CsvExtractor::new("file.csv")
    .with_delimiter(b';')
    .with_headers(true)
    .with_quote_char(b'"');

// Loaders with output options
let loader = JsonLoader::new("output.json")
    .with_pretty(true)
    .with_append(false);

// Transformers with validation
let transform = FilterTransform::new(|row| { /* logic */ })
    .with_validator(Box::new(CustomValidator));
```

## Project-Specific Conventions

### Configuration Pattern
```rust
// Use ETLConfig builder for pipeline configuration
let config = ETLConfig::builder()
    .batch_size(1000)
    .parallel_workers(4)
    .enable_metrics(true)
    .build()?;

// Environment-based config loading
let config = ETLConfig::from_env()?;
```

### Transformation Patterns
```rust
// Simple closures for basic transforms
FilterTransform::new(|row| {
    row.get("age").and_then(|v| v.as_integer()).unwrap_or(0) >= 18
})

// Chained transformations via wrapper structs
struct ChainedTransform {
    transforms: Vec<Box<dyn Transformer>>,
}
// Then implement async iteration through transforms

// Async transformations for I/O operations
AsyncMapTransform::new(|row| Box::pin(async move {
    // async operations here
    row
}))

// Column operations
RenameColumnsTransform::new({
    let mut renames = HashMap::new();
    renames.insert("old_name".to_string(), "new_name".to_string());
    renames
})

SelectColumnsTransform::new(vec!["id", "name", "value"])
```

### Type Conversion & Validation
```rust
// Smart type parsing in CSV: int → float → bool → string hierarchy
// Boolean parsing: "true"/"false"/"1"/"0"/"yes"/"no"/"y"/"n" (case-insensitive)
// Null detection: empty string or "null" (case-insensitive)

// Type conversion transformations
ConvertTypesTransform::new({
    let mut conversions = HashMap::new();
    conversions.insert("age".to_string(), DataType::Integer);
    conversions
})

// Custom validation
struct RangeValidator {
    column: String,
    min: f64,
    max: f64,
}
```

### Aggregation Functions
Built-in aggregations: `Count`, `Sum`, `Average`, `Min`, `Max`, `First`, `Last`
```rust
AggregateTransform::new(
    vec!["category".to_string()], // group by columns
    {
        let mut aggs = HashMap::new();
        aggs.insert("value".to_string(), AggregateFunction::Sum);
        aggs
    }
)
```

### Logging & Observability
- Always use `tracing` crate, not `log`
- Pipeline execution includes built-in metrics tracking
- Use `tracing_subscriber::fmt::init()` in examples
- Metrics are automatically recorded via `PipelineMetrics`
- Health checks via `loader.health_check()` before execution
- Structured logging with spans: `#[tracing::instrument]`

### Multi-Output Pattern
```rust
// Use custom MultiLoader for writing to multiple destinations
struct MultiLoader {
    loaders: Vec<Box<dyn Loader>>,
}
// Implements parallel loading to all destinations
```

### Batch Processing Patterns
```rust
// Use execute_batch() for streaming large datasets
pipeline.execute_batch(1000).await?;

// Implement extract_batch() and has_more() for custom extractors
async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>>;
async fn has_more(&self) -> Result<bool>;

// Memory-efficient processing
pipeline.execute_streaming().await?; // Process one batch at a time
```

## Key Files & Directories

- **`src/traits.rs`**: Core trait definitions - start here for new components
- **`src/pipeline/mod.rs`**: Main pipeline orchestration and builder pattern
- **`src/types.rs`**: `DataRow`, `DataValue`, and result types
- **`src/config.rs`**: Configuration structures and builders
- **`src/error.rs`**: Error handling and custom error types
- **`src/extract/`**: Data extraction components
  - `csv.rs`: CSV file extraction
  - `json.rs`: JSON file extraction
  - `parquet.rs`: Apache Parquet extraction (feature-gated)
- **`src/transform/`**: Data transformation components
  - `common.rs`: Basic transformations (filter, map, aggregate, etc.)
- **`src/load/`**: Data loading components (organized by functionality)
  - `json.rs`: JSON and JSON Lines output
  - `console.rs`: Console/stdout output for debugging
  - `memory.rs`: In-memory storage for testing
  - `parquet.rs`: Apache Parquet output (feature-gated)
  - `common.rs`: Utility functions (`DataFormatter`)
- **`examples/`**: Working examples demonstrating patterns
- **`benches/`**: Performance benchmarks for optimization work
- **`docs/context/`**: Architecture documentation in Portuguese

## Integration Points

### Adding New Extractors/Transformers/Loaders
1. Implement the respective trait in `src/{extract|transform|load}/`
2. Create dedicated module file (e.g., `csv.rs`, `json.rs`, `console.rs`)
3. Add to `mod.rs` and re-export in `lib.rs` prelude
4. Follow async patterns with proper error handling
5. Add examples demonstrating usage
6. Include unit tests with mock data
7. Update benchmarks if performance-critical

### Module Organization Patterns
The project follows a clear module organization pattern:

#### Load Module Structure
```
src/load/
├── mod.rs          # Module declarations and exports
├── common.rs       # Shared utilities (DataFormatter)
├── console.rs      # ConsoleLoader for debug output
├── memory.rs       # MemoryLoader for testing
├── json.rs         # JsonLoader and JsonLinesLoader
└── parquet.rs      # ParquetLoader (feature-gated)
```

#### Component Implementation Pattern
```rust
// Each loader file follows this structure:
//! # Module Name
//! 
//! Brief description of the module's purpose

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{DataRow, PipelineResult};
use crate::traits::Loader;

/// Loader documentation in Portuguese
#[derive(Debug, Clone)]
pub struct MyLoader {
    config: ConfigType,
}

impl MyLoader {
    /// Constructor and builder methods
    pub fn new() -> Self { ... }
    pub fn with_option(mut self, value: Type) -> Self { ... }
}

#[async_trait]
impl Loader for MyLoader {
    // Implement all required trait methods
}

#[cfg(test)]
mod tests {
    // Comprehensive test coverage
}
```

#### Import Pattern Updates
When adding new modules, update these files:
1. **`src/load/mod.rs`**: Add `pub mod new_module;`
2. **`src/lib.rs`**: Add import in prelude section
3. **Pipeline tests**: Update imports if used in pipeline tests
````

## Testing Patterns
- Use `tempfile::NamedTempFile` for file-based tests
- Test both single execution and batch processing
- Verify metrics collection with `pipeline.get_metrics()`
- Include error scenarios in transform tests
- Mock external dependencies with `mockall` or custom traits
- Use `tokio::test` for async tests
- Property-based testing with `proptest` for data transformations

### Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_component_behavior() -> Result<()> {
        // Arrange
        let temp_file = NamedTempFile::new()?;
        
        // Act
        let result = component.process().await?;
        
        // Assert
        assert_eq!(result.processed_count, expected);
        Ok(())
    }
}
```

## Performance Considerations
- Default batch processing with `execute_batch()`
- Use `parallel_workers` config for concurrent processing
- Stream processing via `extract_batch()` and `has_more()`
- Built-in memory limits via `memory_limit_mb` config
- Lazy evaluation where possible
- Use `rayon` for CPU-bound parallel processing
- Profile with `cargo flamegraph` for optimization

### Performance Patterns
```rust
// Parallel transformation
struct ParallelTransform {
    workers: usize,
}

impl ParallelTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        let chunks = data.chunks(data.len() / self.workers);
        let handles: Vec<_> = chunks
            .map(|chunk| {
                tokio::spawn(async move {
                    // Process chunk
                })
            })
            .collect();
        // Join results
    }
}
```

## Critical Implementation Details

### Pipeline Execution Flow
1. Health checks via `loader.health_check()`
2. Extraction with optional batching
3. Transformation with error collection
4. Loading with finalization via `loader.finalize()`
5. Metrics recording in `PipelineMetrics`

### Builder Pattern Constraints
- Type-safe builders: `PipelineBuilder<E, T, L>` with phantom types
- Method chaining changes generic parameters: `.extract<NewE>()` → `PipelineBuilder<NewE, T, L>`
- Final `.build()` requires all components to be set
- Use `#[must_use]` on builder methods

### Memory Management
- Use `Arc<Mutex<PipelineMetrics>>` for thread-safe metrics
- Clone data sparingly; prefer moving `Vec<DataRow>` between stages
- `tempfile::NamedTempFile` auto-cleanup in tests
- Consider using `Cow<'a, str>` for string fields when appropriate
- Stream large files instead of loading into memory

### State Management
```rust
// Pipeline state tracking
#[derive(Debug, Clone)]
pub enum PipelineState {
    Idle,
    Extracting,
    Transforming,
    Loading,
    Completed,
    Failed(String),
}

// Use atomic state updates
let state = Arc::new(Mutex::new(PipelineState::Idle));
```

### Portuguese Documentation Convention
- User-facing documentation and examples use Portuguese
- Error messages and logs in Portuguese for end users
- Code comments and API docs can be in English
- README and architecture docs in Portuguese

## Documentation Requirements
- All public APIs need doc comments with examples
- Use Portuguese in user-facing documentation (following project pattern)
- Reference existing examples in docs
- Include error scenarios in API documentation
- Document performance characteristics (O(n), memory usage)
- Add `#[doc(hidden)]` for internal implementation details

### Documentation Pattern
```rust
/// Extrai dados de um arquivo CSV
/// 
/// # Exemplo
/// ```rust
/// let extractor = CsvExtractor::new("dados.csv")
///     .with_delimiter(b';');
/// ```
/// 
/// # Erros
/// Retorna erro se o arquivo não existir ou não puder ser lido
pub struct CsvExtractor { /* ... */ }
```

## Security Considerations
- Validate all input data sizes to prevent DoS
- Use `SecStr` for sensitive configuration values
- Sanitize file paths to prevent directory traversal
- Limit concurrent connections in database extractors
- Implement timeouts for all I/O operations

## Monitoring & Metrics
- Export metrics in Prometheus format
- Track: rows processed, errors, duration, memory usage
- Use `metrics` crate for instrumentation
- Implement custom metrics via `MetricsCollector` trait
- Alert on error rates above threshold

## Advanced Patterns

### Custom Serialization
```rust
// Implement custom DataValue serialization
impl DataValue {
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            DataValue::Array(values) => {
                serde_json::Value::Array(
                    values.iter().map(|v| v.to_json_value()).collect()
                )
            }
            // ...
        }
    }
}
```

### Plugin System
```rust
// Dynamic component loading
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn register(&self, registry: &mut ComponentRegistry);
}

// Use with dynamic libraries
let plugin = unsafe { lib.get::<fn() -> Box<dyn Plugin>>(b"create_plugin")? };
```

### Event-Driven Architecture
```rust
// Pipeline events for external monitoring
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    Started { pipeline_id: String },
    BatchProcessed { count: usize },
    Error { error: String },
    Completed { metrics: PipelineMetrics },
}

// Event emitter trait
#[async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit(&self, event: PipelineEvent) -> Result<()>;
}
```