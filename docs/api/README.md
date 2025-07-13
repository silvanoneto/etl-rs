# 📚 Documentação da API

## Visão Geral

A API da biblioteca ETLRS fornece interfaces programáticas para todas as funcionalidades principais do sistema. Esta documentação cobre as APIs Rust nativas, interfaces REST quando aplicável, e exemplos de uso para diferentes cenários.

## 📋 Sumário

- [📚 Documentação da API](#-documentação-da-api)
  - [Visão Geral](#visão-geral)
  - [📋 Sumário](#-sumário)
  - [Estrutura da API](#estrutura-da-api)
    - [Hierarquia de Módulos](#hierarquia-de-módulos)
    - [Traits Principais](#traits-principais)
      - [Pipeline Trait](#pipeline-trait)
      - [Extract Trait](#extract-trait)
      - [Transform Trait](#transform-trait)
      - [Load Trait](#load-trait)
  - [APIs por Módulo](#apis-por-módulo)
    - [Pipeline API](#pipeline-api)
      - [PipelineBuilder](#pipelinebuilder)
      - [PipelineExecutor](#pipelineexecutor)
    - [Extract API](#extract-api)
      - [CSV Extractor](#csv-extractor)
      - [Database Extractor](#database-extractor)
    - [Transform API](#transform-api)
      - [FilterTransform](#filtertransform)
      - [MapTransform](#maptransform)
    - [Load API](#load-api)
      - [JSON Loader](#json-loader)
  - [Exemplos de Uso](#exemplos-de-uso)
    - [Pipeline Simples](#pipeline-simples)
    - [Pipeline com Database](#pipeline-com-database)
    - [Pipeline Streaming](#pipeline-streaming)

## Estrutura da API

### Hierarquia de Módulos

```
etlrs::
├── pipeline/          # Orquestração de pipelines
├── extract/           # Extração de dados  
├── transform/         # Transformação de dados
├── load/              # Carregamento de dados
├── config/            # Configuração do sistema
├── error/             # Tratamento de erros
├── observability/     # Métricas e observabilidade
└── types/             # Tipos de dados comuns
```

### Traits Principais

#### Pipeline Trait
```rust
use async_trait::async_trait;

#[async_trait]
pub trait Pipeline {
    type Input;
    type Output;
    type Error;

    /// Executa o pipeline completo
    async fn execute(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
    
    /// Valida o pipeline antes da execução
    async fn validate(&self) -> Result<(), Self::Error>;
    
    /// Obtém métricas do pipeline
    fn get_metrics(&self) -> PipelineMetrics;
    
    /// Obtém configuração atual
    fn get_config(&self) -> &PipelineConfig;
}
```

#### Extract Trait
```rust
#[async_trait]
pub trait Extract<T> {
    type Error;
    
    /// Extrai dados da fonte
    async fn extract(&self) -> Result<Vec<T>, Self::Error>;
    
    /// Extrai dados em stream
    fn extract_stream(&self) -> impl Stream<Item = Result<T, Self::Error>>;
    
    /// Valida a conexão com a fonte
    async fn validate_connection(&self) -> Result<(), Self::Error>;
    
    /// Obtém metadados da fonte
    async fn get_metadata(&self) -> Result<SourceMetadata, Self::Error>;
}
```

#### Transform Trait
```rust
#[async_trait]
pub trait Transform<I, O> {
    type Error;
    
    /// Transforma um único item
    async fn transform_item(&self, item: I) -> Result<O, Self::Error>;
    
    /// Transforma um batch de itens
    async fn transform_batch(&self, items: Vec<I>) -> Result<Vec<O>, Self::Error>;
    
    /// Transforma dados em stream
    fn transform_stream(
        &self, 
        input: impl Stream<Item = Result<I, Self::Error>>
    ) -> impl Stream<Item = Result<O, Self::Error>>;
    
    /// Valida transformação
    async fn validate(&self, sample: &I) -> Result<(), Self::Error>;
}
```

#### Load Trait
```rust
#[async_trait]
pub trait Load<T> {
    type Error;
    
    /// Carrega dados no destino
    async fn load(&self, data: Vec<T>) -> Result<LoadResult, Self::Error>;
    
    /// Carrega dados em stream
    async fn load_stream(
        &self, 
        stream: impl Stream<Item = Result<T, Self::Error>>
    ) -> Result<LoadResult, Self::Error>;
    
    /// Valida conexão com destino
    async fn validate_connection(&self) -> Result<(), Self::Error>;
    
    /// Obtém estatísticas de carregamento
    fn get_load_stats(&self) -> LoadStats;
}
```

## APIs por Módulo

### Pipeline API

#### PipelineBuilder

```rust
use etlrs::pipeline::{Pipeline, PipelineBuilder, PipelineConfig};

/// Constrói pipelines de forma fluente
pub struct PipelineBuilder<E, T, L> {
    extractor: Option<E>,
    transformer: Option<T>, 
    loader: Option<L>,
    config: PipelineConfig,
}

impl PipelineBuilder<(), (), ()> {
    /// Cria novo builder
    pub fn new() -> Self {
        Self {
            extractor: None,
            transformer: None,
            loader: None,
            config: PipelineConfig::default(),
        }
    }
}

impl<E, T, L> PipelineBuilder<E, T, L> {
    /// Define extrator
    pub fn with_extractor<E2>(self, extractor: E2) -> PipelineBuilder<E2, T, L> {
        PipelineBuilder {
            extractor: Some(extractor),
            transformer: self.transformer,
            loader: self.loader,
            config: self.config,
        }
    }
    
    /// Define transformador
    pub fn with_transformer<T2>(self, transformer: T2) -> PipelineBuilder<E, T2, L> {
        PipelineBuilder {
            extractor: self.extractor,
            transformer: Some(transformer),
            loader: self.loader,
            config: self.config,
        }
    }
    
    /// Define carregador
    pub fn with_loader<L2>(self, loader: L2) -> PipelineBuilder<E, T, L2> {
        PipelineBuilder {
            extractor: self.extractor,
            transformer: self.transformer,
            loader: Some(loader),
            config: self.config,
        }
    }
    
    /// Define configuração
    pub fn with_config(mut self, config: PipelineConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Constrói o pipeline
    pub fn build(self) -> Result<EtlPipeline<E, T, L>, PipelineError>
    where
        E: Extract<T::Input>,
        T: Transform<E::Output, L::Input>,
        L: Load<T::Output>,
    {
        let extractor = self.extractor.ok_or(PipelineError::MissingExtractor)?;
        let transformer = self.transformer.ok_or(PipelineError::MissingTransformer)?;
        let loader = self.loader.ok_or(PipelineError::MissingLoader)?;
        
        Ok(EtlPipeline::new(extractor, transformer, loader, self.config))
    }
}
```

#### PipelineExecutor

```rust
/// Executor principal de pipelines
pub struct PipelineExecutor<E, T, L> {
    extractor: E,
    transformer: T,
    loader: L,
    config: PipelineConfig,
    metrics: Arc<PipelineMetrics>,
}

impl<E, T, L> PipelineExecutor<E, T, L>
where
    E: Extract + Send + Sync,
    T: Transform<E::Output> + Send + Sync,
    L: Load<T::Output> + Send + Sync,
{
    /// Executa pipeline sequencial
    pub async fn execute_sequential(&self) -> Result<ExecutionResult, PipelineError> {
        let start_time = Instant::now();
        
        // Extract
        let extracted_data = self.extractor.extract().await
            .map_err(PipelineError::ExtractError)?;
        
        self.metrics.record_extraction(extracted_data.len(), start_time.elapsed());
        
        // Transform
        let transform_start = Instant::now();
        let transformed_data = self.transformer.transform_batch(extracted_data).await
            .map_err(PipelineError::TransformError)?;
            
        self.metrics.record_transformation(transformed_data.len(), transform_start.elapsed());
        
        // Load
        let load_start = Instant::now();
        let load_result = self.loader.load(transformed_data).await
            .map_err(PipelineError::LoadError)?;
            
        self.metrics.record_loading(load_result.records_loaded, load_start.elapsed());
        
        Ok(ExecutionResult {
            total_duration: start_time.elapsed(),
            records_processed: load_result.records_loaded,
            load_result,
        })
    }
    
    /// Executa pipeline em paralelo
    pub async fn execute_parallel(&self) -> Result<ExecutionResult, PipelineError> {
        let batch_size = self.config.batch_size;
        let max_parallel = self.config.max_parallel_tasks;
        
        // Extract em stream
        let data_stream = self.extractor.extract_stream();
        
        // Process em batches paralelos
        let results = data_stream
            .chunks(batch_size)
            .map(|batch| async {
                let transformed = self.transformer.transform_batch(batch).await?;
                self.loader.load(transformed).await
            })
            .buffer_unordered(max_parallel)
            .try_collect::<Vec<_>>()
            .await
            .map_err(PipelineError::ProcessingError)?;
        
        // Aggregate results
        let total_records: usize = results.iter().map(|r| r.records_loaded).sum();
        
        Ok(ExecutionResult {
            total_duration: start_time.elapsed(),
            records_processed: total_records,
            load_result: LoadResult::aggregate(results),
        })
    }
    
    /// Executa com retry automático
    pub async fn execute_with_retry(&self) -> Result<ExecutionResult, PipelineError> {
        let max_retries = self.config.retry_attempts;
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match self.execute_sequential().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(1000 * 2_u64.pow(attempt as u32));
                        tokio::time::sleep(delay).await;
                        warn!("Pipeline execution failed, retrying in {:?}", delay);
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}
```

### Extract API

#### CSV Extractor

```rust
use etlrs::extract::{CsvExtractor, CsvConfig};

/// Extrator para arquivos CSV
pub struct CsvExtractor {
    config: CsvConfig,
    file_path: PathBuf,
}

impl CsvExtractor {
    /// Cria novo extrator CSV
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            config: CsvConfig::default(),
            file_path: file_path.into(),
        }
    }
    
    /// Configura delimitador
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.config.delimiter = delimiter;
        self
    }
    
    /// Configura se possui cabeçalho
    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.config.has_headers = has_headers;
        self
    }
    
    /// Configura encoding
    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.config.encoding = encoding;
        self
    }
}

#[async_trait]
impl Extract<CsvRecord> for CsvExtractor {
    type Error = CsvError;
    
    async fn extract(&self) -> Result<Vec<CsvRecord>, Self::Error> {
        let file = File::open(&self.file_path).await
            .map_err(CsvError::IoError)?;
        
        let mut reader = csv_async::AsyncReaderBuilder::new()
            .delimiter(self.config.delimiter)
            .has_headers(self.config.has_headers)
            .create_reader(file);
        
        let mut records = Vec::new();
        let mut deserializer = reader.deserialize::<CsvRecord>();
        
        while let Some(result) = deserializer.next().await {
            let record = result.map_err(CsvError::ParseError)?;
            records.push(record);
        }
        
        Ok(records)
    }
    
    fn extract_stream(&self) -> impl Stream<Item = Result<CsvRecord, Self::Error>> {
        async_stream::stream! {
            let file = match File::open(&self.file_path).await {
                Ok(file) => file,
                Err(e) => {
                    yield Err(CsvError::IoError(e));
                    return;
                }
            };
            
            let mut reader = csv_async::AsyncReaderBuilder::new()
                .delimiter(self.config.delimiter)
                .has_headers(self.config.has_headers)
                .create_reader(file);
            
            let mut deserializer = reader.deserialize::<CsvRecord>();
            
            while let Some(result) = deserializer.next().await {
                match result {
                    Ok(record) => yield Ok(record),
                    Err(e) => yield Err(CsvError::ParseError(e)),
                }
            }
        }
    }
    
    async fn validate_connection(&self) -> Result<(), Self::Error> {
        if !self.file_path.exists() {
            return Err(CsvError::FileNotFound(self.file_path.clone()));
        }
        
        // Tenta ler primeiro registro
        let file = File::open(&self.file_path).await
            .map_err(CsvError::IoError)?;
        
        let mut reader = csv_async::AsyncReaderBuilder::new()
            .delimiter(self.config.delimiter)
            .has_headers(self.config.has_headers)
            .create_reader(file);
        
        let mut deserializer = reader.deserialize::<CsvRecord>();
        
        if let Some(result) = deserializer.next().await {
            result.map_err(CsvError::ParseError)?;
        }
        
        Ok(())
    }
    
    async fn get_metadata(&self) -> Result<SourceMetadata, Self::Error> {
        let file_metadata = tokio::fs::metadata(&self.file_path).await
            .map_err(CsvError::IoError)?;
        
        Ok(SourceMetadata {
            source_type: "csv".to_string(),
            size_bytes: file_metadata.len(),
            last_modified: file_metadata.modified().ok(),
            estimated_records: None, // Calculado durante primeira leitura
            schema: None,
        })
    }
}
```

#### Database Extractor

```rust
use etlrs::extract::{DatabaseExtractor, DatabaseConfig};
use sqlx::{PgPool, Row};

/// Extrator para bancos de dados
pub struct DatabaseExtractor {
    pool: PgPool,
    query: String,
    config: DatabaseConfig,
}

impl DatabaseExtractor {
    /// Cria novo extrator de database
    pub async fn new(
        database_url: &str, 
        query: impl Into<String>
    ) -> Result<Self, DatabaseError> {
        let pool = PgPool::connect(database_url).await
            .map_err(DatabaseError::ConnectionError)?;
        
        Ok(Self {
            pool,
            query: query.into(),
            config: DatabaseConfig::default(),
        })
    }
    
    /// Configura batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size;
        self
    }
    
    /// Configura timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
}

#[async_trait]
impl Extract<DatabaseRecord> for DatabaseExtractor {
    type Error = DatabaseError;
    
    async fn extract(&self) -> Result<Vec<DatabaseRecord>, Self::Error> {
        let rows = sqlx::query(&self.query)
            .fetch_all(&self.pool)
            .await
            .map_err(DatabaseError::QueryError)?;
        
        let records = rows.into_iter()
            .map(|row| DatabaseRecord::from_row(row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::ConversionError)?;
        
        Ok(records)
    }
    
    fn extract_stream(&self) -> impl Stream<Item = Result<DatabaseRecord, Self::Error>> {
        let query = self.query.clone();
        let pool = self.pool.clone();
        
        async_stream::stream! {
            let mut rows = sqlx::query(&query).fetch(&pool);
            
            while let Some(row_result) = rows.next().await {
                match row_result {
                    Ok(row) => {
                        match DatabaseRecord::from_row(row) {
                            Ok(record) => yield Ok(record),
                            Err(e) => yield Err(DatabaseError::ConversionError(e)),
                        }
                    }
                    Err(e) => yield Err(DatabaseError::QueryError(e)),
                }
            }
        }
    }
    
    async fn validate_connection(&self) -> Result<(), Self::Error> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(DatabaseError::ConnectionError)?;
        
        Ok(())
    }
    
    async fn get_metadata(&self) -> Result<SourceMetadata, Self::Error> {
        // Executa EXPLAIN para obter informações da query
        let explain_query = format!("EXPLAIN (FORMAT JSON) {}", self.query);
        let row = sqlx::query(&explain_query)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::QueryError)?;
        
        let plan: serde_json::Value = row.get(0);
        
        Ok(SourceMetadata {
            source_type: "database".to_string(),
            size_bytes: 0, // Não disponível para queries
            last_modified: None,
            estimated_records: extract_row_estimate(&plan),
            schema: None,
        })
    }
}

fn extract_row_estimate(plan: &serde_json::Value) -> Option<usize> {
    plan.get(0)?
        .get("Plan")?
        .get("Plan Rows")?
        .as_f64()
        .map(|f| f as usize)
}
```

### Transform API

#### FilterTransform

```rust
use etlrs::transform::{FilterTransform, FilterPredicate};

/// Transformador que filtra registros baseado em predicados
pub struct FilterTransform<T> {
    predicate: Box<dyn FilterPredicate<T> + Send + Sync>,
}

impl<T> FilterTransform<T> {
    /// Cria novo filtro com predicado
    pub fn new(predicate: impl FilterPredicate<T> + Send + Sync + 'static) -> Self {
        Self {
            predicate: Box::new(predicate),
        }
    }
    
    /// Cria filtro com closure
    pub fn from_fn<F>(predicate: F) -> Self 
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        Self::new(ClosurePredicate::new(predicate))
    }
}

#[async_trait]
impl<T> Transform<T, T> for FilterTransform<T> 
where
    T: Send + Sync + 'static,
{
    type Error = TransformError;
    
    async fn transform_item(&self, item: T) -> Result<Option<T>, Self::Error> {
        if self.predicate.test(&item).await? {
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }
    
    async fn transform_batch(&self, items: Vec<T>) -> Result<Vec<T>, Self::Error> {
        let mut results = Vec::new();
        
        for item in items {
            if self.predicate.test(&item).await? {
                results.push(item);
            }
        }
        
        Ok(results)
    }
    
    fn transform_stream(
        &self,
        input: impl Stream<Item = Result<T, Self::Error>>
    ) -> impl Stream<Item = Result<T, Self::Error>> {
        let predicate = self.predicate.clone();
        
        input.filter_map(move |result| {
            let predicate = predicate.clone();
            async move {
                match result {
                    Ok(item) => {
                        match predicate.test(&item).await {
                            Ok(true) => Some(Ok(item)),
                            Ok(false) => None,
                            Err(e) => Some(Err(e)),
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        })
    }
    
    async fn validate(&self, sample: &T) -> Result<(), Self::Error> {
        self.predicate.test(sample).await?;
        Ok(())
    }
}

/// Trait para predicados de filtro
#[async_trait]
pub trait FilterPredicate<T>: Send + Sync {
    async fn test(&self, item: &T) -> Result<bool, TransformError>;
}

/// Predicado baseado em closure
pub struct ClosurePredicate<T, F> {
    func: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> ClosurePredicate<T, F>
where
    F: Fn(&T) -> bool + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, F> FilterPredicate<T> for ClosurePredicate<T, F>
where
    T: Send + Sync,
    F: Fn(&T) -> bool + Send + Sync,
{
    async fn test(&self, item: &T) -> Result<bool, TransformError> {
        Ok((self.func)(item))
    }
}
```

#### MapTransform

```rust
/// Transformador que mapeia registros
pub struct MapTransform<I, O, F> {
    mapper: F,
    _phantom: std::marker::PhantomData<(I, O)>,
}

impl<I, O, F> MapTransform<I, O, F>
where
    F: Fn(I) -> Result<O, TransformError> + Send + Sync,
{
    pub fn new(mapper: F) -> Self {
        Self {
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<I, O, F> Transform<I, O> for MapTransform<I, O, F>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    F: Fn(I) -> Result<O, TransformError> + Send + Sync,
{
    type Error = TransformError;
    
    async fn transform_item(&self, item: I) -> Result<O, Self::Error> {
        (self.mapper)(item)
    }
    
    async fn transform_batch(&self, items: Vec<I>) -> Result<Vec<O>, Self::Error> {
        items.into_iter()
            .map(|item| (self.mapper)(item))
            .collect()
    }
    
    fn transform_stream(
        &self,
        input: impl Stream<Item = Result<I, Self::Error>>
    ) -> impl Stream<Item = Result<O, Self::Error>> {
        let mapper = &self.mapper;
        
        input.map(move |result| {
            result.and_then(|item| (mapper)(item))
        })
    }
    
    async fn validate(&self, sample: &I) -> Result<(), Self::Error> {
        // Clone sample for validation (if possible)
        // This is a simplified example
        Ok(())
    }
}
```

### Load API

#### JSON Loader

```rust
use etlrs::load::{JsonLoader, JsonConfig};

/// Carregador para arquivos JSON
pub struct JsonLoader {
    file_path: PathBuf,
    config: JsonConfig,
}

impl JsonLoader {
    /// Cria novo carregador JSON
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            config: JsonConfig::default(),
        }
    }
    
    /// Configura formato (JSON ou JSONL)
    pub fn with_format(mut self, format: JsonFormat) -> Self {
        self.config.format = format;
        self
    }
    
    /// Configura se deve fazer append
    pub fn with_append(mut self, append: bool) -> Self {
        self.config.append = append;
        self
    }
}

#[async_trait]
impl<T> Load<T> for JsonLoader 
where
    T: Serialize + Send + Sync,
{
    type Error = JsonError;
    
    async fn load(&self, data: Vec<T>) -> Result<LoadResult, Self::Error> {
        let start_time = Instant::now();
        
        let file = if self.config.append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .await
        } else {
            File::create(&self.file_path).await
        }.map_err(JsonError::IoError)?;
        
        let mut writer = BufWriter::new(file);
        
        match self.config.format {
            JsonFormat::Json => {
                let json_string = serde_json::to_string_pretty(&data)
                    .map_err(JsonError::SerializationError)?;
                writer.write_all(json_string.as_bytes()).await
                    .map_err(JsonError::IoError)?;
            }
            JsonFormat::JsonLines => {
                for item in &data {
                    let json_line = serde_json::to_string(item)
                        .map_err(JsonError::SerializationError)?;
                    writer.write_all(json_line.as_bytes()).await
                        .map_err(JsonError::IoError)?;
                    writer.write_all(b"\n").await
                        .map_err(JsonError::IoError)?;
                }
            }
        }
        
        writer.flush().await.map_err(JsonError::IoError)?;
        
        Ok(LoadResult {
            records_loaded: data.len(),
            bytes_written: writer.get_ref().metadata().await
                .map(|m| m.len())
                .unwrap_or(0),
            duration: start_time.elapsed(),
            destination: self.file_path.to_string_lossy().to_string(),
        })
    }
    
    async fn load_stream(
        &self,
        mut stream: impl Stream<Item = Result<T, Self::Error>> + Unpin
    ) -> Result<LoadResult, Self::Error> {
        let start_time = Instant::now();
        let mut records_loaded = 0;
        
        let file = if self.config.append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .await
        } else {
            File::create(&self.file_path).await
        }.map_err(JsonError::IoError)?;
        
        let mut writer = BufWriter::new(file);
        
        match self.config.format {
            JsonFormat::Json => {
                // Para JSON, precisamos coletar todos os itens primeiro
                let mut all_items = Vec::new();
                while let Some(result) = stream.next().await {
                    all_items.push(result?);
                }
                
                let json_string = serde_json::to_string_pretty(&all_items)
                    .map_err(JsonError::SerializationError)?;
                writer.write_all(json_string.as_bytes()).await
                    .map_err(JsonError::IoError)?;
                
                records_loaded = all_items.len();
            }
            JsonFormat::JsonLines => {
                while let Some(result) = stream.next().await {
                    let item = result?;
                    let json_line = serde_json::to_string(&item)
                        .map_err(JsonError::SerializationError)?;
                    writer.write_all(json_line.as_bytes()).await
                        .map_err(JsonError::IoError)?;
                    writer.write_all(b"\n").await
                        .map_err(JsonError::IoError)?;
                    
                    records_loaded += 1;
                }
            }
        }
        
        writer.flush().await.map_err(JsonError::IoError)?;
        
        Ok(LoadResult {
            records_loaded,
            bytes_written: writer.get_ref().metadata().await
                .map(|m| m.len())
                .unwrap_or(0),
            duration: start_time.elapsed(),
            destination: self.file_path.to_string_lossy().to_string(),
        })
    }
    
    async fn validate_connection(&self) -> Result<(), Self::Error> {
        // Verifica se pode escrever no diretório
        if let Some(parent) = self.file_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(JsonError::IoError)?;
            }
        }
        
        // Testa escrita criando arquivo temporário
        let test_file = self.file_path.with_extension("test");
        File::create(&test_file).await
            .map_err(JsonError::IoError)?;
        tokio::fs::remove_file(&test_file).await
            .map_err(JsonError::IoError)?;
        
        Ok(())
    }
    
    fn get_load_stats(&self) -> LoadStats {
        LoadStats {
            destination_type: "json_file".to_string(),
            connection_pool_size: 1,
            active_connections: 1,
            total_loaded: 0, // Seria mantido em estado interno
            total_errors: 0,
            average_load_time: Duration::ZERO,
        }
    }
}
```

## Exemplos de Uso

### Pipeline Simples

```rust
use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configuração
    let config = PipelineConfig::builder()
        .batch_size(1000)
        .max_parallel_tasks(4)
        .retry_attempts(3)
        .timeout(Duration::from_secs(300))
        .build();
    
    // Extrator CSV
    let extractor = CsvExtractor::new("data/input.csv")
        .with_delimiter(b',')
        .with_headers(true);
    
    // Transformador de filtro + mapeamento
    let filter = FilterTransform::from_fn(|record: &CsvRecord| {
        record.get("status") == Some("active")
    });
    
    let mapper = MapTransform::new(|record: CsvRecord| {
        // Converte CSV para JSON
        Ok(JsonRecord::from_csv(record)?)
    });
    
    let transformer = filter.chain(mapper);
    
    // Carregador JSON
    let loader = JsonLoader::new("data/output.json")
        .with_format(JsonFormat::JsonLines);
    
    // Pipeline
    let pipeline = PipelineBuilder::new()
        .with_extractor(extractor)
        .with_transformer(transformer)
        .with_loader(loader)
        .with_config(config)
        .build()?;
    
    // Execução
    let result = pipeline.execute().await?;
    
    println!("Pipeline executado com sucesso!");
    println!("Registros processados: {}", result.records_processed);
    println!("Duração total: {:?}", result.total_duration);
    
    Ok(())
}
```

### Pipeline com Database

```rust
use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Extrator de database
    let extractor = DatabaseExtractor::new(
        "postgresql://user:pass@localhost/db",
        "SELECT id, name, email, created_at FROM users WHERE active = true"
    ).await?;
    
    // Transformador de validação + enriquecimento
    let validator = ValidateTransform::new(UserValidator::new());
    let enricher = EnrichTransform::new(UserEnricher::new());
    let transformer = validator.chain(enricher);
    
    // Carregador para banco de destino
    let loader = DatabaseLoader::new(
        "postgresql://user:pass@destination/db",
        "INSERT INTO processed_users (id, name, email, metadata, processed_at) VALUES ($1, $2, $3, $4, $5)"
    ).await?;
    
    // Pipeline com configuração avançada
    let config = PipelineConfig::builder()
        .batch_size(500)
        .max_parallel_tasks(8)
        .enable_checkpointing(true)
        .checkpoint_interval(Duration::from_secs(60))
        .build();
    
    let pipeline = PipelineBuilder::new()
        .with_extractor(extractor)
        .with_transformer(transformer)
        .with_loader(loader)
        .with_config(config)
        .build()?;
    
    // Execução com monitoramento
    let metrics_collector = MetricsCollector::new();
    let result = pipeline
        .with_metrics(metrics_collector)
        .execute_with_retry()
        .await?;
    
    Ok(())
}
```

### Pipeline Streaming

```rust
use etlrs::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Extrator streaming
    let extractor = KafkaExtractor::new("localhost:9092")
        .with_topic("input-topic")
        .with_consumer_group("etlrs-group");
    
    // Transformador real-time
    let transformer = RealTimeTransform::new()
        .with_window(Duration::from_secs(60))
        .with_aggregation(AggregationType::Sum);
    
    // Carregador streaming
    let loader = KafkaLoader::new("localhost:9092")
        .with_topic("output-topic");
    
    // Pipeline streaming
    let pipeline = StreamingPipeline::new()
        .with_extractor(extractor)
        .with_transformer(transformer)
        .with_loader(loader)
        .build()?;
    
    // Execução contínua
    let mut stream = pipeline.stream();
    
    while let Some(result) = stream.next().await {
        match result {
            Ok(batch_result) => {
                info!("Processed batch: {} records", batch_result.records_processed);
            }
            Err(e) => {
                error!("Batch processing failed: {}", e);
                // Implementar estratégia de retry ou dead letter queue
            }
        }
    }
    
    Ok(())
}
```

Esta documentação da API fornece uma base completa para usar todas as funcionalidades da biblioteca ETLRS, com exemplos práticos e APIs bem estruturadas.
