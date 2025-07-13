//! # Parquet Extractor
//! 
//! Módulo para extração de dados de arquivos Apache Parquet.
//! Suporta leitura eficiente com projeção de colunas e processamento em batches.

use crate::{
    error::Result,
    traits::Extractor,
    types::{DataRow, DataValue},
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::{debug, info};

#[cfg(feature = "parquet")]
use {
    arrow::{
        array::*,
        datatypes::DataType as ArrowDataType,
        record_batch::RecordBatch,
    },
    parquet::{
        arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
        file::reader::{FileReader, SerializedFileReader},
    },
    std::fs::File,
};

/// Extractor para arquivos Apache Parquet
/// 
/// Suporta leitura eficiente de dados colunares com:
/// - Projeção de colunas para reduzir I/O
/// - Processamento em batches para grandes volumes
/// - Conversão automática de tipos Arrow para DataValue
/// - Acesso a metadados do arquivo
/// 
/// # Exemplos
/// 
/// ```rust
/// #[cfg(feature = "parquet")]
/// use etlrs::extract::ParquetExtractor;
/// 
/// #[cfg(feature = "parquet")]
/// async fn exemplo() -> Result<(), Box<dyn std::error::Error>> {
///     let extractor = ParquetExtractor::new("dados.parquet")?
///         .with_columns(vec!["nome".to_string(), "idade".to_string()])
///         .with_batch_size(5000);
///     
///     let dados = extractor.extract().await?;
///     println!("Extraídos {} registros", dados.len());
///     
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct ParquetExtractor {
    /// Caminho do arquivo Parquet
    file_path: PathBuf,
    /// Colunas a serem extraídas (None = todas)
    columns: Option<Vec<String>>,
    /// Tamanho do batch para leitura
    batch_size: usize,
    /// Metadados do arquivo em cache
    #[cfg(feature = "parquet")]
    cached_metadata: Option<ParquetMetadata>,
    #[cfg(not(feature = "parquet"))]
    cached_metadata: Option<()>,
}

impl ParquetExtractor {
    /// Cria um novo extractor Parquet
    /// 
    /// # Argumentos
    /// 
    /// * `file_path` - Caminho para o arquivo Parquet
    /// 
    /// # Erros
    /// 
    /// Retorna erro se o arquivo não existir ou não for um Parquet válido
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let path = file_path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(crate::error::ETLError::Extract(
                crate::error::ExtractError::FileNotFound(format!("Arquivo Parquet não encontrado: {:?}", path))
            ));
        }

        debug!("Criando ParquetExtractor para: {:?}", path);
        
        Ok(Self {
            file_path: path,
            columns: None,
            batch_size: 8192,
            cached_metadata: None,
        })
    }

    /// Define colunas específicas para extrair (projeção)
    pub fn with_columns<I, S>(mut self, columns: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = Some(columns.into_iter().map(|s| s.into()).collect());
        self
    }

    /// Define o tamanho do batch para leitura
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// Obtém metadados do arquivo Parquet
    #[cfg(feature = "parquet")]
    pub async fn get_metadata(&mut self) -> Result<&ParquetMetadata> {
        if self.cached_metadata.is_none() {
            let file = File::open(&self.file_path)
                .map_err(|e| crate::error::ETLError::Extract(
                    crate::error::ExtractError::FileNotFound(format!("Erro ao abrir arquivo: {}", e))
                ))?;

            let reader = SerializedFileReader::new(file)
                .map_err(|e| crate::error::ETLError::Extract(
                    crate::error::ExtractError::InvalidFormat(format!("Erro ao criar reader Parquet: {}", e))
                ))?;

            let metadata = reader.metadata();
            let file_metadata = metadata.file_metadata();
            
            let parquet_metadata = ParquetMetadata {
                num_rows: file_metadata.num_rows(),
                num_row_groups: metadata.num_row_groups() as i32,
                created_by: file_metadata.created_by().map(|s| s.to_string()),
                schema: format!("{:?}", metadata.file_metadata().schema_descr()),
            };

            self.cached_metadata = Some(parquet_metadata);
        }

        Ok(self.cached_metadata.as_ref().unwrap())
    }

    #[cfg(not(feature = "parquet"))]
    pub async fn get_metadata(&mut self) -> Result<&()> {
        Err(crate::error::ETLError::Config(
            crate::error::ConfigError::InvalidConfig("Feature 'parquet' requerida para ParquetExtractor".to_string())
        ))
    }

    /// Converte RecordBatch do Arrow para DataRows
    #[cfg(feature = "parquet")]
    fn convert_batch_to_rows(&self, batch: &RecordBatch) -> Result<Vec<DataRow>> {
        let mut rows = Vec::new();
        let num_rows = batch.num_rows();
        let schema = batch.schema();

        for row_index in 0..num_rows {
            let mut row = HashMap::new();
            
            for (col_index, field) in schema.fields().iter().enumerate() {
                let column = batch.column(col_index);
                let value = self.arrow_value_to_data_value(column, row_index)?;
                row.insert(field.name().clone(), value);
            }
            
            rows.push(row);
        }

        Ok(rows)
    }

    /// Converte valor Arrow para DataValue
    #[cfg(feature = "parquet")]
    fn arrow_value_to_data_value(&self, array: &dyn Array, index: usize) -> Result<DataValue> {
        if array.is_null(index) {
            return Ok(DataValue::Null);
        }

        match array.data_type() {
            ArrowDataType::Boolean => {
                let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                Ok(DataValue::Boolean(arr.value(index)))
            },
            ArrowDataType::Int8 => {
                let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::Int16 => {
                let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::Int32 => {
                let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::Int64 => {
                let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index)))
            },
            ArrowDataType::UInt8 => {
                let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::UInt16 => {
                let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::UInt32 => {
                let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::UInt64 => {
                let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                Ok(DataValue::Integer(arr.value(index) as i64))
            },
            ArrowDataType::Float32 => {
                let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
                Ok(DataValue::Float(arr.value(index) as f64))
            },
            ArrowDataType::Float64 => {
                let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
                Ok(DataValue::Float(arr.value(index)))
            },
            ArrowDataType::Utf8 => {
                let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
                Ok(DataValue::String(arr.value(index).to_string()))
            },
            ArrowDataType::LargeUtf8 => {
                let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                Ok(DataValue::String(arr.value(index).to_string()))
            },
            _ => {
                // Para tipos não suportados, converter para string
                Ok(DataValue::String(format!("{:?}", array)))
            }
        }
    }
}

#[async_trait]
impl Extractor for ParquetExtractor {
    #[cfg(feature = "parquet")]
    async fn extract(&self) -> Result<Vec<DataRow>> {
        info!("Iniciando extração Parquet de: {:?}", self.file_path);
        
        let file = File::open(&self.file_path)
            .map_err(|e| crate::error::ETLError::Extract(
                crate::error::ExtractError::FileNotFound(format!("Erro ao abrir arquivo Parquet: {}", e))
            ))?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| crate::error::ETLError::Extract(
                crate::error::ExtractError::InvalidFormat(format!("Erro ao criar builder Parquet: {}", e))
            ))?;

        let reader = builder.build()
            .map_err(|e| crate::error::ETLError::Extract(
                crate::error::ExtractError::InvalidFormat(format!("Erro ao construir leitor: {}", e))
            ))?;

        let mut all_data = Vec::new();

        for batch_result in reader {
            let batch = batch_result
                .map_err(|e| crate::error::ETLError::Extract(
                    crate::error::ExtractError::ParseError(format!("Erro ao ler batch: {}", e))
                ))?;

            let mut rows = self.convert_batch_to_rows(&batch)?;
            
            // Filtrar colunas se especificado
            if let Some(ref columns) = self.columns {
                rows = rows.into_iter().map(|mut row| {
                    let filtered_row: HashMap<String, DataValue> = columns.iter()
                        .filter_map(|col| {
                            row.remove(col).map(|value| (col.clone(), value))
                        })
                        .collect();
                    filtered_row
                }).collect();
            }
            
            all_data.extend(rows);
        }

        info!("Extração Parquet concluída: {} registros", all_data.len());
        Ok(all_data)
    }

    #[cfg(not(feature = "parquet"))]
    async fn extract(&self) -> Result<Vec<DataRow>> {
        Err(crate::error::ETLError::Config(
            crate::error::ConfigError::InvalidConfig("Feature 'parquet' requerida para ParquetExtractor".to_string())
        ))
    }
}

/// Metadados de um arquivo Parquet
#[derive(Debug, Clone)]
pub struct ParquetMetadata {
    /// Número total de linhas
    pub num_rows: i64,
    /// Número de row groups
    pub num_row_groups: i32,
    /// Informação sobre o criador do arquivo
    pub created_by: Option<String>,
    /// Schema do arquivo como string
    pub schema: String,
}

#[cfg(not(feature = "parquet"))]
impl ParquetMetadata {
    /// Cria metadados vazios quando feature não está habilitada
    pub fn empty() -> Self {
        Self {
            num_rows: 0,
            num_row_groups: 0,
            created_by: None,
            schema: "Feature 'parquet' não habilitada".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parquet_extractor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.parquet");
        
        // Criar arquivo vazio para teste
        std::fs::write(&file_path, b"").unwrap();
        
        let extractor = ParquetExtractor::new(&file_path).unwrap();
        assert_eq!(extractor.file_path, file_path);
        assert_eq!(extractor.batch_size, 8192);
    }

    #[tokio::test]
    async fn test_parquet_extractor_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.parquet");
        
        std::fs::write(&file_path, b"").unwrap();
        
        let extractor = ParquetExtractor::new(&file_path).unwrap()
            .with_columns(vec!["col1", "col2"])
            .with_batch_size(1000);

        assert!(extractor.columns.is_some());
        assert_eq!(extractor.batch_size, 1000);
    }

    #[tokio::test]
    async fn test_extract_without_feature() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.parquet");
        
        std::fs::write(&file_path, b"").unwrap();
        
        let extractor = ParquetExtractor::new(&file_path).unwrap();
        
        #[cfg(not(feature = "parquet"))]
        {
            let result = extractor.extract().await;
            assert!(result.is_err());
        }
    }
}
