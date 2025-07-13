//! Sistema de plugins para extensibilidade do pipeline

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{DataRow, PipelineEvent};
use std::collections::HashMap;

/// Trait para plugins do pipeline
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Nome do plugin
    fn name(&self) -> &str;
    
    /// Versão do plugin
    fn version(&self) -> &str;
    
    /// Descrição do plugin
    fn description(&self) -> &str;
    
    /// Inicializa o plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Finaliza o plugin
    async fn finalize(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado antes da extração
    async fn before_extract(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado após a extração
    async fn after_extract(&self, _context: &PluginContext, _data: &[DataRow]) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado antes da transformação
    async fn before_transform(&self, _context: &PluginContext, _data: &[DataRow]) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado após a transformação
    async fn after_transform(&self, _context: &PluginContext, _data: &[DataRow]) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado antes do carregamento
    async fn before_load(&self, _context: &PluginContext, _data: &[DataRow]) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado após o carregamento
    async fn after_load(&self, _context: &PluginContext, _result: &crate::types::PipelineResult) -> Result<()> {
        Ok(())
    }
    
    /// Hook executado quando um evento é emitido
    async fn on_event(&self, _context: &PluginContext, _event: &PipelineEvent) -> Result<()> {
        Ok(())
    }
}

/// Contexto fornecido aos plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub pipeline_id: String,
    pub config: crate::config::ETLConfig,
    pub metadata: HashMap<String, String>,
}

impl PluginContext {
    pub fn new(pipeline_id: String, config: crate::config::ETLConfig) -> Self {
        Self {
            pipeline_id,
            config,
            metadata: HashMap::new(),
        }
    }
    
    /// Adiciona metadados ao contexto
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Obtém metadados do contexto
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Registry de plugins
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }
    
    /// Registra um plugin
    pub fn register<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }
    
    /// Lista todos os plugins registrados
    pub fn list_plugins(&self) -> Vec<(&str, &str, &str)> {
        self.plugins
            .iter()
            .map(|p| (p.name(), p.version(), p.description()))
            .collect()
    }
    
    /// Inicializa todos os plugins
    pub async fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize().await?;
        }
        Ok(())
    }
    
    /// Finaliza todos os plugins
    pub async fn finalize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.finalize().await?;
        }
        Ok(())
    }
    
    /// Executa hook before_extract em todos os plugins
    pub async fn before_extract(&self, context: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_extract(context).await?;
        }
        Ok(())
    }
    
    /// Executa hook after_extract em todos os plugins
    pub async fn after_extract(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_extract(context, data).await?;
        }
        Ok(())
    }
    
    /// Executa hook before_transform em todos os plugins
    pub async fn before_transform(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_transform(context, data).await?;
        }
        Ok(())
    }
    
    /// Executa hook after_transform em todos os plugins
    pub async fn after_transform(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_transform(context, data).await?;
        }
        Ok(())
    }
    
    /// Executa hook before_load em todos os plugins
    pub async fn before_load(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        for plugin in &self.plugins {
            plugin.before_load(context, data).await?;
        }
        Ok(())
    }
    
    /// Executa hook after_load em todos os plugins
    pub async fn after_load(&self, context: &PluginContext, result: &crate::types::PipelineResult) -> Result<()> {
        for plugin in &self.plugins {
            plugin.after_load(context, result).await?;
        }
        Ok(())
    }
    
    /// Executa hook on_event em todos os plugins
    pub async fn on_event(&self, context: &PluginContext, event: &PipelineEvent) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_event(context, event).await?;
        }
        Ok(())
    }
}

/// Plugin de exemplo para logging detalhado
pub struct LoggingPlugin {
    name: String,
}

impl LoggingPlugin {
    pub fn new() -> Self {
        Self {
            name: "LoggingPlugin".to_string(),
        }
    }
}

impl Default for LoggingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "Plugin que faz logging detalhado de cada fase do pipeline"
    }
    
    async fn before_extract(&self, context: &PluginContext) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            "Plugin {}: Iniciando extração",
            self.name
        );
        Ok(())
    }
    
    async fn after_extract(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            rows_extracted = data.len(),
            "Plugin {}: Extração concluída",
            self.name
        );
        Ok(())
    }
    
    async fn before_transform(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            rows_to_transform = data.len(),
            "Plugin {}: Iniciando transformação",
            self.name
        );
        Ok(())
    }
    
    async fn after_transform(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            rows_transformed = data.len(),
            "Plugin {}: Transformação concluída",
            self.name
        );
        Ok(())
    }
    
    async fn before_load(&self, context: &PluginContext, data: &[DataRow]) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            rows_to_load = data.len(),
            "Plugin {}: Iniciando carregamento",
            self.name
        );
        Ok(())
    }
    
    async fn after_load(&self, context: &PluginContext, result: &crate::types::PipelineResult) -> Result<()> {
        tracing::info!(
            pipeline_id = %context.pipeline_id,
            rows_processed = result.rows_processed,
            rows_successful = result.rows_successful,
            rows_failed = result.rows_failed,
            success_rate = result.success_rate(),
            "Plugin {}: Carregamento concluído",
            self.name
        );
        Ok(())
    }
    
    async fn on_event(&self, context: &PluginContext, event: &PipelineEvent) -> Result<()> {
        match event {
            PipelineEvent::StateChanged { old_state, new_state, .. } => {
                tracing::info!(
                    pipeline_id = %context.pipeline_id,
                    old_state = %old_state,
                    new_state = %new_state,
                    "Plugin {}: Estado alterado",
                    self.name
                );
            }
            PipelineEvent::Error { error, .. } => {
                tracing::error!(
                    pipeline_id = %context.pipeline_id,
                    error = %error,
                    "Plugin {}: Erro detectado",
                    self.name
                );
            }
            _ => {}
        }
        Ok(())
    }
}

/// Plugin de exemplo para métricas customizadas
pub struct MetricsPlugin {
    name: String,
    start_time: std::sync::Mutex<Option<std::time::Instant>>,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            name: "MetricsPlugin".to_string(),
            start_time: std::sync::Mutex::new(None),
        }
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn description(&self) -> &str {
        "Plugin que coleta métricas detalhadas de performance"
    }
    
    async fn before_extract(&self, _context: &PluginContext) -> Result<()> {
        *self.start_time.lock().unwrap() = Some(std::time::Instant::now());
        Ok(())
    }
    
    async fn after_load(&self, context: &PluginContext, result: &crate::types::PipelineResult) -> Result<()> {
        if let Some(start) = *self.start_time.lock().unwrap() {
            let total_time = start.elapsed();
            tracing::info!(
                pipeline_id = %context.pipeline_id,
                total_time_ms = total_time.as_millis(),
                throughput_rows_per_sec = if total_time.as_secs() > 0 { 
                    result.rows_processed as u64 / total_time.as_secs() 
                } else { 
                    result.rows_processed as u64 
                },
                "Plugin {}: Métricas de performance",
                self.name
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ETLConfig;

    #[tokio::test]
    async fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        
        registry.register(LoggingPlugin::new());
        registry.register(MetricsPlugin::new());
        
        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 2);
        
        let config = ETLConfig::default();
        let context = PluginContext::new("test-pipeline".to_string(), config);
        
        // Testa hooks
        assert!(registry.before_extract(&context).await.is_ok());
        assert!(registry.after_extract(&context, &[]).await.is_ok());
        
        let result = crate::types::PipelineResult::new();
        assert!(registry.after_load(&context, &result).await.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_context() {
        let config = ETLConfig::default();
        let mut context = PluginContext::new("test-pipeline".to_string(), config);
        
        context.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(context.get_metadata("key1"), Some(&"value1".to_string()));
        assert_eq!(context.get_metadata("key2"), None);
    }
}
