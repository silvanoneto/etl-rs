//! Sistema de eventos para observabilidade do pipeline

use async_trait::async_trait;
use crate::error::Result;
use crate::traits::EventEmitter;
use crate::types::PipelineEvent;
use tracing::{info, error};

/// Implementação simples de EventEmitter que logga eventos
#[derive(Debug, Clone, Default)]
pub struct LoggingEventEmitter;

impl LoggingEventEmitter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventEmitter for LoggingEventEmitter {
    async fn emit(&self, event: PipelineEvent) -> Result<()> {
        match event {
            PipelineEvent::Started { pipeline_id, timestamp } => {
                info!(
                    pipeline_id = %pipeline_id,
                    timestamp = ?timestamp,
                    "Pipeline iniciado"
                );
            }
            PipelineEvent::StateChanged { 
                pipeline_id, 
                old_state, 
                new_state, 
                timestamp 
            } => {
                info!(
                    pipeline_id = %pipeline_id,
                    old_state = %old_state,
                    new_state = %new_state,
                    timestamp = ?timestamp,
                    "Estado do pipeline alterado"
                );
            }
            PipelineEvent::BatchProcessed { 
                pipeline_id, 
                batch_number, 
                rows_count, 
                timestamp 
            } => {
                info!(
                    pipeline_id = %pipeline_id,
                    batch_number = batch_number,
                    rows_count = rows_count,
                    timestamp = ?timestamp,
                    "Batch processado"
                );
            }
            PipelineEvent::Error { pipeline_id, error, timestamp } => {
                error!(
                    pipeline_id = %pipeline_id,
                    error = %error,
                    timestamp = ?timestamp,
                    "Erro no pipeline"
                );
            }
            PipelineEvent::Completed { pipeline_id, result, timestamp } => {
                info!(
                    pipeline_id = %pipeline_id,
                    rows_processed = result.rows_processed,
                    rows_successful = result.rows_successful,
                    rows_failed = result.rows_failed,
                    execution_time_ms = result.execution_time_ms,
                    success_rate = result.success_rate(),
                    timestamp = ?timestamp,
                    "Pipeline concluído"
                );
            }
        }
        
        Ok(())
    }
}

/// EventEmitter que armazena eventos em memória para testes
#[derive(Debug, Clone, Default)]
pub struct InMemoryEventEmitter {
    events: std::sync::Arc<std::sync::Mutex<Vec<PipelineEvent>>>,
}

impl InMemoryEventEmitter {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    /// Retorna todos os eventos capturados
    pub fn get_events(&self) -> Vec<PipelineEvent> {
        self.events.lock().unwrap().clone()
    }
    
    /// Limpa todos os eventos armazenados
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
    
    /// Retorna o número de eventos capturados
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

#[async_trait]
impl EventEmitter for InMemoryEventEmitter {
    async fn emit(&self, event: PipelineEvent) -> Result<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PipelineState;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_logging_event_emitter() {
        let emitter = LoggingEventEmitter::new();
        
        let event = PipelineEvent::Started {
            pipeline_id: "test-pipeline".to_string(),
            timestamp: SystemTime::now(),
        };
        
        // Deve loggar sem erro
        assert!(emitter.emit(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_in_memory_event_emitter() {
        let emitter = InMemoryEventEmitter::new();
        
        assert_eq!(emitter.event_count(), 0);
        
        let event1 = PipelineEvent::Started {
            pipeline_id: "test-pipeline".to_string(),
            timestamp: SystemTime::now(),
        };
        
        let event2 = PipelineEvent::StateChanged {
            pipeline_id: "test-pipeline".to_string(),
            old_state: PipelineState::Idle,
            new_state: PipelineState::Extracting,
            timestamp: SystemTime::now(),
        };
        
        emitter.emit(event1).await.unwrap();
        emitter.emit(event2).await.unwrap();
        
        assert_eq!(emitter.event_count(), 2);
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 2);
        
        emitter.clear();
        assert_eq!(emitter.event_count(), 0);
    }
}
