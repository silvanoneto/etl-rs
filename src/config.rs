use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configuração principal do ETL
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ETLConfig {
    pub pipeline: PipelineConfig,
    pub features: FeatureFlags,
    pub observability: ObservabilityConfig,
    pub performance: PerformanceConfig,
}

/// Configuração do pipeline
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineConfig {
    pub batch_size: usize,
    pub parallel_workers: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: usize,
    pub retry_delay_ms: u64,
}

/// Flags de funcionalidades
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeatureFlags {
    pub enable_metrics: bool,
    pub enable_logging: bool,
    pub enable_tracing: bool,
    pub enable_validation: bool,
    pub enable_caching: bool,
}

/// Configuração de observabilidade
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub metrics_endpoint: Option<String>,
    pub tracing_endpoint: Option<String>,
    pub log_format: LogFormat,
}

/// Configuração de performance
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub memory_limit_mb: usize,
    pub disk_cache_size_mb: usize,
    pub connection_pool_size: usize,
    pub connection_timeout_seconds: u64,
}

/// Formato de log
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl Default for ETLConfig {
    fn default() -> Self {
        Self {
            pipeline: PipelineConfig::default(),
            features: FeatureFlags::default(),
            observability: ObservabilityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            parallel_workers: num_cpus::get(),
            timeout_seconds: 300,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_logging: true,
            enable_tracing: true,
            enable_validation: true,
            enable_caching: false,
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            metrics_endpoint: None,
            tracing_endpoint: None,
            log_format: LogFormat::Pretty,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 1024,
            disk_cache_size_mb: 512,
            connection_pool_size: 10,
            connection_timeout_seconds: 30,
        }
    }
}

impl ETLConfig {
    /// Cria um novo builder para configuração
    pub fn builder() -> ETLConfigBuilder {
        ETLConfigBuilder::default()
    }
    
    /// Carrega configuração do ambiente
    pub fn from_env() -> Result<Self, crate::error::ETLError> {
        let mut builder = Self::builder();
        
        // Carrega batch_size do ambiente
        if let Ok(batch_size) = std::env::var("ETL_BATCH_SIZE") {
            if let Ok(size) = batch_size.parse::<usize>() {
                builder = builder.batch_size(size);
            }
        }
        
        // Carrega parallel_workers do ambiente
        if let Ok(workers) = std::env::var("ETL_PARALLEL_WORKERS") {
            if let Ok(worker_count) = workers.parse::<usize>() {
                builder = builder.parallel_workers(worker_count);
            }
        }
        
        // Carrega timeout do ambiente
        if let Ok(timeout) = std::env::var("ETL_TIMEOUT_SECONDS") {
            if let Ok(timeout_val) = timeout.parse::<u64>() {
                builder = builder.timeout_seconds(timeout_val);
            }
        }
        
        // Carrega flags de features
        if let Ok(metrics) = std::env::var("ETL_ENABLE_METRICS") {
            if let Ok(enable) = metrics.parse::<bool>() {
                builder = builder.enable_metrics(enable);
            }
        }
        
        if let Ok(logging) = std::env::var("ETL_ENABLE_LOGGING") {
            if let Ok(enable) = logging.parse::<bool>() {
                builder = builder.enable_logging(enable);
            }
        }
        
        // Carrega log level
        if let Ok(level) = std::env::var("ETL_LOG_LEVEL") {
            builder = builder.log_level(level);
        }
        
        // Carrega limites de performance
        if let Ok(memory) = std::env::var("ETL_MEMORY_LIMIT_MB") {
            if let Ok(limit) = memory.parse::<usize>() {
                builder = builder.memory_limit_mb(limit);
            }
        }
        
        builder.build()
    }

    /// Carrega configuração de arquivo
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, crate::error::ETLError> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
    
    /// Carrega configuração de string TOML
    pub fn from_toml(toml_str: &str) -> Result<Self, crate::error::ETLError> {
        let config = config::Config::builder()
            .add_source(config::File::from_str(toml_str, config::FileFormat::Toml))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
    
    /// Valida a configuração
    pub fn validate(&self) -> Result<(), crate::error::ETLError> {
        use crate::error::{ETLError, ConfigError};
        
        if self.pipeline.batch_size == 0 {
            return Err(ETLError::Config(ConfigError::InvalidValue {
                param: "batch_size".to_string(),
                value: "0".to_string(),
            }));
        }
        
        if self.pipeline.parallel_workers == 0 {
            return Err(ETLError::Config(ConfigError::InvalidValue {
                param: "parallel_workers".to_string(),
                value: "0".to_string(),
            }));
        }
        
        if self.performance.memory_limit_mb == 0 {
            return Err(ETLError::Config(ConfigError::InvalidValue {
                param: "memory_limit_mb".to_string(),
                value: "0".to_string(),
            }));
        }
        
        Ok(())
    }
}

/// Builder para configuração ETL
#[derive(Default)]
pub struct ETLConfigBuilder {
    config: ETLConfig,
}

impl ETLConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.pipeline.batch_size = size;
        self
    }
    
    pub fn parallel_workers(mut self, workers: usize) -> Self {
        self.config.pipeline.parallel_workers = workers;
        self
    }
    
    pub fn timeout_seconds(mut self, timeout: u64) -> Self {
        self.config.pipeline.timeout_seconds = timeout;
        self
    }
    
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config.features.enable_metrics = enable;
        self
    }
    
    pub fn enable_logging(mut self, enable: bool) -> Self {
        self.config.features.enable_logging = enable;
        self
    }
    
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.config.observability.log_level = level.into();
        self
    }
    
    pub fn memory_limit_mb(mut self, limit: usize) -> Self {
        self.config.performance.memory_limit_mb = limit;
        self
    }
    
    pub fn connection_pool_size(mut self, size: usize) -> Self {
        self.config.performance.connection_pool_size = size;
        self
    }
    
    pub fn build(self) -> Result<ETLConfig, crate::error::ETLError> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Configuração específica para extratores
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractorConfig {
    pub source_type: String,
    pub connection_string: Option<String>,
    pub file_path: Option<String>,
    pub query: Option<String>,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: u64,
}

/// Configuração específica para transformadores
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransformerConfig {
    pub transform_type: String,
    pub parameters: HashMap<String, String>,
    pub validation_rules: Vec<String>,
    pub error_handling: ErrorHandling,
}

/// Configuração específica para carregadores
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoaderConfig {
    pub destination_type: String,
    pub connection_string: Option<String>,
    pub file_path: Option<String>,
    pub batch_size: usize,
    pub conflict_resolution: ConflictResolution,
}

/// Estratégias de tratamento de erro
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ErrorHandling {
    Skip,
    Fail,
    Retry { max_attempts: usize, delay_ms: u64 },
    Log,
}

/// Estratégias de resolução de conflito
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ConflictResolution {
    Ignore,
    Overwrite,
    Merge,
    Fail,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = ETLConfig::default();
        assert_eq!(config.pipeline.batch_size, 1000);
        assert!(config.features.enable_metrics);
        assert_eq!(config.observability.log_level, "info");
    }
    
    #[test]
    fn test_config_builder() {
        let config = ETLConfig::builder()
            .batch_size(2000)
            .parallel_workers(8)
            .enable_metrics(false)
            .log_level("debug")
            .build()
            .unwrap();
        
        assert_eq!(config.pipeline.batch_size, 2000);
        assert_eq!(config.pipeline.parallel_workers, 8);
        assert!(!config.features.enable_metrics);
        assert_eq!(config.observability.log_level, "debug");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = ETLConfig::default();
        config.pipeline.batch_size = 0;
        
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
        [pipeline]
        batch_size = 500
        parallel_workers = 4
        timeout_seconds = 300
        retry_attempts = 3
        retry_delay_ms = 1000
        
        [features]
        enable_metrics = false
        enable_logging = true
        enable_tracing = false
        enable_validation = true
        enable_caching = false
        
        [observability]
        log_level = "info"
        log_format = "json"
        
        [performance]
        memory_limit_mb = 512
        disk_cache_size_mb = 256
        connection_pool_size = 10
        connection_timeout_seconds = 30
        "#;
        
        let config = ETLConfig::from_toml(toml_str).unwrap();
        assert_eq!(config.pipeline.batch_size, 500);
        assert_eq!(config.pipeline.parallel_workers, 4);
        assert!(!config.features.enable_metrics);
    }
}
