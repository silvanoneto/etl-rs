//! # Memory Loader
//! 
//! Módulo para carregamento de dados em memória.
//! Especialmente útil para testes e cenários onde os dados precisam ser mantidos na memória.

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{DataRow, PipelineResult};
use crate::traits::Loader;

/// Carregador que acumula dados em memória
/// 
/// Ideal para:
/// - Testes unitários e de integração
/// - Pipelines intermediários onde dados são processados em etapas
/// - Cenários onde é necessário acesso aos dados após carregamento
/// - Debug e desenvolvimento
/// 
/// Os dados são armazenados de forma thread-safe usando Arc<Mutex<>>.
/// 
/// # Exemplos
/// 
/// ```rust
/// use etlrs::load::memory::MemoryLoader;
/// 
/// async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
///     let loader = MemoryLoader::new();
///     
///     // dados seria Vec<DataRow>
///     let dados = vec![];
///     let resultado = loader.load(dados).await?;
///     
///     // Acessar dados armazenados
///     let dados_armazenados = loader.get_data().await;
///     println!("Armazenados {} registros", dados_armazenados.len());
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MemoryLoader {
    data: std::sync::Arc<std::sync::Mutex<Vec<DataRow>>>,
}

impl MemoryLoader {
    /// Cria um novo MemoryLoader
    /// 
    /// Inicializa com um vetor vazio para armazenamento dos dados
    pub fn new() -> Self {
        Self {
            data: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    /// Obtém uma cópia dos dados armazenados
    /// 
    /// # Retorna
    /// 
    /// Uma cópia de todos os dados que foram carregados no MemoryLoader
    pub async fn get_data(&self) -> Vec<DataRow> {
        self.data.lock().unwrap().clone()
    }
    
    /// Limpa todos os dados armazenados
    /// 
    /// Remove todos os registros da memória, útil para reutilizar
    /// o mesmo loader ou para testes
    pub async fn clear(&self) {
        self.data.lock().unwrap().clear();
    }
    
    /// Obtém o número de registros armazenados
    /// 
    /// # Retorna
    /// 
    /// O número total de registros na memória
    pub async fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }
    
    /// Verifica se está vazio
    /// 
    /// # Retorna
    /// 
    /// True se não há dados armazenados, false caso contrário
    pub async fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }
    
    /// Obtém uma referência aos dados sem clonagem (para operações rápidas)
    /// 
    /// # Atenção
    /// 
    /// Esta função retorna uma closure que recebe os dados e deve ser executada
    /// rapidamente para evitar bloquear outras threads
    pub async fn with_data<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&Vec<DataRow>) -> T,
    {
        let data = self.data.lock().unwrap();
        f(&*data)
    }
}

impl Default for MemoryLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Loader for MemoryLoader {
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();
        let mut result = PipelineResult::new();
        
        let mut stored_data = self.data.lock().unwrap();
        stored_data.extend(data.clone());
        
        result.rows_processed = data.len();
        result.rows_successful = data.len();
        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(result)
    }
    
    async fn load_batch(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        self.load(data).await
    }
    
    async fn finalize(&self) -> Result<()> {
        // Para memory loader, não há operação de finalização necessária
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Memory loader está sempre "saudável" se conseguir adquirir o lock
        match self.data.try_lock() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Lock contendido, mas não é erro fatal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DataValue;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_memory_loader_basic() {
        let loader = MemoryLoader::new();
        
        let mut row = HashMap::new();
        row.insert("test".to_string(), DataValue::String("value".to_string()));
        
        let data = vec![row];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 1);
        assert_eq!(result.rows_successful, 1);
        
        let stored_data = loader.get_data().await;
        assert_eq!(stored_data.len(), 1);
        assert_eq!(stored_data[0].get("test"), Some(&DataValue::String("value".to_string())));
    }
    
    #[tokio::test]
    async fn test_memory_loader_multiple_loads() {
        let loader = MemoryLoader::new();
        
        // Primeiro carregamento
        let mut row1 = HashMap::new();
        row1.insert("id".to_string(), DataValue::Integer(1));
        let result1 = loader.load(vec![row1]).await.unwrap();
        assert_eq!(result1.rows_processed, 1);
        
        // Segundo carregamento
        let mut row2 = HashMap::new();
        row2.insert("id".to_string(), DataValue::Integer(2));
        let result2 = loader.load(vec![row2]).await.unwrap();
        assert_eq!(result2.rows_processed, 1);
        
        // Verifica se ambos estão armazenados
        let stored_data = loader.get_data().await;
        assert_eq!(stored_data.len(), 2);
        assert_eq!(loader.len().await, 2);
        assert!(!loader.is_empty().await);
    }
    
    #[tokio::test]
    async fn test_memory_loader_clear() {
        let loader = MemoryLoader::new();
        
        let mut row = HashMap::new();
        row.insert("test".to_string(), DataValue::String("value".to_string()));
        
        loader.load(vec![row]).await.unwrap();
        assert_eq!(loader.len().await, 1);
        
        loader.clear().await;
        assert_eq!(loader.len().await, 0);
        assert!(loader.is_empty().await);
    }
    
    #[tokio::test]
    async fn test_memory_loader_with_data() {
        let loader = MemoryLoader::new();
        
        let mut row = HashMap::new();
        row.insert("count".to_string(), DataValue::Integer(42));
        
        loader.load(vec![row]).await.unwrap();
        
        // Teste da função with_data
        let sum = loader.with_data(|data| {
            data.iter()
                .filter_map(|row| row.get("count"))
                .filter_map(|v| match v {
                    DataValue::Integer(i) => Some(*i),
                    _ => None,
                })
                .sum::<i64>()
        }).await;
        
        assert_eq!(sum, 42);
    }
    
    #[tokio::test]
    async fn test_memory_loader_health_check() {
        let loader = MemoryLoader::new();
        let health = loader.health_check().await.unwrap();
        assert!(health);
    }
    
    #[tokio::test]
    async fn test_memory_loader_batch() {
        let loader = MemoryLoader::new();
        
        let mut rows = Vec::new();
        for i in 0..5 {
            let mut row = HashMap::new();
            row.insert("id".to_string(), DataValue::Integer(i));
            rows.push(row);
        }
        
        let result = loader.load_batch(rows).await.unwrap();
        assert_eq!(result.rows_processed, 5);
        assert_eq!(loader.len().await, 5);
    }
}
