//! # JSON Loader
//! 
//! Módulo para carregamento de dados em formato JSON.
//! Suporta formatação pretty-print e modo append.

use async_trait::async_trait;
use std::path::Path;
use crate::error::Result;
use crate::types::{DataRow, DataValue, PipelineResult};
use crate::traits::Loader;

/// Carregador para arquivos JSON
/// 
/// Permite salvar dados em formato JSON com opções de:
/// - Pretty-print para melhor legibilidade
/// - Modo append para adicionar dados a arquivos existentes
/// - Conversão automática de tipos DataValue para JSON
/// 
/// # Exemplos
/// 
/// ```rust
/// use etlrs::load::json::JsonLoader;
/// 
/// async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
///     let loader = JsonLoader::new("output.json")
///         .with_pretty(true)
///         .with_append(false);
///     
///     // dados seria Vec<DataRow>
///     let dados = vec![];
///     let resultado = loader.load(dados).await?;
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct JsonLoader {
    file_path: String,
    pretty: bool,
    append: bool,
}

impl JsonLoader {
    /// Cria um novo JsonLoader
    /// 
    /// # Argumentos
    /// 
    /// * `file_path` - Caminho do arquivo JSON de destino
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            pretty: false,
            append: false,
        }
    }
    
    /// Define se deve usar formatação pretty-print
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
    
    /// Define se deve adicionar dados ao arquivo existente
    pub fn with_append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }
    
    /// Converte DataValue para serde_json::Value
    fn data_value_to_json(&self, value: &DataValue) -> serde_json::Value {
        match value {
            DataValue::String(s) => serde_json::Value::String(s.clone()),
            DataValue::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            DataValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DataValue::Boolean(b) => serde_json::Value::Bool(*b),
            DataValue::Null => serde_json::Value::Null,
            DataValue::Array(arr) => {
                let values: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.data_value_to_json(v))
                    .collect();
                serde_json::Value::Array(values)
            }
            DataValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (key, value) in obj {
                    map.insert(key.clone(), self.data_value_to_json(value));
                }
                serde_json::Value::Object(map)
            }
            DataValue::Date(date) => {
                serde_json::Value::String(date.format("%Y-%m-%d").to_string())
            }
            DataValue::DateTime(dt) => {
                serde_json::Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string())
            }
            DataValue::Timestamp(ts) => {
                serde_json::Value::String(ts.to_rfc3339())
            }
        }
    }
    
    /// Converte DataRow para serde_json::Value
    fn row_to_json(&self, row: &DataRow) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (key, value) in row {
            map.insert(key.clone(), self.data_value_to_json(value));
        }
        serde_json::Value::Object(map)
    }
}

#[async_trait]
impl Loader for JsonLoader {
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();
        let mut result = PipelineResult::new();
        
        // Converte os dados para JSON
        let json_values: Vec<serde_json::Value> = data.iter()
            .map(|row| self.row_to_json(row))
            .collect();
        
        let json_array = serde_json::Value::Array(json_values);
        
        // Serializa para string
        let json_string = if self.pretty {
            serde_json::to_string_pretty(&json_array)?
        } else {
            serde_json::to_string(&json_array)?
        };
        
        // Escreve no arquivo
        if self.append {
            // Para append, precisamos ler o arquivo existente e adicionar os dados
            let existing_content = match tokio::fs::read_to_string(&self.file_path).await {
                Ok(content) => content,
                Err(_) => "[]".to_string(), // Se não existe, começa com array vazio
            };
            
            let mut existing_array: serde_json::Value = serde_json::from_str(&existing_content)?;
            
            if let serde_json::Value::Array(ref mut existing_vec) = existing_array {
                if let serde_json::Value::Array(new_vec) = json_array {
                    existing_vec.extend(new_vec);
                }
            }
            
            let final_json = if self.pretty {
                serde_json::to_string_pretty(&existing_array)?
            } else {
                serde_json::to_string(&existing_array)?
            };
            
            tokio::fs::write(&self.file_path, final_json).await?;
        } else {
            tokio::fs::write(&self.file_path, json_string).await?;
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
        // Para arquivos JSON, não há finalização necessária
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Verifica se o diretório pai existe e é gravável
        if let Some(parent) = Path::new(&self.file_path).parent() {
            Ok(parent.exists() && parent.is_dir())
        } else {
            Ok(true)
        }
    }
}

/// Carregador para arquivos JSON Lines (JSONL)
/// 
/// JSON Lines é um formato conveniente para dados estruturados que podem ser
/// processados uma linha por vez. Cada linha é um objeto JSON válido separadamente.
/// 
/// # Exemplos
/// 
/// ```rust
/// use etlrs::load::json::JsonLinesLoader;
/// 
/// async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
///     let loader = JsonLinesLoader::new("output.jsonl")
///         .with_append(true);
///     
///     // dados seria Vec<DataRow>
///     let dados = vec![];
///     let resultado = loader.load(dados).await?;
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct JsonLinesLoader {
    file_path: String,
    append: bool,
}

impl JsonLinesLoader {
    /// Cria um novo JsonLinesLoader
    /// 
    /// # Argumentos
    /// 
    /// * `file_path` - Caminho do arquivo JSONL de destino
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            append: false,
        }
    }
    
    /// Define se deve adicionar ao final do arquivo existente
    pub fn with_append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }
    
    /// Converte DataValue para serde_json::Value
    fn data_value_to_json(&self, value: &DataValue) -> serde_json::Value {
        match value {
            DataValue::String(s) => serde_json::Value::String(s.clone()),
            DataValue::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            DataValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            DataValue::Boolean(b) => serde_json::Value::Bool(*b),
            DataValue::Null => serde_json::Value::Null,
            DataValue::Array(arr) => {
                let values: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.data_value_to_json(v))
                    .collect();
                serde_json::Value::Array(values)
            }
            DataValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (key, value) in obj {
                    map.insert(key.clone(), self.data_value_to_json(value));
                }
                serde_json::Value::Object(map)
            }
            DataValue::Date(date) => {
                serde_json::Value::String(date.format("%Y-%m-%d").to_string())
            }
            DataValue::DateTime(dt) => {
                serde_json::Value::String(dt.format("%Y-%m-%d %H:%M:%S").to_string())
            }
            DataValue::Timestamp(ts) => {
                serde_json::Value::String(ts.to_rfc3339())
            }
        }
    }
    
    /// Converte DataRow para serde_json::Value
    fn row_to_json(&self, row: &DataRow) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (key, value) in row {
            map.insert(key.clone(), self.data_value_to_json(value));
        }
        serde_json::Value::Object(map)
    }
}

#[async_trait]
impl Loader for JsonLinesLoader {
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let start_time = std::time::Instant::now();
        let mut result = PipelineResult::new();
        
        // Converte cada linha para JSON
        let json_lines: Vec<String> = data.iter()
            .map(|row| self.row_to_json(row))
            .map(|json| serde_json::to_string(&json))
            .collect::<std::result::Result<Vec<String>, serde_json::Error>>()?;
        
        let content = json_lines.join("\n");
        
        // Escreve no arquivo
        if self.append {
            // Para append, anexa ao arquivo existente
            let mut file_content = String::new();
            if Path::new(&self.file_path).exists() {
                let existing = tokio::fs::read_to_string(&self.file_path).await?;
                file_content.push_str(&existing);
                if !existing.is_empty() && !existing.ends_with('\n') {
                    file_content.push('\n');
                }
            }
            file_content.push_str(&content);
            
            tokio::fs::write(&self.file_path, file_content).await?;
        } else {
            tokio::fs::write(&self.file_path, content).await?;
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
        if let Some(parent) = Path::new(&self.file_path).parent() {
            Ok(parent.exists() && parent.is_dir())
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_json_loader() {
        let temp_file = NamedTempFile::new().unwrap();
        let loader = JsonLoader::new(temp_file.path()).with_pretty(true);
        
        let mut row1 = DataRow::new();
        row1.insert("name".to_string(), DataValue::String("Alice".to_string()));
        row1.insert("age".to_string(), DataValue::Integer(30));
        
        let mut row2 = DataRow::new();
        row2.insert("name".to_string(), DataValue::String("Bob".to_string()));
        row2.insert("age".to_string(), DataValue::Integer(25));
        
        let data = vec![row1, row2];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 2);
        assert_eq!(result.rows_successful, 2);
        
        // Verifica se o arquivo foi criado
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));
    }
    
    #[tokio::test]
    async fn test_json_loader_append() {
        let temp_file = NamedTempFile::new().unwrap();
        
        // Primeiro carregamento
        let loader = JsonLoader::new(temp_file.path()).with_append(false);
        let mut row1 = DataRow::new();
        row1.insert("name".to_string(), DataValue::String("Alice".to_string()));
        let data1 = vec![row1];
        loader.load(data1).await.unwrap();
        
        // Segundo carregamento com append
        let loader = JsonLoader::new(temp_file.path()).with_append(true);
        let mut row2 = DataRow::new();
        row2.insert("name".to_string(), DataValue::String("Bob".to_string()));
        let data2 = vec![row2];
        loader.load(data2).await.unwrap();
        
        // Verifica se ambos os dados estão no arquivo
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));
        
        // Verifica se é um array JSON válido
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        if let serde_json::Value::Array(arr) = parsed {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("Conteúdo não é um array JSON");
        }
    }
}

#[cfg(test)]
mod jsonl_tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_jsonl_loader() {
        let temp_file = NamedTempFile::new().unwrap();
        let loader = JsonLinesLoader::new(temp_file.path());
        
        let mut row1 = DataRow::new();
        row1.insert("name".to_string(), DataValue::String("Alice".to_string()));
        row1.insert("age".to_string(), DataValue::Integer(30));
        
        let mut row2 = DataRow::new();
        row2.insert("name".to_string(), DataValue::String("Bob".to_string()));
        row2.insert("age".to_string(), DataValue::Integer(25));
        
        let data = vec![row1, row2];
        let result = loader.load(data).await.unwrap();
        
        assert_eq!(result.rows_processed, 2);
        
        // Verifica se o arquivo foi criado com formato JSONL
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Alice"));
        assert!(lines[1].contains("Bob"));
        
        // Verifica se cada linha é um JSON válido
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object());
        }
    }
    
    #[tokio::test]
    async fn test_jsonl_append_mode() {
        let temp_file = NamedTempFile::new().unwrap();
        let loader = JsonLinesLoader::new(temp_file.path()).with_append(true);
        
        // Primeiro carregamento
        let mut row1 = DataRow::new();
        row1.insert("id".to_string(), DataValue::Integer(1));
        let result1 = loader.load(vec![row1]).await.unwrap();
        assert_eq!(result1.rows_processed, 1);
        
        // Segundo carregamento (append)
        let mut row2 = DataRow::new();
        row2.insert("id".to_string(), DataValue::Integer(2));
        let result2 = loader.load(vec![row2]).await.unwrap();
        assert_eq!(result2.rows_processed, 1);
        
        // Verifica se ambos estão no arquivo
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"id\":1"));
        assert!(lines[1].contains("\"id\":2"));
    }
}
