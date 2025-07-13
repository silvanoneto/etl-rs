# Arquitetura Principal

A arquitetura da biblioteca ETLRS foi cuidadosamente projetada para garantir modularidade, extensibilidade e alta performance em pipelines ETL. Esta seção apresenta uma visão geral dos principais componentes, padrões de projeto e estruturas de dados que compõem o núcleo do sistema, detalhando como cada parte se integra para oferecer uma solução robusta e escalável para processamento de dados.

## 1. Tipos de Dados

Os tipos de dados definidos nesta seção foram escolhidos para garantir flexibilidade, eficiência e segurança no processamento de dados em pipelines ETL. Eles representam abstrações fundamentais como registros, snapshots de datasets e valores genéricos, permitindo que o pipeline manipule diferentes formatos e fontes de dados de maneira consistente e extensível.

```rust
// src/lib.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub fields: HashMap<String, Value>,
    pub metadata: RecordMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMetadata {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
    pub source: String,
    pub schema_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSnapshot {
    pub records: Vec<Record>,
    pub total_count: usize,
    pub checksum: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub schema_hash: String,
}
```

## 2. Definições de Traits

Crie traits para extensibilidade:

```rust
// src/lib.rs
use async_trait::async_trait;

#[async_trait]
pub trait Extractor {
    async fn extract(&self) -> Result<DatasetSnapshot, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait Transformer {
    async fn transform(&self, snapshot: DatasetSnapshot) -> Result<DatasetSnapshot, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait Loader {
    async fn load(&self, snapshot: DatasetSnapshot) -> Result<LoadResult, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait IntegrityChecker {
    async fn verify_integrity(&self, snapshot: &DatasetSnapshot) -> Result<IntegrityReport, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    pub records_loaded: usize,
    pub checksum: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub is_valid: bool,
    pub checksum_match: bool,
    pub record_count_match: bool,
    pub schema_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
```

## 3. Padrão Pipeline Builder

Implemente uma API fluente para construção de pipeline:

```rust
// src/pipeline/builder.rs
pub struct PipelineBuilder {
    extractors: Vec<Box<dyn Extractor>>,
    transformers: Vec<Box<dyn Transformer>>,
    loaders: Vec<Box<dyn Loader>>,
    integrity_checkers: Vec<Box<dyn IntegrityChecker>>,
    enable_integrity: bool,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            extractors: Vec::new(),
            transformers: Vec::new(),
            loaders: Vec::new(),
            integrity_checkers: Vec::new(),
            enable_integrity: true,
        }
    }

    pub fn extract<E: Extractor + 'static>(mut self, extractor: E) -> Self {
        self.extractors.push(Box::new(extractor));
        self
    }

    pub fn transform<T: Transformer + 'static>(mut self, transformer: T) -> Self {
        self.transformers.push(Box::new(transformer));
        self
    }

    pub fn load<L: Loader + 'static>(mut self, loader: L) -> Self {
        self.loaders.push(Box::new(loader));
        self
    }

    pub fn with_integrity_checker<I: IntegrityChecker + 'static>(mut self, checker: I) -> Self {
        self.integrity_checkers.push(Box::new(checker));
        self
    }

    pub fn disable_integrity_checks(mut self) -> Self {
        self.enable_integrity = false;
        self
    }

    pub fn build(self) -> Pipeline {
        Pipeline {
            extractors: self.extractors,
            transformers: self.transformers,
            loaders: self.loaders,
            integrity_checkers: self.integrity_checkers,
            enable_integrity: self.enable_integrity,
        }
    }
}
```

## 4. Pipeline Core

```rust
// src/pipeline/mod.rs
pub struct Pipeline {
    extractors: Vec<Box<dyn Extractor>>,
    transformers: Vec<Box<dyn Transformer>>,
    loaders: Vec<Box<dyn Loader>>,
    integrity_checkers: Vec<Box<dyn IntegrityChecker>>,
    enable_integrity: bool,
}

impl Pipeline {
    pub async fn execute(&self) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        let mut result = PipelineResult::new();
        
        // Executar extração
        let mut current_snapshot = self.execute_extraction().await?;
        result.extraction_result = Some(current_snapshot.clone());
        
        // Verificação de integridade pós-extração
        if self.enable_integrity {
            self.verify_integrity(&current_snapshot).await?;
        }
        
        // Executar transformações
        for transformer in &self.transformers {
            current_snapshot = transformer.transform(current_snapshot).await?;
            
            // Verificação de integridade pós-transformação
            if self.enable_integrity {
                self.verify_integrity(&current_snapshot).await?;
            }
        }
        result.transformation_result = Some(current_snapshot.clone());
        
        // Executar carregamento
        let load_results = self.execute_loading(&current_snapshot).await?;
        result.load_results = load_results;
        
        Ok(result)
    }
    
    async fn execute_extraction(&self) -> Result<DatasetSnapshot, Box<dyn std::error::Error>> {
        // Combinar resultados de múltiplos extractors
        let mut all_records = Vec::new();
        
        for extractor in &self.extractors {
            let snapshot = extractor.extract().await?;
            all_records.extend(snapshot.records);
        }
        
        Ok(DatasetSnapshot {
            records: all_records,
            total_count: all_records.len(),
            checksum: self.calculate_checksum(&all_records),
            timestamp: chrono::Utc::now(),
            schema_hash: self.calculate_schema_hash(&all_records),
        })
    }
    
    async fn execute_loading(&self, snapshot: &DatasetSnapshot) -> Result<Vec<LoadResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        for loader in &self.loaders {
            let result = loader.load(snapshot.clone()).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    async fn verify_integrity(&self, snapshot: &DatasetSnapshot) -> Result<(), Box<dyn std::error::Error>> {
        for checker in &self.integrity_checkers {
            let report = checker.verify_integrity(snapshot).await?;
            if !report.is_valid {
                return Err(format!("Integrity check failed: {:?}", report.errors).into());
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PipelineResult {
    pub extraction_result: Option<DatasetSnapshot>,
    pub transformation_result: Option<DatasetSnapshot>,
    pub load_results: Vec<LoadResult>,
    pub execution_time: std::time::Duration,
    pub success: bool,
}

impl PipelineResult {
    pub fn new() -> Self {
        Self {
            extraction_result: None,
            transformation_result: None,
            load_results: Vec::new(),
            execution_time: std::time::Duration::new(0, 0),
            success: false,
        }
    }
}
```

## 5. Sistema de Erros

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ETLError {
    #[error("Extraction failed: {0}")]
    ExtractionError(String),
    
    #[error("Transformation failed: {0}")]
    TransformationError(String),
    
    #[error("Load failed: {0}")]
    LoadError(String),
    
    #[error("Integrity check failed: {0}")]
    IntegrityError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
}

pub type ETLResult<T> = Result<T, ETLError>;
```

## 6. Sistema de Configuração

```rust
// src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETLConfig {
    pub name: String,
    pub description: Option<String>,
    pub pipeline: PipelineConfig,
    pub extractors: Vec<ExtractorConfig>,
    pub transformers: Vec<TransformerConfig>,
    pub loaders: Vec<LoaderConfig>,
    pub integrity: IntegrityConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub max_parallel_tasks: usize,
    pub retry_attempts: u32,
    pub timeout_seconds: u64,
    pub checkpoint_interval: u64,
    pub enable_recovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    pub name: String,
    pub extractor_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub retry_config: Option<RetryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformerConfig {
    pub name: String,
    pub transformer_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub order: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderConfig {
    pub name: String,
    pub loader_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub batch_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl ETLConfig {
    pub fn from_file(path: &str) -> Result<Self, ETLError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ETLError::ConfigError(format!("Failed to read config file: {}", e)))?;
        
        let config: ETLConfig = serde_json::from_str(&content)
            .map_err(|e| ETLError::ConfigError(format!("Failed to parse config: {}", e)))?;
        
        config.validate()?;
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<(), ETLError> {
        if self.name.is_empty() {
            return Err(ETLError::ConfigError("Pipeline name cannot be empty".to_string()));
        }
        
        if self.extractors.is_empty() {
            return Err(ETLError::ConfigError("At least one extractor is required".to_string()));
        }
        
        if self.loaders.is_empty() {
            return Err(ETLError::ConfigError("At least one loader is required".to_string()));
        }
        
        Ok(())
    }
}
```

## 7. Exemplo de Uso

```rust
// examples/basic_pipeline.rs
use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar pipeline
    let pipeline = PipelineBuilder::new()
        .extract(CsvExtractor::new("data/input.csv"))
        .transform(FilterTransformer::new(|record| {
            record.fields.get("status").map_or(false, |v| v == &Value::String("active".to_string()))
        }))
        .transform(MapTransformer::new(|mut record| {
            record.fields.insert("processed_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
            record
        }))
        .load(JsonLoader::new("data/output.json"))
        .with_integrity_checker(ChecksumChecker::new())
        .build();
    
    // Executar pipeline
    let result = pipeline.execute().await?;
    
    println!("Pipeline executado com sucesso!");
    println!("Registros processados: {}", result.transformation_result.unwrap().total_count);
    
    Ok(())
}
```

Esta arquitetura fornece uma base sólida e extensível para construir pipelines ETL complexos, mantendo a flexibilidade para diferentes casos de uso enquanto garante alta performance e confiabilidade.
