use async_trait::async_trait;
use std::path::Path;
use crate::error::Result;
use crate::types::{DataRow, DataValue};
use crate::traits::Extractor;

/// Extrator para arquivos JSON
#[derive(Debug, Clone)]
pub struct JsonExtractor {
    file_path: String,
    array_path: Option<String>,
}

impl JsonExtractor {
    /// Cria um novo extrator JSON
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            array_path: None,
        }
    }
    
    /// Define o caminho do array no JSON (para JSONs aninhados)
    pub fn with_array_path(mut self, path: impl Into<String>) -> Self {
        self.array_path = Some(path.into());
        self
    }
    
    /// Converte serde_json::Value para DataValue
    fn json_to_data_value(&self, value: &serde_json::Value) -> DataValue {
        match value {
            serde_json::Value::String(s) => DataValue::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    DataValue::Float(f)
                } else {
                    DataValue::String(n.to_string())
                }
            }
            serde_json::Value::Bool(b) => DataValue::Boolean(*b),
            serde_json::Value::Null => DataValue::Null,
            serde_json::Value::Array(arr) => {
                let values: Vec<DataValue> = arr.iter()
                    .map(|v| self.json_to_data_value(v))
                    .collect();
                DataValue::Array(values)
            }
            serde_json::Value::Object(obj) => {
                let mut map = std::collections::HashMap::new();
                for (key, value) in obj {
                    map.insert(key.clone(), self.json_to_data_value(value));
                }
                DataValue::Object(map)
            }
        }
    }
    
    /// Converte um objeto JSON para DataRow
    fn json_object_to_row(&self, obj: &serde_json::Map<String, serde_json::Value>) -> DataRow {
        let mut row = DataRow::new();
        for (key, value) in obj {
            row.insert(key.clone(), self.json_to_data_value(value));
        }
        row
    }
    
    /// Extrai dados de um caminho específico no JSON
    fn extract_from_path<'a>(&self, json: &'a serde_json::Value, path: &str) -> Result<&'a serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;
        
        for part in parts {
            match current {
                serde_json::Value::Object(obj) => {
                    current = obj.get(part).ok_or_else(|| {
                        crate::error::ETLError::Extract(
                            crate::error::ExtractError::ParseError(
                                format!("Caminho '{}' não encontrado no JSON", part)
                            )
                        )
                    })?;
                }
                _ => {
                    return Err(crate::error::ETLError::Extract(
                        crate::error::ExtractError::ParseError(
                            format!("Caminho '{}' não é um objeto", part)
                        )
                    ));
                }
            }
        }
        
        Ok(current)
    }
}

#[async_trait]
impl Extractor for JsonExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        let content = tokio::fs::read_to_string(&self.file_path).await?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        
        let target_json = if let Some(path) = &self.array_path {
            self.extract_from_path(&json, path)?
        } else {
            &json
        };
        
        let mut rows = Vec::new();
        
        match target_json {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    match item {
                        serde_json::Value::Object(obj) => {
                            rows.push(self.json_object_to_row(obj));
                        }
                        _ => {
                            // Se o item não é um objeto, cria um row com um campo "value"
                            let mut row = DataRow::new();
                            row.insert("value".to_string(), self.json_to_data_value(item));
                            rows.push(row);
                        }
                    }
                }
            }
            serde_json::Value::Object(obj) => {
                // Se é um objeto único, trata como uma linha
                rows.push(self.json_object_to_row(obj));
            }
            _ => {
                // Se é um valor primitivo, cria um row com um campo "value"
                let mut row = DataRow::new();
                row.insert("value".to_string(), self.json_to_data_value(target_json));
                rows.push(row);
            }
        }
        
        Ok(rows)
    }
    
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        let all_data = self.extract().await?;
        Ok(all_data.into_iter().take(batch_size).collect())
    }
    
    async fn has_more(&self) -> Result<bool> {
        Ok(false)
    }
    
    async fn reset(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Extrator para JSON Lines (JSONL)
#[derive(Debug, Clone)]
pub struct JsonLinesExtractor {
    file_path: String,
    current_line: usize,
}

impl JsonLinesExtractor {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            current_line: 0,
        }
    }
    
    /// Converte serde_json::Value para DataValue
    fn json_to_data_value(&self, value: &serde_json::Value) -> DataValue {
        match value {
            serde_json::Value::String(s) => DataValue::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    DataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    DataValue::Float(f)
                } else {
                    DataValue::String(n.to_string())
                }
            }
            serde_json::Value::Bool(b) => DataValue::Boolean(*b),
            serde_json::Value::Null => DataValue::Null,
            serde_json::Value::Array(arr) => {
                let values: Vec<DataValue> = arr.iter()
                    .map(|v| self.json_to_data_value(v))
                    .collect();
                DataValue::Array(values)
            }
            serde_json::Value::Object(obj) => {
                let mut map = std::collections::HashMap::new();
                for (key, value) in obj {
                    map.insert(key.clone(), self.json_to_data_value(value));
                }
                DataValue::Object(map)
            }
        }
    }
}

#[async_trait]
impl Extractor for JsonLinesExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        let content = tokio::fs::read_to_string(&self.file_path).await?;
        let mut rows = Vec::new();
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            let json: serde_json::Value = serde_json::from_str(line)?;
            
            match json {
                serde_json::Value::Object(obj) => {
                    let mut row = DataRow::new();
                    for (key, value) in obj {
                        row.insert(key, self.json_to_data_value(&value));
                    }
                    rows.push(row);
                }
                _ => {
                    let mut row = DataRow::new();
                    row.insert("value".to_string(), self.json_to_data_value(&json));
                    rows.push(row);
                }
            }
        }
        
        Ok(rows)
    }
    
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        let all_data = self.extract().await?;
        Ok(all_data.into_iter()
            .skip(self.current_line)
            .take(batch_size)
            .collect())
    }
    
    async fn has_more(&self) -> Result<bool> {
        let total_lines = self.extract().await?.len();
        Ok(self.current_line < total_lines)
    }
    
    async fn reset(&mut self) -> Result<()> {
        self.current_line = 0;
        Ok(())
    }
}

/// Extrator para JSON streaming
pub struct JsonStreamExtractor {
    file_path: String,
    buffer_size: usize,
}

impl JsonStreamExtractor {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            buffer_size: 8192,
        }
    }
    
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }
}

#[async_trait]
impl Extractor for JsonStreamExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        // Para esta implementação, vamos usar o extrator básico
        let basic_extractor = JsonExtractor::new(&self.file_path);
        basic_extractor.extract().await
    }
    
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        let all_data = self.extract().await?;
        Ok(all_data.into_iter().take(batch_size).collect())
    }
    
    async fn has_more(&self) -> Result<bool> {
        Ok(false)
    }
    
    async fn reset(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_json_extractor_array() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"[
            {{"name": "Alice", "age": 30, "active": true}},
            {{"name": "Bob", "age": 25, "active": false}}
        ]"#).unwrap();
        
        let extractor = JsonExtractor::new(temp_file.path());
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
        assert_eq!(result[0].get("age"), Some(&DataValue::Integer(30)));
        assert_eq!(result[0].get("active"), Some(&DataValue::Boolean(true)));
    }
    
    #[tokio::test]
    async fn test_json_extractor_single_object() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"name": "Alice", "age": 30, "active": true}}"#).unwrap();
        
        let extractor = JsonExtractor::new(temp_file.path());
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
    }
    
    #[tokio::test]
    async fn test_json_extractor_nested_path() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{
            "users": [
                {{"name": "Alice", "age": 30}},
                {{"name": "Bob", "age": 25}}
            ]
        }}"#).unwrap();
        
        let extractor = JsonExtractor::new(temp_file.path())
            .with_array_path("users");
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
    }
    
    #[tokio::test]
    async fn test_jsonl_extractor() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"{{"name": "Alice", "age": 30}}"#).unwrap();
        writeln!(temp_file, r#"{{"name": "Bob", "age": 25}}"#).unwrap();
        
        let extractor = JsonLinesExtractor::new(temp_file.path());
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
        assert_eq!(result[1].get("name"), Some(&DataValue::String("Bob".to_string())));
    }
}
