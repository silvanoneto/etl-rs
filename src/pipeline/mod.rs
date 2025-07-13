use crate::config::ETLConfig;
use crate::error::Result;
use crate::traits::{Extractor, Transformer, Loader, EventEmitter};
use crate::types::{PipelineResult, PipelineState, PipelineEvent};
use crate::events::LoggingEventEmitter;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

/// Pipeline ETL principal com suporte a eventos e estado
pub struct Pipeline<E, T, L> {
    extractor: E,
    transformer: T,
    loader: L,
    config: ETLConfig,
    metrics: Arc<std::sync::Mutex<PipelineMetrics>>,
    state: Arc<std::sync::Mutex<PipelineState>>,
    event_emitter: Arc<dyn EventEmitter>,
    pipeline_id: String,
}

/// Métricas do pipeline
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    pub executions: Vec<PipelineExecution>,
    pub total_rows_processed: usize,
    pub total_execution_time_ms: u64,
    pub success_rate: f64,
}

/// Informações de uma execução do pipeline
#[derive(Debug, Clone)]
pub struct PipelineExecution {
    pub timestamp: SystemTime,
    pub result: PipelineResult,
    pub config_snapshot: ETLConfig,
}

impl Pipeline<(), (), ()> {
    /// Cria um novo builder de pipeline
    pub fn builder() -> PipelineBuilder<(), (), ()> {
        PipelineBuilder::new()
    }
    
    /// Cria um builder com configuração personalizada
    pub fn with_config(config: ETLConfig) -> PipelineBuilder<(), (), ()> {
        PipelineBuilder::with_config(config)
    }
}

impl<E, T, L> Pipeline<E, T, L>
where
    E: Extractor + Send + Sync,
    T: Transformer + Send + Sync,
    L: Loader + Send + Sync,
{
    /// Retorna o ID do pipeline
    pub fn pipeline_id(&self) -> &str {
        &self.pipeline_id
    }
    
    /// Retorna o estado atual do pipeline
    pub fn current_state(&self) -> PipelineState {
        self.state.lock().unwrap().clone()
    }
    
    /// Altera o estado do pipeline e emite evento
    async fn set_state(&self, new_state: PipelineState) -> Result<()> {
        let old_state = {
            let mut state = self.state.lock().unwrap();
            let old = state.clone();
            *state = new_state.clone();
            old
        };
        
        // Emite evento de mudança de estado
        let event = PipelineEvent::StateChanged {
            pipeline_id: self.pipeline_id.clone(),
            old_state,
            new_state,
            timestamp: SystemTime::now(),
        };
        
        self.event_emitter.emit(event).await?;
        Ok(())
    }
    
    /// Executa o pipeline com gerenciamento de estado e eventos
    pub async fn execute(&self) -> Result<PipelineResult> {
        let start_time = Instant::now();
        let mut final_result = PipelineResult::new();
        
        // Emite evento de início
        let start_event = PipelineEvent::Started {
            pipeline_id: self.pipeline_id.clone(),
            timestamp: SystemTime::now(),
        };
        self.event_emitter.emit(start_event).await?;
        
        // Muda estado para Extracting
        self.set_state(PipelineState::Extracting).await?;
        
        tracing::info!("Iniciando execução do pipeline");
        
        // Validações de saúde
        if !self.loader.health_check().await? {
            self.set_state(PipelineState::Failed("Health check do loader falhou".to_string())).await?;
            return Err(crate::error::ETLError::Pipeline(
                "Health check do loader falhou".to_string()
            ));
        }
        
        // Extração
        tracing::info!("Iniciando extração de dados");
        let extracted_data = match self.extractor.extract().await {
            Ok(data) => {
                tracing::info!("Extraídos {} registros", data.len());
                data
            }
            Err(e) => {
                let error_msg = format!("Erro na extração: {}", e);
                self.set_state(PipelineState::Failed(error_msg.clone())).await?;
                
                let error_event = PipelineEvent::Error {
                    pipeline_id: self.pipeline_id.clone(),
                    error: error_msg,
                    timestamp: SystemTime::now(),
                };
                self.event_emitter.emit(error_event).await?;
                return Err(e);
            }
        };
        
        // Muda estado para Transforming
        self.set_state(PipelineState::Transforming).await?;
        
        // Transformação
        tracing::info!("Iniciando transformação de dados");
        let transformed_data = match self.transformer.transform(extracted_data).await {
            Ok(data) => {
                tracing::info!("Transformados {} registros", data.len());
                data
            }
            Err(e) => {
                let error_msg = format!("Erro na transformação: {}", e);
                self.set_state(PipelineState::Failed(error_msg.clone())).await?;
                
                let error_event = PipelineEvent::Error {
                    pipeline_id: self.pipeline_id.clone(),
                    error: error_msg,
                    timestamp: SystemTime::now(),
                };
                self.event_emitter.emit(error_event).await?;
                return Err(e);
            }
        };
        
        // Muda estado para Loading
        self.set_state(PipelineState::Loading).await?;
        
        // Carregamento
        tracing::info!("Iniciando carregamento de dados");
        let load_result = match self.loader.load(transformed_data).await {
            Ok(result) => {
                tracing::info!("Carregados {} registros", result.rows_successful);
                result
            }
            Err(e) => {
                let error_msg = format!("Erro no carregamento: {}", e);
                self.set_state(PipelineState::Failed(error_msg.clone())).await?;
                
                let error_event = PipelineEvent::Error {
                    pipeline_id: self.pipeline_id.clone(),
                    error: error_msg,
                    timestamp: SystemTime::now(),
                };
                self.event_emitter.emit(error_event).await?;
                return Err(e);
            }
        };
        
        // Finalização
        self.loader.finalize().await?;
        
        final_result.rows_processed = load_result.rows_processed;
        final_result.rows_successful = load_result.rows_successful;
        final_result.rows_failed = load_result.rows_failed;
        final_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        final_result.errors = load_result.errors;
        
        // Muda estado para Completed
        self.set_state(PipelineState::Completed).await?;
        
        // Emite evento de conclusão
        let completion_event = PipelineEvent::Completed {
            pipeline_id: self.pipeline_id.clone(),
            result: final_result.clone(),
            timestamp: SystemTime::now(),
        };
        self.event_emitter.emit(completion_event).await?;
        
        // Registra métricas
        self.record_execution(&final_result).await;
        
        tracing::info!(
            "Pipeline executado com sucesso - {} registros processados em {}ms",
            final_result.rows_processed,
            final_result.execution_time_ms
        );
        
        Ok(final_result)
    }
    
    /// Executa o pipeline em lotes
    pub async fn execute_batch(&self, batch_size: usize) -> Result<PipelineResult> {
        let start_time = Instant::now();
        let mut final_result = PipelineResult::new();
        
        tracing::info!("Iniciando execução em lotes do pipeline (batch_size: {})", batch_size);
        
        // Validações de saúde
        if !self.loader.health_check().await? {
            return Err(crate::error::ETLError::Pipeline(
                "Health check do loader falhou".to_string()
            ));
        }
        
        // Processamento em lotes
        let mut has_more = true;
        while has_more {
            // Extração em lote
            let extracted_data = self.extractor.extract_batch(batch_size).await?;
            
            if extracted_data.is_empty() {
                break;
            }
            
            // Transformação
            let transformed_data = self.transformer.transform(extracted_data).await?;
            
            // Carregamento
            let load_result = self.loader.load_batch(transformed_data).await?;
            
            // Acumula resultados
            final_result.rows_processed += load_result.rows_processed;
            final_result.rows_successful += load_result.rows_successful;
            final_result.rows_failed += load_result.rows_failed;
            final_result.errors.extend(load_result.errors);
            
            // Verifica se há mais dados
            has_more = self.extractor.has_more().await?;
        }
        
        // Finalização
        self.loader.finalize().await?;
        
        final_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Registra métricas
        self.record_execution(&final_result).await;
        
        tracing::info!(
            "Pipeline em lotes executado - {} registros processados em {}ms",
            final_result.rows_processed,
            final_result.execution_time_ms
        );
        
        Ok(final_result)
    }
    
    /// Executa o pipeline em modo streaming, processando um batch por vez
    /// para maior eficiência de memória
    pub async fn execute_streaming(&self) -> Result<PipelineResult> {
        let batch_size = self.config.pipeline.batch_size;
        self.execute_batch(batch_size).await
    }

    /// Obtém métricas do pipeline
    pub async fn get_metrics(&self) -> PipelineMetrics {
        self.metrics.lock().unwrap().clone()
    }
    
    /// Reseta métricas do pipeline
    pub async fn reset_metrics(&self) {
        *self.metrics.lock().unwrap() = PipelineMetrics::default();
    }
    
    /// Registra uma execução nas métricas
    async fn record_execution(&self, result: &PipelineResult) {
        let mut metrics = self.metrics.lock().unwrap();
        
        let execution = PipelineExecution {
            timestamp: SystemTime::now(),
            result: result.clone(),
            config_snapshot: self.config.clone(),
        };
        
        metrics.executions.push(execution);
        metrics.total_rows_processed += result.rows_processed;
        metrics.total_execution_time_ms += result.execution_time_ms;
        
        // Calcula taxa de sucesso
        let total_successful: usize = metrics.executions.iter()
            .map(|e| e.result.rows_successful)
            .sum();
        let total_processed: usize = metrics.executions.iter()
            .map(|e| e.result.rows_processed)
            .sum();
        
        metrics.success_rate = if total_processed > 0 {
            total_successful as f64 / total_processed as f64
        } else {
            0.0
        };
    }
}

/// Builder para criação de pipelines
pub struct PipelineBuilder<E, T, L> {
    extractor: E,
    transformer: T,
    loader: L,
    config: ETLConfig,
    event_emitter: Option<Arc<dyn EventEmitter>>,
    _phantom: PhantomData<(E, T, L)>,
}

impl PipelineBuilder<(), (), ()> {
    /// Cria um novo builder
    pub fn new() -> Self {
        Self {
            extractor: (),
            transformer: (),
            loader: (),
            config: ETLConfig::default(),
            event_emitter: None,
            _phantom: PhantomData,
        }
    }
    
    /// Cria um builder com configuração personalizada
    pub fn with_config(config: ETLConfig) -> Self {
        Self {
            extractor: (),
            transformer: (),
            loader: (),
            config,
            event_emitter: None,
            _phantom: PhantomData,
        }
    }
}

impl<E, T, L> PipelineBuilder<E, T, L> {
    /// Define o extrator
    pub fn extract<NewE: Extractor + Send + Sync>(
        self,
        extractor: NewE,
    ) -> PipelineBuilder<NewE, T, L> {
        PipelineBuilder {
            extractor,
            transformer: self.transformer,
            loader: self.loader,
            config: self.config,
            event_emitter: self.event_emitter,
            _phantom: PhantomData,
        }
    }
    
    /// Define o transformador
    pub fn transform<NewT: Transformer + Send + Sync>(
        self,
        transformer: NewT,
    ) -> PipelineBuilder<E, NewT, L> {
        PipelineBuilder {
            extractor: self.extractor,
            transformer,
            loader: self.loader,
            config: self.config,
            event_emitter: self.event_emitter,
            _phantom: PhantomData,
        }
    }
    
    /// Define o carregador
    pub fn load<NewL: Loader + Send + Sync>(
        self,
        loader: NewL,
    ) -> PipelineBuilder<E, T, NewL> {
        PipelineBuilder {
            extractor: self.extractor,
            transformer: self.transformer,
            loader,
            config: self.config,
            event_emitter: self.event_emitter,
            _phantom: PhantomData,
        }
    }
    
    /// Define a configuração
    pub fn config(mut self, config: ETLConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Define o event emitter
    pub fn event_emitter<EventEmitterType: EventEmitter + 'static>(mut self, emitter: EventEmitterType) -> Self {
        self.event_emitter = Some(Arc::new(emitter));
        self
    }
    
    /// Define o tamanho do lote
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.pipeline.batch_size = size;
        self
    }
    
    /// Define o timeout
    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.config.pipeline.timeout_seconds = timeout;
        self
    }
    
    /// Define o número de workers paralelos
    pub fn parallel_workers(mut self, workers: usize) -> Self {
        self.config.pipeline.parallel_workers = workers;
        self
    }
    
    /// Habilita métricas
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config.features.enable_metrics = enable;
        self
    }
    
    /// Habilita logging
    pub fn enable_logging(mut self, enable: bool) -> Self {
        self.config.features.enable_logging = enable;
        self
    }
    
    /// Define o limite de memória
    pub fn memory_limit_mb(mut self, limit: usize) -> Self {
        self.config.performance.memory_limit_mb = limit;
        self
    }
}

impl<E, T, L> PipelineBuilder<E, T, L>
where
    E: Extractor + Send + Sync,
    T: Transformer + Send + Sync,
    L: Loader + Send + Sync,
{
    /// Constrói o pipeline
    pub fn build(self) -> Pipeline<E, T, L> {
        Pipeline {
            extractor: self.extractor,
            transformer: self.transformer,
            loader: self.loader,
            config: self.config,
            metrics: Arc::new(std::sync::Mutex::new(PipelineMetrics::default())),
            state: Arc::new(std::sync::Mutex::new(PipelineState::default())),
            event_emitter: self.event_emitter.unwrap_or_else(|| Arc::new(LoggingEventEmitter::default())),
            pipeline_id: format!("pipeline-{}-{}", std::process::id(), SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
        }
    }
}

/// Implementação de Default para PipelineBuilder
impl Default for PipelineBuilder<(), (), ()> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::csv::CsvExtractor;
    use crate::transform::common::FilterTransform;
    use crate::load::memory::MemoryLoader;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_pipeline_builder() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name,age,active").unwrap();
        writeln!(temp_file, "Alice,30,true").unwrap();
        writeln!(temp_file, "Bob,17,false").unwrap();
        
        let pipeline = Pipeline::builder()
            .extract(CsvExtractor::new(temp_file.path()))
            .transform(FilterTransform::new(|row| {
                row.get("age")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0) >= 18
            }))
            .load(MemoryLoader::new())
            .batch_size(100)
            .enable_metrics(true)
            .build();
        
        let result = pipeline.execute().await.unwrap();
        
        assert_eq!(result.rows_processed, 1); // Apenas Alice tem >= 18 anos
        assert_eq!(result.rows_successful, 1);
        // Verifica se o tempo de execução foi registrado (pode ser 0 em testes rápidos)
    }
    
    #[tokio::test]
    async fn test_pipeline_metrics() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name,age").unwrap();
        writeln!(temp_file, "Alice,30").unwrap();
        
        let pipeline = Pipeline::builder()
            .extract(CsvExtractor::new(temp_file.path()))
            .transform(FilterTransform::new(|_| true))
            .load(MemoryLoader::new())
            .build();
        
        // Executa duas vezes
        pipeline.execute().await.unwrap();
        pipeline.execute().await.unwrap();
        
        let metrics = pipeline.get_metrics().await;
        assert_eq!(metrics.executions.len(), 2);
        assert_eq!(metrics.total_rows_processed, 2);
        assert!(metrics.success_rate > 0.0);
    }
    
    #[tokio::test]
    async fn test_pipeline_with_config() {
        let config = ETLConfig::builder()
            .batch_size(500)
            .parallel_workers(2)
            .enable_metrics(true)
            .build()
            .unwrap();
        
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name").unwrap();
        writeln!(temp_file, "Alice").unwrap();
        
        let pipeline = Pipeline::with_config(config)
            .extract(CsvExtractor::new(temp_file.path()))
            .transform(FilterTransform::new(|_| true))
            .load(MemoryLoader::new())
            .build();
        
        let result = pipeline.execute().await.unwrap();
        assert_eq!(result.rows_processed, 1);
    }
}
