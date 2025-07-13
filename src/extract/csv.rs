use async_trait::async_trait;
use std::path::Path;
use crate::error::Result;
use crate::types::{DataRow, DataValue};
use crate::traits::Extractor;

/// Extrator para arquivos CSV
#[derive(Debug, Clone)]
pub struct CsvExtractor {
    file_path: String,
    delimiter: u8,
    has_headers: bool,
    quote_char: Option<u8>,
    escape_char: Option<u8>,
}

impl CsvExtractor {
    /// Cria um novo extrator CSV
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            delimiter: b',',
            has_headers: true,
            quote_char: Some(b'"'),
            escape_char: None,
        }
    }
    
    /// Define o delimitador
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }
    
    /// Define se tem cabeçalhos
    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }
    
    /// Define o caractere de aspas
    pub fn with_quote_char(mut self, quote_char: u8) -> Self {
        self.quote_char = Some(quote_char);
        self
    }
    
    /// Define o caractere de escape
    pub fn with_escape_char(mut self, escape_char: u8) -> Self {
        self.escape_char = Some(escape_char);
        self
    }
    
    /// Remove aspas
    pub fn without_quotes(mut self) -> Self {
        self.quote_char = None;
        self
    }
    
    /// Converte valor CSV para DataValue
    fn parse_value(&self, value: &str) -> DataValue {
        // Tenta parsear como inteiro
        if let Ok(int_val) = value.parse::<i64>() {
            return DataValue::Integer(int_val);
        }
        
        // Tenta parsear como float
        if let Ok(float_val) = value.parse::<f64>() {
            return DataValue::Float(float_val);
        }
        
        // Tenta parsear como boolean
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => return DataValue::Boolean(true),
            "false" | "0" | "no" | "n" => return DataValue::Boolean(false),
            _ => {}
        }
        
        // Verifica se é nulo
        if value.is_empty() || value.to_lowercase() == "null" {
            return DataValue::Null;
        }
        
        // Retorna como string
        DataValue::String(value.to_string())
    }
}

#[async_trait]
impl Extractor for CsvExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        use std::io::BufReader;
        use std::fs::File;
        
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        
        let mut csv_reader = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .quote(self.quote_char.unwrap_or(b'"'))
            .escape(self.escape_char)
            .from_reader(reader);
        
        let mut rows = Vec::new();
        
        if self.has_headers {
            let headers = csv_reader.headers()?.clone();
            
            for result in csv_reader.records() {
                let record = result?;
                let mut row = DataRow::new();
                
                for (i, field) in record.iter().enumerate() {
                    if let Some(header) = headers.get(i) {
                        row.insert(header.to_string(), self.parse_value(field));
                    }
                }
                
                rows.push(row);
            }
        } else {
            for (_line_number, result) in csv_reader.records().enumerate() {
                let record = result?;
                let mut row = DataRow::new();
                
                for (i, field) in record.iter().enumerate() {
                    row.insert(format!("column_{}", i), self.parse_value(field));
                }
                
                rows.push(row);
            }
        }
        
        Ok(rows)
    }
    
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        // Para esta implementação simples, vamos usar extract() e limitar o resultado
        // Em uma implementação real, você manteria o estado do leitor
        let all_data = self.extract().await?;
        Ok(all_data.into_iter().take(batch_size).collect())
    }
    
    async fn has_more(&self) -> Result<bool> {
        // Implementação simples - em uma implementação real, você manteria o estado
        Ok(false)
    }
    
    async fn reset(&mut self) -> Result<()> {
        // Implementação simples - em uma implementação real, você resetaria o leitor
        Ok(())
    }
}

/// Extrator CSV assíncrono com streaming
pub struct AsyncCsvExtractor {
    file_path: String,
    delimiter: u8,
    has_headers: bool,
    current_position: usize,
    batch_size: usize,
}

impl AsyncCsvExtractor {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            delimiter: b',',
            has_headers: true,
            current_position: 0,
            batch_size: 1000,
        }
    }
    
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

#[async_trait]
impl Extractor for AsyncCsvExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        // Implementação que lê todo o arquivo
        let basic_extractor = CsvExtractor::new(&self.file_path)
            .with_delimiter(self.delimiter)
            .with_headers(self.has_headers);
        
        basic_extractor.extract().await
    }
    
    async fn extract_batch(&self, batch_size: usize) -> Result<Vec<DataRow>> {
        // Implementação que lê apenas um lote
        // Em uma implementação real, você manteria o estado do arquivo
        let all_data = self.extract().await?;
        Ok(all_data.into_iter()
            .skip(self.current_position)
            .take(batch_size)
            .collect())
    }
    
    async fn has_more(&self) -> Result<bool> {
        // Implementação que verifica se há mais dados
        let total_rows = self.extract().await?.len();
        Ok(self.current_position < total_rows)
    }
    
    async fn reset(&mut self) -> Result<()> {
        self.current_position = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_csv_extractor() {
        // Cria arquivo CSV temporário
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name,age,active").unwrap();
        writeln!(temp_file, "Alice,30,true").unwrap();
        writeln!(temp_file, "Bob,25,false").unwrap();
        
        let extractor = CsvExtractor::new(temp_file.path());
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
        assert_eq!(result[0].get("age"), Some(&DataValue::Integer(30)));
        assert_eq!(result[0].get("active"), Some(&DataValue::Boolean(true)));
    }
    
    #[tokio::test]
    async fn test_csv_extractor_without_headers() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Alice,30,true").unwrap();
        writeln!(temp_file, "Bob,25,false").unwrap();
        
        let extractor = CsvExtractor::new(temp_file.path())
            .with_headers(false);
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("column_0"), Some(&DataValue::String("Alice".to_string())));
        assert_eq!(result[0].get("column_1"), Some(&DataValue::Integer(30)));
        assert_eq!(result[0].get("column_2"), Some(&DataValue::Boolean(true)));
    }
    
    #[tokio::test]
    async fn test_csv_extractor_custom_delimiter() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "name;age;active").unwrap();
        writeln!(temp_file, "Alice;30;true").unwrap();
        
        let extractor = CsvExtractor::new(temp_file.path())
            .with_delimiter(b';');
        let result = extractor.extract().await.unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
    }
}
