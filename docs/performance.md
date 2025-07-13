# ⚡ Guia de Performance

## Visão Geral

Este guia aborda as melhores práticas para otimizar a performance de pipelines ETL construídos com a biblioteca ETLRS. Cobrimos desde otimizações básicas até técnicas avançadas de tuning, monitoramento e troubleshooting de performance.

## Princípios de Performance

### 1. Medição Antes da Otimização
- **Estabelecer baselines**: Sempre meça a performance atual antes de fazer otimizações
- **Identificar gargalos**: Use profiling para identificar onde o tempo é realmente gasto
- **Métricas relevantes**: Foque em métricas que impactam o negócio (throughput, latência, SLA)

### 2. Otimização Baseada em Dados
- **Profile primeiro**: Use ferramentas de profiling para identificar hotspots
- **Teste mudanças**: Sempre teste o impacto das otimizações em ambiente controlado
- **Monitore continuamente**: Performance pode degradar com o tempo

## Configuração de Performance

### 1. Configurações Básicas

```toml
# Cargo.toml
[profile.release]
lto = true                    # Link Time Optimization
codegen-units = 1            # Melhor otimização, build mais lento
panic = "abort"              # Reduce binary size
opt-level = 3                # Máxima otimização

[profile.release.package."*"]
opt-level = 3

# Para debug de performance
[profile.bench]
debug = true
opt-level = 3
```

### 2. Configuração de Runtime

```rust
// src/performance/config.rs
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    // Threading
    pub max_threads: Option<usize>,           // None = número de CPUs
    pub thread_stack_size: Option<usize>,     // Stack size por thread
    
    // Memory
    pub max_memory_usage: Option<usize>,      // Limite de memória em bytes
    pub batch_size: usize,                    // Tamanho do batch padrão
    pub enable_memory_mapping: bool,          // Memory-mapped files
    
    // I/O
    pub io_buffer_size: usize,               // Buffer size para I/O
    pub max_concurrent_connections: usize,    // Conexões simultâneas
    pub connection_pool_size: usize,         // Tamanho do pool de conexões
    
    // Processing
    pub enable_parallel_processing: bool,    // Processamento paralelo
    pub chunk_size: usize,                   // Tamanho dos chunks
    pub enable_streaming: bool,              // Streaming vs batch
    
    // Caching
    pub enable_cache: bool,                  // Sistema de cache
    pub cache_size_mb: usize,               // Tamanho do cache em MB
    pub cache_ttl_seconds: u64,             // TTL do cache
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_threads: None,
            thread_stack_size: Some(2 * 1024 * 1024), // 2MB
            max_memory_usage: None,
            batch_size: 10_000,
            enable_memory_mapping: true,
            io_buffer_size: 64 * 1024, // 64KB
            max_concurrent_connections: 100,
            connection_pool_size: 10,
            enable_parallel_processing: true,
            chunk_size: 1_000,
            enable_streaming: true,
            enable_cache: true,
            cache_size_mb: 256,
            cache_ttl_seconds: 3600,
        }
    }
}
```

## Otimizações por Estágio

### 1. Extração (Extract)

#### Otimizações de I/O
```rust
// Usar buffer sizes otimizados
let extractor = CsvExtractor::new("large_file.csv")
    .with_buffer_size(1024 * 1024)  // 1MB buffer
    .with_parallel_readers(4)        // 4 readers paralelos
    .with_memory_mapping(true);      // Memory-mapped I/O

// Para bases de dados
let db_extractor = DatabaseExtractor::new(connection_pool)
    .with_fetch_size(50_000)         // Fetch mais registros por vez
    .with_parallel_queries(8)        // Queries paralelas
    .with_prepared_statements(true); // Usar prepared statements
```

#### Particionamento de Dados
```rust
pub struct PartitionedExtractor {
    partitions: Vec<PartitionInfo>,
    parallel_workers: usize,
}

impl PartitionedExtractor {
    pub async fn extract_parallel(&self) -> Result<DatasetSnapshot, ETLError> {
        let tasks: Vec<_> = self.partitions
            .chunks(self.partitions.len() / self.parallel_workers)
            .map(|partition_chunk| {
                tokio::spawn(async move {
                    self.extract_partition(partition_chunk).await
                })
            })
            .collect();
        
        // Combinar resultados
        let mut combined_records = Vec::new();
        for task in tasks {
            let result = task.await??;
            combined_records.extend(result.records);
        }
        
        Ok(DatasetSnapshot {
            records: combined_records,
            total_count: combined_records.len(),
            checksum: self.calculate_checksum(&combined_records),
            timestamp: chrono::Utc::now(),
            schema_hash: self.calculate_schema_hash(&combined_records),
        })
    }
}
```

### 2. Transformação (Transform)

#### Processamento Paralelo
```rust
use rayon::prelude::*;

pub struct OptimizedTransformer {
    chunk_size: usize,
    max_parallelism: usize,
}

impl Transformer for OptimizedTransformer {
    async fn transform(&self, snapshot: DatasetSnapshot) -> Result<DatasetSnapshot, ETLError> {
        let transformed_records: Vec<Record> = snapshot.records
            .par_chunks(self.chunk_size)
            .map(|chunk| {
                chunk.iter()
                    .map(|record| self.transform_record(record))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        
        Ok(DatasetSnapshot {
            records: transformed_records,
            total_count: transformed_records.len(),
            checksum: self.calculate_checksum(&transformed_records),
            timestamp: chrono::Utc::now(),
            schema_hash: self.calculate_schema_hash(&transformed_records),
        })
    }
}
```

#### Streaming Transformations
```rust
use futures::stream::{self, StreamExt};

pub struct StreamingTransformer {
    buffer_size: usize,
}

impl StreamingTransformer {
    pub async fn transform_stream(&self, input_stream: impl Stream<Item = Record>) -> impl Stream<Item = Result<Record, ETLError>> {
        input_stream
            .map(|record| self.transform_record(record))
            .buffer_unordered(self.buffer_size)
    }
}
```

### 3. Carregamento (Load)

#### Batch Loading
```rust
pub struct BatchLoader {
    batch_size: usize,
    connection_pool: Arc<ConnectionPool>,
}

impl Loader for BatchLoader {
    async fn load(&self, snapshot: DatasetSnapshot) -> Result<LoadResult, ETLError> {
        let batches: Vec<_> = snapshot.records
            .chunks(self.batch_size)
            .collect();
        
        let mut total_loaded = 0;
        let semaphore = Arc::new(Semaphore::new(self.connection_pool.max_size()));
        
        for batch in batches {
            let permit = semaphore.acquire().await?;
            let connection = self.connection_pool.get().await?;
            
            tokio::spawn(async move {
                let _permit = permit;
                let result = Self::load_batch(&connection, batch).await;
                total_loaded += batch.len();
                result
            });
        }
        
        Ok(LoadResult {
            records_loaded: total_loaded,
            success: true,
            checksum: snapshot.checksum,
            timestamp: chrono::Utc::now(),
        })
    }
}
```

## Otimizações de Memória

### 1. Gestão de Memória
```rust
// Usar Arc/Rc para compartilhar dados read-only
pub struct SharedDataTransformer {
    lookup_table: Arc<HashMap<String, String>>,
}

// Streaming para datasets grandes
pub async fn process_large_dataset(file_path: &str) -> Result<(), ETLError> {
    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    while let Some(line) = lines.next_line().await? {
        let record = parse_record(&line)?;
        let transformed = transform_record(record).await?;
        load_record(transformed).await?;
        // Memória é liberada automaticamente a cada iteração
    }
    
    Ok(())
}
```

### 2. Memory Pooling
```rust
use object_pool::Pool;

pub struct RecordPool {
    pool: Pool<Vec<Record>>,
}

impl RecordPool {
    pub fn new() -> Self {
        Self {
            pool: Pool::new(|| Vec::with_capacity(1000), |vec| {
                vec.clear();
                vec.shrink_to(1000);
            }),
        }
    }
    
    pub fn get_buffer(&self) -> PoolGuard<Vec<Record>> {
        self.pool.pull()
    }
}
```

## Benchmarking e Profiling

### 1. Micro-benchmarks
```rust
// benches/transform_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_transformers(c: &mut Criterion) {
    let mut group = c.benchmark_group("transformers");
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        let data = generate_test_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("map_transformer", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let transformer = MapTransformer::new(black_box(|r| r));
                    black_box(transformer.transform(data.clone()))
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("parallel_transformer", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let transformer = ParallelTransformer::new(black_box(|r| r));
                    black_box(transformer.transform(data.clone()))
                })
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_transformers);
criterion_main!(benches);
```

### 2. Integration Benchmarks
```rust
// benches/pipeline_benchmark.rs
fn benchmark_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");
    group.measurement_time(Duration::from_secs(30));
    
    for dataset_size in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("csv_to_json", dataset_size),
            dataset_size,
            |b, &size| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let pipeline = PipelineBuilder::new()
                            .extract(CsvExtractor::new(&format!("test_data_{}.csv", size)))
                            .transform(FilterTransformer::new(|_| true))
                            .load(JsonLoader::new(&format!("output_{}.json", size)))
                            .build();
                        
                        black_box(pipeline.execute().await.unwrap())
                    })
                })
            },
        );
    }
    
    group.finish();
}
```

### 3. Profiling com perf
```bash
# Compilar com símbolos de debug
cargo build --release --features profiling

# Rodar com perf
perf record --call-graph=dwarf target/release/etlrs_example

# Analisar resultados
perf report
```

## Monitoramento de Performance

### 1. Métricas de Performance
```rust
use prometheus::{Histogram, Counter, Gauge};

pub struct PerformanceMetrics {
    // Throughput
    pub records_processed_total: Counter,
    pub records_per_second: Gauge,
    
    // Latência
    pub processing_duration: Histogram,
    pub stage_duration: Histogram,
    
    // Recursos
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
    pub disk_io: Counter,
    pub network_io: Counter,
}

impl PerformanceMetrics {
    pub fn record_pipeline_execution(&self, duration: Duration, records: usize) {
        self.processing_duration.observe(duration.as_secs_f64());
        self.records_processed_total.inc_by(records as u64);
        self.records_per_second.set(records as f64 / duration.as_secs_f64());
    }
    
    pub fn update_resource_usage(&self) {
        if let Ok(usage) = get_memory_usage() {
            self.memory_usage.set(usage as f64);
        }
        
        if let Ok(usage) = get_cpu_usage() {
            self.cpu_usage.set(usage);
        }
    }
}
```

### 2. Alertas de Performance
```rust
pub struct PerformanceAlert {
    pub threshold_throughput: f64,      // registros/segundo mínimo
    pub threshold_latency: Duration,    // latência máxima
    pub threshold_memory: usize,        // uso de memória máximo
    pub threshold_error_rate: f64,      // taxa de erro máxima
}

impl PerformanceAlert {
    pub fn check_thresholds(&self, metrics: &PerformanceSnapshot) -> Vec<Alert> {
        let mut alerts = Vec::new();
        
        if metrics.throughput < self.threshold_throughput {
            alerts.push(Alert::low_throughput(metrics.throughput));
        }
        
        if metrics.latency > self.threshold_latency {
            alerts.push(Alert::high_latency(metrics.latency));
        }
        
        if metrics.memory_usage > self.threshold_memory {
            alerts.push(Alert::high_memory_usage(metrics.memory_usage));
        }
        
        alerts
    }
}
```

## Troubleshooting de Performance

### 1. Problemas Comuns

#### Alto Uso de Memória
```rust
// Diagnóstico
fn diagnose_memory_usage() {
    // Verificar heap size
    let heap_size = get_heap_size();
    println!("Heap size: {} MB", heap_size / 1024 / 1024);
    
    // Verificar se há memory leaks
    let allocations = get_current_allocations();
    println!("Active allocations: {}", allocations);
    
    // Verificar cache usage
    let cache_usage = get_cache_usage();
    println!("Cache usage: {} MB", cache_usage / 1024 / 1024);
}

// Soluções
pub fn optimize_memory_usage(config: &mut PerformanceConfig) {
    // Reduzir batch size
    config.batch_size = config.batch_size / 2;
    
    // Habilitar streaming
    config.enable_streaming = true;
    
    // Reduzir cache size
    config.cache_size_mb = config.cache_size_mb / 2;
    
    // Limitar paralelismo
    config.max_threads = Some(num_cpus::get() / 2);
}
```

#### Baixo Throughput
```rust
fn diagnose_throughput() -> ThroughputAnalysis {
    let cpu_usage = get_cpu_usage();
    let io_wait = get_io_wait();
    let network_latency = get_network_latency();
    
    ThroughputAnalysis {
        cpu_bound: cpu_usage > 80.0,
        io_bound: io_wait > 20.0,
        network_bound: network_latency > Duration::from_millis(100),
        memory_bound: get_memory_pressure() > 0.8,
    }
}

fn optimize_throughput(analysis: &ThroughputAnalysis, config: &mut PerformanceConfig) {
    if analysis.cpu_bound {
        config.max_threads = Some(num_cpus::get() * 2);
        config.enable_parallel_processing = true;
    }
    
    if analysis.io_bound {
        config.io_buffer_size *= 2;
        config.max_concurrent_connections *= 2;
    }
    
    if analysis.network_bound {
        config.connection_pool_size *= 2;
        config.batch_size *= 2;
    }
}
```

### 2. Performance Testing

```rust
// Teste de carga
#[tokio::test]
async fn load_test() {
    let pipeline = create_test_pipeline();
    let start = Instant::now();
    
    // Simular carga pesada
    let tasks: Vec<_> = (0..100)
        .map(|i| {
            let pipeline = pipeline.clone();
            tokio::spawn(async move {
                let result = pipeline.execute().await;
                (i, result, Instant::now())
            })
        })
        .collect();
    
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }
    
    let total_duration = start.elapsed();
    let successful = results.iter().filter(|(_, result, _)| result.is_ok()).count();
    
    println!("Load test results:");
    println!("  Total duration: {:?}", total_duration);
    println!("  Successful executions: {}/100", successful);
    println!("  Success rate: {:.2}%", successful as f64);
    
    assert!(successful >= 95, "Success rate too low");
}

// Teste de stress
#[tokio::test]
async fn stress_test() {
    let initial_memory = get_memory_usage().unwrap();
    
    for i in 0..1000 {
        let pipeline = create_test_pipeline();
        pipeline.execute().await.unwrap();
        
        if i % 100 == 0 {
            let current_memory = get_memory_usage().unwrap();
            let memory_growth = current_memory - initial_memory;
            
            println!("Iteration {}: Memory growth: {} MB", 
                    i, memory_growth / 1024 / 1024);
            
            // Verificar memory leak
            assert!(memory_growth < 100 * 1024 * 1024, 
                   "Possible memory leak detected");
        }
    }
}
```

## Melhores Práticas

### 1. Design para Performance
- **Profile early**: Começe a medir performance desde o início
- **Choose algorithms wisely**: Use algoritmos com complexidade adequada
- **Minimize allocations**: Evite alocações desnecessárias
- **Use appropriate data structures**: HashMap vs BTreeMap vs Vec

### 2. Configuration Tuning
- **Start with defaults**: Use configurações padrão e otimize incrementalmente
- **Test in production-like environment**: Teste com dados e carga reais
- **Monitor continuously**: Performance pode degradar com o tempo

### 3. Scaling Strategies
- **Horizontal before vertical**: Scale out antes de scale up
- **Partition data**: Divida dados em partições menores
- **Cache wisely**: Use cache apenas onde faz diferença
- **Async all the way**: Evite blocking operations

Este guia fornece uma base sólida para otimizar a performance de pipelines ETL, garantindo que a biblioteca ETLRS atenda aos requisitos mais exigentes de throughput e latência.
