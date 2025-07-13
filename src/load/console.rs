//! # Console Loader
//! 
//! Módulo para carregamento de dados em console/stdout.
//! Útil para debug, demonstrações e desenvolvimento.

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{DataRow, DataValue, PipelineResult};
use crate::traits::Loader;

/// Carregador para output em console/stdout
/// 
/// Permite exibir dados processados diretamente no terminal com:
/// - Modo pretty-print para melhor legibilidade humana
/// - Modo JSON para saída estruturada
/// - Formatação automática de tipos de dados
/// 
/// # Exemplos
/// 
/// ```rust
/// use etlrs::load::console::ConsoleLoader;
/// 
/// async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
///     let loader = ConsoleLoader::new()
///         .with_pretty(true);
///     
///     // dados seria Vec<DataRow>
///     let dados = vec![];
///     let resultado = loader.load(dados).await?;
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConsoleLoader {
    pretty: bool,
}

impl ConsoleLoader {
    /// Cria um novo ConsoleLoader
    /// 
    /// Por padrão, usa formatação pretty-print para melhor legibilidade
    pub fn new() -> Self {
        Self { pretty: true }
    }
    
    /// Define se deve usar formatação pretty-print
    /// 
    /// # Argumentos
    /// 
    /// * `pretty` - Se true, usa formatação estruturada. Se false, usa JSON compacto
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

impl Default for ConsoleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Loader for ConsoleLoader {
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();
        let mut result = PipelineResult::new();
        
        for (i, row) in data.iter().enumerate() {
            if self.pretty {
                println!("Record {}: {{", i + 1);
                for (key, value) in row {
                    match value {
                        DataValue::String(s) => println!("  {}: \"{}\"", key, s),
                        DataValue::Integer(i) => println!("  {}: {}", key, i),
                        DataValue::Float(f) => println!("  {}: {}", key, f),
                        DataValue::Boolean(b) => println!("  {}: {}", key, b),
                        DataValue::Null => println!("  {}: null", key),
                        DataValue::Array(arr) => println!("  {}: {:?}", key, arr),
                        DataValue::Object(obj) => println!("  {}: {:?}", key, obj),
                        DataValue::Date(date) => println!("  {}: {}", key, date.format("%Y-%m-%d")),
                        DataValue::DateTime(dt) => println!("  {}: {}", key, dt.format("%Y-%m-%d %H:%M:%S")),
                        DataValue::Timestamp(ts) => println!("  {}: {}", key, ts.to_rfc3339()),
                    }
                }
                println!("}}");
            } else {
                let json_value = serde_json::to_string(&row)?;
                println!("{}", json_value);
            }
        }
        
        result.rows_processed = data.len();
        result.rows_successful = data.len();
        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(result)
    }
    
    async fn load_batch(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        self.load(data).await
    }
    
    async fn finalize(&self) -> Result<()> {
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_console_loader() {
        let loader = ConsoleLoader::new();
        
        let mut row = HashMap::new();
        row.insert("name".to_string(), DataValue::String("Alice".to_string()));
        row.insert("age".to_string(), DataValue::Integer(30));
        
        let data = vec![row];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 1);
        assert_eq!(result.rows_successful, 1);
    }
    
    #[tokio::test]
    async fn test_console_loader_json_mode() {
        let loader = ConsoleLoader::new().with_pretty(false);
        
        let mut row = HashMap::new();
        row.insert("test".to_string(), DataValue::String("value".to_string()));
        
        let data = vec![row];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 1);
        assert_eq!(result.rows_successful, 1);
    }
    
    #[tokio::test]
    async fn test_console_loader_data_types() {
        let loader = ConsoleLoader::new();
        
        let mut row = HashMap::new();
        row.insert("string".to_string(), DataValue::String("texto".to_string()));
        row.insert("integer".to_string(), DataValue::Integer(42));
        row.insert("float".to_string(), DataValue::Float(3.14));
        row.insert("boolean".to_string(), DataValue::Boolean(true));
        row.insert("null".to_string(), DataValue::Null);
        
        let data = vec![row];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 1);
        assert_eq!(result.rows_successful, 1);
        // execution_time_ms pode ser 0 para operações muito rápidas
    }
}
