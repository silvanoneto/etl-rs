use async_trait::async_trait;
use crate::error::Result;
use crate::types::{DataRow, PipelineResult};

/// Trait para componentes que extraem dados
#[async_trait]
pub trait Extractor: Send + Sync {
    /// Extrai dados da fonte
    async fn extract(&self) -> Result<Vec<DataRow>>;
    
    /// Extrai dados em lotes (streaming)
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        // Implementação padrão usa extract() e divide em lotes
        let data = self.extract().await?;
        Ok(data.into_iter().take(batch_size).collect())
    }
    
    /// Verifica se há mais dados disponíveis
    async fn has_more(&self) -> Result<bool> {
        Ok(false) // Implementação padrão
    }
    
    /// Reset do estado interno para reprocessamento
    async fn reset(&mut self) -> Result<()> {
        Ok(()) // Implementação padrão
    }
}

/// Trait para componentes que transformam dados
#[async_trait]
pub trait Transformer: Send + Sync {
    /// Transforma um lote de dados
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>>;
    
    /// Transforma uma única linha (implementação padrão)
    async fn transform_row(&self, row: DataRow) -> Result<DataRow> {
        let result = self.transform(vec![row]).await?;
        result.into_iter().next().ok_or_else(|| {
            crate::error::ETLError::Transform(
                crate::error::TransformError::ProcessingError(
                    "Nenhum resultado retornado da transformação".to_string()
                )
            )
        })
    }
    
    /// Valida os dados antes da transformação
    async fn validate(&self, _data: &[DataRow]) -> Result<Vec<String>> {
        Ok(Vec::new()) // Implementação padrão sem validação
    }
}

/// Trait para componentes que carregam dados
#[async_trait]
pub trait Loader: Send + Sync {
    /// Carrega dados para o destino
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult>;
    
    /// Carrega dados em lotes
    async fn load_batch(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        self.load(data).await // Implementação padrão
    }
    
    /// Finaliza o carregamento (flush, commit, etc.)
    async fn finalize(&self) -> Result<()> {
        Ok(()) // Implementação padrão
    }
    
    /// Verifica se o destino está disponível
    async fn health_check(&self) -> Result<bool> {
        Ok(true) // Implementação padrão
    }
}

/// Trait para componentes que validam dados
#[async_trait]
pub trait Validator: Send + Sync {
    /// Valida um lote de dados
    async fn validate(&self, data: &[DataRow]) -> Result<Vec<String>>;
    
    /// Valida uma única linha
    async fn validate_row(&self, row: &DataRow) -> Result<Vec<String>> {
        self.validate(&[row.clone()]).await
    }
}

/// Trait para componentes que fazem agregações
#[async_trait]
pub trait Aggregator: Send + Sync {
    /// Agrega dados
    async fn aggregate(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>>;
    
    /// Tipos de agregação suportados
    fn supported_operations(&self) -> Vec<String>;
}

/// Trait para componentes que filtram dados
#[async_trait]
pub trait Filter: Send + Sync {
    /// Filtra dados baseado em critérios
    async fn filter(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>>;
    
    /// Filtra uma única linha
    async fn filter_row(&self, row: &DataRow) -> Result<bool>;
}

/// Trait para componentes que fazem junções
#[async_trait]
pub trait Joiner: Send + Sync {
    /// Junta dados de duas fontes
    async fn join(&self, left: Vec<DataRow>, right: Vec<DataRow>) -> Result<Vec<DataRow>>;
    
    /// Tipo de junção
    fn join_type(&self) -> JoinType;
}

/// Tipos de junção suportados
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Trait para componentes que fazem cache
#[async_trait]
pub trait Cache: Send + Sync {
    /// Armazena dados no cache
    async fn store(&self, key: String, data: Vec<DataRow>) -> Result<()>;
    
    /// Recupera dados do cache
    async fn retrieve(&self, key: &str) -> Result<Option<Vec<DataRow>>>;
    
    /// Remove dados do cache
    async fn remove(&self, key: &str) -> Result<()>;
    
    /// Limpa todo o cache
    async fn clear(&self) -> Result<()>;
}

/// Trait para componentes que fazem monitoramento
#[async_trait]
pub trait Monitor: Send + Sync {
    /// Registra métricas
    async fn record_metric(&self, name: &str, value: f64) -> Result<()>;
    
    /// Registra evento
    async fn record_event(&self, event: &str, data: &DataRow) -> Result<()>;
    
    /// Obtém métricas
    async fn get_metrics(&self) -> Result<std::collections::HashMap<String, f64>>;
}

/// Trait para componentes que fazem logging
pub trait Logger: Send + Sync {
    /// Log de informação
    fn info(&self, message: &str);
    
    /// Log de warning
    fn warn(&self, message: &str);
    
    /// Log de erro
    fn error(&self, message: &str);
    
    /// Log de debug
    fn debug(&self, message: &str);
}

/// Trait para componentes que fazem retry
#[async_trait]
pub trait Retryable: Send + Sync {
    /// Executa operação com retry
    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T> + Send + Sync,
        T: Send + Sync;
    
    /// Configuração de retry
    fn retry_config(&self) -> RetryConfig;
}

/// Configuração de retry
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 1000,
            backoff_factor: 2.0,
        }
    }
}

/// Trait para emissão de eventos do pipeline
#[async_trait]
pub trait EventEmitter: Send + Sync {
    /// Emite um evento do pipeline
    async fn emit(&self, event: crate::types::PipelineEvent) -> Result<()>;
    
    /// Registra um listener para eventos específicos
    async fn subscribe(&self, _event_type: &str) -> Result<()> {
        Ok(()) // Implementação padrão vazia
    }
    
    /// Remove um listener
    async fn unsubscribe(&self, _event_type: &str) -> Result<()> {
        Ok(()) // Implementação padrão vazia
    }
}

/// Macro para implementar traits básicos
#[macro_export]
macro_rules! impl_basic_traits {
    ($type:ty) => {
        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", std::any::type_name::<$type>())
            }
        }
        
        impl Clone for $type {
            fn clone(&self) -> Self {
                Self::new()
            }
        }
    };
}
