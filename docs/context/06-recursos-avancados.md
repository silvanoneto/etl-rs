# Recursos e Capacidades Avançadas

A biblioteca ETLRS oferece um conjunto robusto de recursos avançados que permitem construir pipelines ETL sofisticados, escaláveis e resilientes. Esta seção explora funcionalidades como processamento paralelo, sistema de cache inteligente, recuperação automática de falhas, observabilidade completa e integração com sistemas externos, demonstrando como cada recurso contribui para a criação de soluções ETL enterprise-grade.

## 1. Processamento Paralelo e Concorrência

### Paralelização de Transformações

```rust
// src/transform/parallel.rs
use rayon::prelude::*;
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ParallelTransformer<T> 
where 
    T: Transformer + Send + Sync + Clone + 'static,
{
    transformer: T,
    max_concurrency: usize,
    chunk_size: usize,
}

impl<T> ParallelTransformer<T> 
where 
    T: Transformer + Send + Sync + Clone + 'static,
{
    pub fn new(transformer: T) -> Self {
        Self {
            transformer,
            max_concurrency: num_cpus::get(),
            chunk_size: 1000,
        }
    }
    
    pub fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = max_concurrency;
        self
    }
    
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
}

#[async_trait]
impl<T> Transformer for ParallelTransformer<T> 
where 
    T: Transformer + Send + Sync + Clone + 'static,
{
    async fn transform(&self, mut snapshot: DatasetSnapshot) -> Result<DatasetSnapshot, Box<dyn std::error::Error>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));
        
        // Dividir em chunks para processamento paralelo
        let chunks: Vec<_> = snapshot.records
            .chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        let mut handles = Vec::new();
        
        for chunk in chunks {
            let transformer = self.transformer.clone();
            let semaphore = semaphore.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let chunk_snapshot = DatasetSnapshot {
                    records: chunk,
                    total_count: 0, // Será recalculado
                    checksum: String::new(), // Será recalculado
                    timestamp: chrono::Utc::now(),
                    schema_hash: String::new(), // Será recalculado
                };
                
                transformer.transform(chunk_snapshot).await
            });
            
            handles.push(handle);
        }
        
        // Aguardar e combinar resultados
        let mut combined_records = Vec::new();
        
        for handle in handles {
            let result = handle.await??;
            combined_records.extend(result.records);
        }
        
        snapshot.records = combined_records;
        snapshot.total_count = snapshot.records.len();
        snapshot.timestamp = chrono::Utc::now();
        
        Ok(snapshot)
    }
}
```

### Pool de Workers Assíncrono

```rust
// src/workers/pool.rs
use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;

pub struct WorkerPool {
    workers: Vec<Worker>,
    task_sender: mpsc::UnboundedSender<Task>,
    metrics: Arc<Mutex<WorkerMetrics>>,
}

#[derive(Debug)]
pub struct Worker {
    id: usize,
    status: WorkerStatus,
    current_task: Option<String>,
    tasks_completed: usize,
}

#[derive(Debug)]
pub enum WorkerStatus {
    Idle,
    Busy,
    Error,
}

pub struct Task {
    id: String,
    payload: TaskPayload,
    response_sender: oneshot::Sender<TaskResult>,
}

pub enum TaskPayload {
    Transform {
        records: Vec<Record>,
        transformer_type: String,
        config: HashMap<String, serde_json::Value>,
    },
    Load {
        records: Vec<Record>,
        destination: String,
        config: HashMap<String, serde_json::Value>,
    },
    Custom {
        operation: String,
        data: serde_json::Value,
    },
}

pub enum TaskResult {
    Success(serde_json::Value),
    Error(String),
}

impl WorkerPool {
    pub async fn new(worker_count: usize) -> Self {
        let (task_sender, mut task_receiver) = mpsc::unbounded_channel::<Task>();
        let metrics = Arc::new(Mutex::new(WorkerMetrics::new()));
        let mut workers = Vec::new();
        
        for i in 0..worker_count {
            workers.push(Worker {
                id: i,
                status: WorkerStatus::Idle,
                current_task: None,
                tasks_completed: 0,
            });
        }
        
        // Spawnar loop principal de distribuição de tarefas
        let workers_handle = Arc::new(Mutex::new(workers.clone()));
        let metrics_handle = metrics.clone();
        
        tokio::spawn(async move {
            while let Some(task) = task_receiver.recv().await {
                let worker_id = Self::find_available_worker(&workers_handle).await;
                Self::execute_task(worker_id, task, &workers_handle, &metrics_handle).await;
            }
        });
        
        Self {
            workers,
            task_sender,
            metrics,
        }
    }
    
    pub async fn submit_task(&self, task: Task) -> Result<TaskResult, Box<dyn std::error::Error>> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let task_with_response = Task {
            id: task.id,
            payload: task.payload,
            response_sender,
        };
        
        self.task_sender.send(task_with_response)?;
        let result = response_receiver.await?;
        
        Ok(result)
    }
    
    async fn find_available_worker(workers: &Arc<Mutex<Vec<Worker>>>) -> usize {
        loop {
            {
                let workers_guard = workers.lock().await;
                for (i, worker) in workers_guard.iter().enumerate() {
                    if matches!(worker.status, WorkerStatus::Idle) {
                        return i;
                    }
                }
            }
            
            // Se não há workers disponíveis, aguardar um pouco
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
    
    async fn execute_task(
        worker_id: usize,
        task: Task,
        workers: &Arc<Mutex<Vec<Worker>>>,
        metrics: &Arc<Mutex<WorkerMetrics>>,
    ) {
        // Marcar worker como ocupado
        {
            let mut workers_guard = workers.lock().await;
            workers_guard[worker_id].status = WorkerStatus::Busy;
            workers_guard[worker_id].current_task = Some(task.id.clone());
        }
        
        let start_time = std::time::Instant::now();
        
        // Executar tarefa
        let result = match task.payload {
            TaskPayload::Transform { records, transformer_type, config } => {
                Self::execute_transform_task(records, transformer_type, config).await
            },
            TaskPayload::Load { records, destination, config } => {
                Self::execute_load_task(records, destination, config).await
            },
            TaskPayload::Custom { operation, data } => {
                Self::execute_custom_task(operation, data).await
            },
        };
        
        let execution_time = start_time.elapsed();
        
        // Enviar resultado
        let _ = task.response_sender.send(result);
        
        // Atualizar status do worker e métricas
        {
            let mut workers_guard = workers.lock().await;
            workers_guard[worker_id].status = WorkerStatus::Idle;
            workers_guard[worker_id].current_task = None;
            workers_guard[worker_id].tasks_completed += 1;
        }
        
        {
            let mut metrics_guard = metrics.lock().await;
            metrics_guard.total_tasks_completed += 1;
            metrics_guard.total_execution_time += execution_time;
        }
    }
}
```

## 2. Sistema de Cache Inteligente

```rust
// src/cache/mod.rs
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CacheManager {
    storage: Arc<RwLock<HashMap<String, CacheEntry>>>,
    config: CacheConfig,
    metrics: Arc<RwLock<CacheMetrics>>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,
    pub ttl_seconds: u64,
    pub enable_compression: bool,
    pub eviction_strategy: EvictionStrategy,
}

#[derive(Debug, Clone)]
pub enum EvictionStrategy {
    LRU,
    LFU,
    TTL,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: usize,
    pub ttl: Option<chrono::Duration>,
    pub compressed: bool,
}

#[derive(Debug, Default)]
pub struct CacheMetrics {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub size_bytes: usize,
    pub entry_count: usize,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }
    
    pub async fn get<T>(&self, key: &str) -> Option<T> 
    where 
        T: for<'de> Deserialize<'de>,
    {
        let mut storage = self.storage.write().await;
        let mut metrics = self.metrics.write().await;
        
        if let Some(entry) = storage.get_mut(key) {
            // Verificar TTL
            if let Some(ttl) = entry.ttl {
                if chrono::Utc::now() - entry.created_at > ttl {
                    storage.remove(key);
                    metrics.misses += 1;
                    metrics.evictions += 1;
                    return None;
                }
            }
            
            // Atualizar estatísticas de acesso
            entry.last_accessed = chrono::Utc::now();
            entry.access_count += 1;
            metrics.hits += 1;
            
            // Descomprimir se necessário
            let data = if entry.compressed {
                self.decompress(&entry.value).ok()?
            } else {
                entry.value.clone()
            };
            
            // Deserializar
            serde_json::from_slice(&data).ok()
        } else {
            metrics.misses += 1;
            None
        }
    }
    
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<chrono::Duration>) 
    where 
        T: Serialize,
    {
        let serialized = match serde_json::to_vec(value) {
            Ok(data) => data,
            Err(_) => return,
        };
        
        let (final_data, compressed) = if self.config.enable_compression && serialized.len() > 1024 {
            match self.compress(&serialized) {
                Ok(compressed_data) => (compressed_data, true),
                Err(_) => (serialized, false),
            }
        } else {
            (serialized, false)
        };
        
        let entry = CacheEntry {
            key: key.to_string(),
            value: final_data,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            ttl,
            compressed,
        };
        
        let mut storage = self.storage.write().await;
        let mut metrics = self.metrics.write().await;
        
        // Verificar se precisamos de eviction
        if storage.len() >= self.config.max_size {
            self.evict_entry(&mut storage, &mut metrics).await;
        }
        
        let entry_size = entry.value.len();
        storage.insert(key.to_string(), entry);
        
        metrics.entry_count = storage.len();
        metrics.size_bytes += entry_size;
    }
    
    async fn evict_entry(
        &self,
        storage: &mut HashMap<String, CacheEntry>,
        metrics: &mut CacheMetrics,
    ) {
        if storage.is_empty() {
            return;
        }
        
        let key_to_remove = match self.config.eviction_strategy {
            EvictionStrategy::LRU => {
                storage.iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(key, _)| key.clone())
            },
            EvictionStrategy::LFU => {
                storage.iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(key, _)| key.clone())
            },
            EvictionStrategy::TTL => {
                storage.iter()
                    .min_by_key(|(_, entry)| entry.created_at)
                    .map(|(key, _)| key.clone())
            },
            EvictionStrategy::Random => {
                use rand::seq::IteratorRandom;
                storage.keys().choose(&mut rand::thread_rng()).cloned()
            },
        };
        
        if let Some(key) = key_to_remove {
            if let Some(entry) = storage.remove(&key) {
                metrics.size_bytes -= entry.value.len();
                metrics.evictions += 1;
            }
        }
    }
    
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
    
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        let mut metrics = self.metrics.write().await;
        
        storage.clear();
        *metrics = CacheMetrics::default();
    }
    
    pub async fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }
}
```

## 3. Sistema de Recovery e Checkpoints

```rust
// src/recovery/mod.rs
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub pipeline_id: String,
    pub stage: PipelineStage,
    pub snapshot: DatasetSnapshot,
    pub metadata: CheckpointMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub total_records: usize,
    pub processed_records: usize,
    pub stage_progress: f64,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
    pub memory_usage: usize,
    pub errors_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStage {
    Extraction,
    Transformation(usize), // índice da transformação
    Loading,
    Completed,
}

pub struct RecoveryManager {
    checkpoint_dir: PathBuf,
    max_checkpoints: usize,
    checkpoint_interval: std::time::Duration,
}

impl RecoveryManager {
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&checkpoint_dir).ok();
        
        Self {
            checkpoint_dir,
            max_checkpoints: 10,
            checkpoint_interval: std::time::Duration::from_secs(300), // 5 minutos
        }
    }
    
    pub async fn create_checkpoint(
        &self,
        pipeline_id: &str,
        stage: PipelineStage,
        snapshot: DatasetSnapshot,
        metadata: CheckpointMetadata,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        
        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            pipeline_id: pipeline_id.to_string(),
            stage,
            snapshot,
            metadata,
            created_at: chrono::Utc::now(),
        };
        
        let filename = format!("checkpoint_{}_{}.json", pipeline_id, checkpoint_id);
        let filepath = self.checkpoint_dir.join(filename);
        
        let serialized = serde_json::to_string_pretty(&checkpoint)?;
        tokio::fs::write(filepath, serialized).await?;
        
        // Limpar checkpoints antigos
        self.cleanup_old_checkpoints(pipeline_id).await?;
        
        Ok(checkpoint_id)
    }
    
    pub async fn get_latest_checkpoint(&self, pipeline_id: &str) -> Result<Option<Checkpoint>, Box<dyn std::error::Error>> {
        let mut checkpoints = self.list_checkpoints(pipeline_id).await?;
        
        if checkpoints.is_empty() {
            return Ok(None);
        }
        
        // Ordenar por data de criação (mais recente primeiro)
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        Ok(Some(checkpoints.into_iter().next().unwrap()))
    }
    
    pub async fn list_checkpoints(&self, pipeline_id: &str) -> Result<Vec<Checkpoint>, Box<dyn std::error::Error>> {
        let mut checkpoints = Vec::new();
        let mut dir = tokio::fs::read_dir(&self.checkpoint_dir).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with(&format!("checkpoint_{}_", pipeline_id)) {
                let content = tokio::fs::read_to_string(entry.path()).await?;
                if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&content) {
                    checkpoints.push(checkpoint);
                }
            }
        }
        
        Ok(checkpoints)
    }
    
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>, Box<dyn std::error::Error>> {
        let mut dir = tokio::fs::read_dir(&self.checkpoint_dir).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.contains(&checkpoint_id) {
                let content = tokio::fs::read_to_string(entry.path()).await?;
                let checkpoint = serde_json::from_str::<Checkpoint>(&content)?;
                return Ok(Some(checkpoint));
            }
        }
        
        Ok(None)
    }
    
    async fn cleanup_old_checkpoints(&self, pipeline_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut checkpoints = self.list_checkpoints(pipeline_id).await?;
        
        if checkpoints.len() <= self.max_checkpoints {
            return Ok(());
        }
        
        // Ordenar por data (mais antigo primeiro)
        checkpoints.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        // Remover checkpoints excedentes
        let to_remove = checkpoints.len() - self.max_checkpoints;
        for checkpoint in checkpoints.iter().take(to_remove) {
            let filename = format!("checkpoint_{}_{}.json", pipeline_id, checkpoint.id);
            let filepath = self.checkpoint_dir.join(filename);
            
            if filepath.exists() {
                tokio::fs::remove_file(filepath).await?;
            }
        }
        
        Ok(())
    }
}

// Pipeline com Recovery
pub struct RecoverablePipeline {
    pipeline: Pipeline,
    recovery_manager: RecoveryManager,
    enable_checkpoints: bool,
    checkpoint_interval: std::time::Duration,
}

impl RecoverablePipeline {
    pub fn new(pipeline: Pipeline, recovery_manager: RecoveryManager) -> Self {
        Self {
            pipeline,
            recovery_manager,
            enable_checkpoints: true,
            checkpoint_interval: std::time::Duration::from_secs(300),
        }
    }
    
    pub async fn execute_with_recovery(&self, pipeline_id: &str) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        // Verificar se existe checkpoint para retomar
        if let Some(checkpoint) = self.recovery_manager.get_latest_checkpoint(pipeline_id).await? {
            println!("Checkpoint encontrado, retomando do estágio: {:?}", checkpoint.stage);
            return self.resume_from_checkpoint(checkpoint).await;
        }
        
        // Executar pipeline normalmente com checkpoints
        self.execute_with_checkpoints(pipeline_id).await
    }
    
    async fn execute_with_checkpoints(&self, pipeline_id: &str) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let mut last_checkpoint = start_time;
        
        // Executar extração
        let extraction_snapshot = self.pipeline.execute_extraction().await?;
        
        // Checkpoint após extração
        if self.should_create_checkpoint(last_checkpoint) {
            let metadata = CheckpointMetadata {
                total_records: extraction_snapshot.total_count,
                processed_records: extraction_snapshot.total_count,
                stage_progress: 25.0, // 25% completo
                estimated_completion: None,
                memory_usage: self.get_memory_usage(),
                errors_count: 0,
            };
            
            self.recovery_manager.create_checkpoint(
                pipeline_id,
                PipelineStage::Extraction,
                extraction_snapshot.clone(),
                metadata,
            ).await?;
            
            last_checkpoint = std::time::Instant::now();
        }
        
        // Executar transformações
        let mut current_snapshot = extraction_snapshot;
        for (i, transformer) in self.pipeline.transformers.iter().enumerate() {
            current_snapshot = transformer.transform(current_snapshot).await?;
            
            // Checkpoint após cada transformação importante
            if self.should_create_checkpoint(last_checkpoint) {
                let progress = 25.0 + (50.0 * (i + 1) as f64 / self.pipeline.transformers.len() as f64);
                let metadata = CheckpointMetadata {
                    total_records: current_snapshot.total_count,
                    processed_records: current_snapshot.total_count,
                    stage_progress: progress,
                    estimated_completion: self.estimate_completion(start_time, progress),
                    memory_usage: self.get_memory_usage(),
                    errors_count: 0,
                };
                
                self.recovery_manager.create_checkpoint(
                    pipeline_id,
                    PipelineStage::Transformation(i),
                    current_snapshot.clone(),
                    metadata,
                ).await?;
                
                last_checkpoint = std::time::Instant::now();
            }
        }
        
        // Executar loading
        let load_results = self.pipeline.execute_loading(&current_snapshot).await?;
        
        // Checkpoint final
        let metadata = CheckpointMetadata {
            total_records: current_snapshot.total_count,
            processed_records: current_snapshot.total_count,
            stage_progress: 100.0,
            estimated_completion: Some(chrono::Utc::now()),
            memory_usage: self.get_memory_usage(),
            errors_count: 0,
        };
        
        self.recovery_manager.create_checkpoint(
            pipeline_id,
            PipelineStage::Completed,
            current_snapshot.clone(),
            metadata,
        ).await?;
        
        Ok(PipelineResult {
            extraction_result: Some(current_snapshot.clone()),
            transformation_result: Some(current_snapshot),
            load_results,
            execution_time: start_time.elapsed(),
            success: true,
        })
    }
    
    fn should_create_checkpoint(&self, last_checkpoint: std::time::Instant) -> bool {
        self.enable_checkpoints && last_checkpoint.elapsed() >= self.checkpoint_interval
    }
    
    fn get_memory_usage(&self) -> usize {
        // Implementar coleta de métricas de memória
        0
    }
    
    fn estimate_completion(&self, start_time: std::time::Instant, progress: f64) -> Option<chrono::DateTime<chrono::Utc>> {
        if progress > 0.0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let estimated_total = elapsed / (progress / 100.0);
            let remaining = estimated_total - elapsed;
            
            Some(chrono::Utc::now() + chrono::Duration::seconds(remaining as i64))
        } else {
            None
        }
    }
}
```

## 4. Observabilidade e Monitoramento

```rust
// src/observability/mod.rs
use prometheus::{Counter, Histogram, Gauge, Registry};
use tracing::{info, warn, error, debug};
use std::sync::Arc;

pub struct ObservabilityManager {
    registry: Registry,
    metrics: ETLMetrics,
    tracer: Arc<dyn Tracer>,
}

pub struct ETLMetrics {
    pub records_processed: Counter,
    pub pipeline_duration: Histogram,
    pub error_count: Counter,
    pub memory_usage: Gauge,
    pub active_pipelines: Gauge,
    pub transformation_duration: Histogram,
    pub extraction_duration: Histogram,
    pub loading_duration: Histogram,
}

impl ObservabilityManager {
    pub fn new() -> Self {
        let registry = Registry::new();
        
        let records_processed = Counter::new("etl_records_processed_total", "Total number of records processed").unwrap();
        let pipeline_duration = Histogram::new("etl_pipeline_duration_seconds", "Pipeline execution duration").unwrap();
        let error_count = Counter::new("etl_errors_total", "Total number of errors").unwrap();
        let memory_usage = Gauge::new("etl_memory_usage_bytes", "Current memory usage").unwrap();
        let active_pipelines = Gauge::new("etl_active_pipelines", "Number of active pipelines").unwrap();
        let transformation_duration = Histogram::new("etl_transformation_duration_seconds", "Transformation duration").unwrap();
        let extraction_duration = Histogram::new("etl_extraction_duration_seconds", "Extraction duration").unwrap();
        let loading_duration = Histogram::new("etl_loading_duration_seconds", "Loading duration").unwrap();
        
        registry.register(Box::new(records_processed.clone())).unwrap();
        registry.register(Box::new(pipeline_duration.clone())).unwrap();
        registry.register(Box::new(error_count.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();
        registry.register(Box::new(active_pipelines.clone())).unwrap();
        registry.register(Box::new(transformation_duration.clone())).unwrap();
        registry.register(Box::new(extraction_duration.clone())).unwrap();
        registry.register(Box::new(loading_duration.clone())).unwrap();
        
        let metrics = ETLMetrics {
            records_processed,
            pipeline_duration,
            error_count,
            memory_usage,
            active_pipelines,
            transformation_duration,
            extraction_duration,
            loading_duration,
        };
        
        Self {
            registry,
            metrics,
            tracer: Arc::new(DefaultTracer::new()),
        }
    }
    
    pub fn start_pipeline_span(&self, pipeline_id: &str) -> Span {
        self.metrics.active_pipelines.inc();
        
        info!(
            pipeline_id = %pipeline_id,
            "Pipeline execution started"
        );
        
        self.tracer.start_span("pipeline_execution", Some(pipeline_id))
    }
    
    pub fn record_extraction(&self, duration: std::time::Duration, record_count: usize) {
        self.metrics.extraction_duration.observe(duration.as_secs_f64());
        self.metrics.records_processed.inc_by(record_count as u64);
        
        info!(
            duration_ms = %duration.as_millis(),
            record_count = %record_count,
            "Extraction completed"
        );
    }
    
    pub fn record_transformation(&self, duration: std::time::Duration, transformer_name: &str) {
        self.metrics.transformation_duration.observe(duration.as_secs_f64());
        
        debug!(
            transformer = %transformer_name,
            duration_ms = %duration.as_millis(),
            "Transformation completed"
        );
    }
    
    pub fn record_loading(&self, duration: std::time::Duration, record_count: usize, destination: &str) {
        self.metrics.loading_duration.observe(duration.as_secs_f64());
        
        info!(
            destination = %destination,
            duration_ms = %duration.as_millis(),
            record_count = %record_count,
            "Loading completed"
        );
    }
    
    pub fn record_error(&self, error: &str, stage: &str) {
        self.metrics.error_count.inc();
        
        error!(
            error_message = %error,
            stage = %stage,
            "Pipeline error occurred"
        );
    }
    
    pub fn finish_pipeline(&self, duration: std::time::Duration, success: bool) {
        self.metrics.active_pipelines.dec();
        self.metrics.pipeline_duration.observe(duration.as_secs_f64());
        
        if success {
            info!(
                duration_ms = %duration.as_millis(),
                "Pipeline completed successfully"
            );
        } else {
            warn!(
                duration_ms = %duration.as_millis(),
                "Pipeline completed with errors"
            );
        }
    }
    
    pub fn get_metrics_text(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }
}
```

## 5. Integração com Sistemas Externos

```rust
// src/integrations/mod.rs
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait ExternalSystemConnector: Send + Sync {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>>;
    async fn send_data(&self, data: &DatasetSnapshot) -> Result<Value, Box<dyn std::error::Error>>;
    async fn receive_data(&self, query: &str) -> Result<DatasetSnapshot, Box<dyn std::error::Error>>;
}

// Exemplo: Conector Kafka
pub struct KafkaConnector {
    bootstrap_servers: String,
    topic: String,
    config: HashMap<String, String>,
    producer: Option<rdkafka::producer::FutureProducer>,
    consumer: Option<rdkafka::consumer::StreamConsumer>,
}

impl KafkaConnector {
    pub fn new(bootstrap_servers: String, topic: String) -> Self {
        Self {
            bootstrap_servers,
            topic,
            config: HashMap::new(),
            producer: None,
            consumer: None,
        }
    }
    
    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }
}

#[async_trait]
impl ExternalSystemConnector for KafkaConnector {
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementar conexão com Kafka
        Ok(())
    }
    
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementar desconexão
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Implementar verificação de saúde
        Ok(true)
    }
    
    async fn send_data(&self, data: &DatasetSnapshot) -> Result<Value, Box<dyn std::error::Error>> {
        // Implementar envio para Kafka
        Ok(serde_json::json!({"status": "sent", "records": data.total_count}))
    }
    
    async fn receive_data(&self, query: &str) -> Result<DatasetSnapshot, Box<dyn std::error::Error>> {
        // Implementar consumo do Kafka
        Ok(DatasetSnapshot {
            records: Vec::new(),
            total_count: 0,
            checksum: String::new(),
            timestamp: chrono::Utc::now(),
            schema_hash: String::new(),
        })
    }
}

// Registro de Conectores
pub struct ConnectorRegistry {
    connectors: HashMap<String, Box<dyn ExternalSystemConnector>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: HashMap::new(),
        }
    }
    
    pub fn register<T: ExternalSystemConnector + 'static>(&mut self, name: &str, connector: T) {
        self.connectors.insert(name.to_string(), Box::new(connector));
    }
    
    pub async fn get_connector(&self, name: &str) -> Option<&Box<dyn ExternalSystemConnector>> {
        self.connectors.get(name)
    }
    
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        
        for (name, connector) in &self.connectors {
            let health = connector.health_check().await.unwrap_or(false);
            results.insert(name.clone(), health);
        }
        
        results
    }
}
```

Este conjunto de recursos avançados transforma a biblioteca ETLRS em uma solução enterprise-grade, capaz de lidar com os requisitos mais exigentes de processamento de dados em ambientes de produção.
