//! # Parquet Loader
//! 
//! Módulo para carregamento de dados em arquivos Apache Parquet.
//! Otimizado para analytics com compressão eficiente e acesso colunar.

use crate::{
    error::Result,
    traits::Loader,
    types::{DataRow, DataValue, PipelineResult},
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use tracing::{debug, info, warn};

#[cfg(feature = "parquet")]
use {
    arrow::{
        array::*,
        datatypes::{DataType as ArrowDataType, Field, Schema},
        record_batch::RecordBatch,
    },
    parquet::{
        arrow::ArrowWriter,
        basic::{Compression, ZstdLevel, GzipLevel, BrotliLevel},
        file::properties::WriterProperties,
    },
    std::fs::File,
};

/// Tipo de compressão suportado
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionType {
    Uncompressed,
    Snappy,
    Gzip,
    Lzo,
    Brotli,
    Zstd,
}

/// Loader para arquivos Apache Parquet
#[derive(Debug)]
pub struct ParquetLoader {
    /// Caminho do arquivo de destino
    file_path: PathBuf,
    /// Tipo de compressão
    compression: CompressionType,
    /// Sobrescrever arquivo existente
    overwrite: bool,
    /// Tamanho do batch para escrita
    batch_size: usize,
    /// Metadados personalizados
    metadata: HashMap<String, String>,
}

impl ParquetLoader {
    /// Cria um novo loader Parquet
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let path = file_path.as_ref().to_path_buf();
        
        // Criar diretório pai se não existir
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| crate::error::ETLError::Load(
                        crate::error::LoadError::WriteError(format!("Erro ao criar diretório: {}", e))
                    ))?;
            }
        }

        debug!("Criando ParquetLoader para: {:?}", path);
        
        Ok(Self {
            file_path: path,
            compression: CompressionType::Snappy,
            overwrite: false,
            batch_size: 8192,
            metadata: HashMap::new(),
        })
    }

    /// Define o algoritmo de compressão
    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression = compression;
        self
    }

    /// Define se deve sobrescrever arquivo existente
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Define o tamanho do batch para escrita
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// Adiciona metadados personalizados
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self 
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Infere schema Arrow dos dados
    #[cfg(feature = "parquet")]
    fn infer_schema(&self, data: &[DataRow]) -> Result<Schema> {
        if data.is_empty() {
            return Err(crate::error::ETLError::Load(
                crate::error::LoadError::WriteError("Não é possível inferir schema de dados vazios".to_string())
            ));
        }

        let mut fields = Vec::new();
        let mut all_columns = std::collections::BTreeSet::new();

        // Coletar todas as colunas
        for row in data {
            all_columns.extend(row.keys().cloned());
        }

        // Inferir tipo para cada coluna
        for column in all_columns {
            let data_type = self.infer_column_type(data, &column);
            fields.push(Field::new(&column, data_type, true));
        }

        Ok(Schema::new(fields))
    }

    /// Infere o tipo de dados de uma coluna
    #[cfg(feature = "parquet")]
    fn infer_column_type(&self, data: &[DataRow], column: &str) -> ArrowDataType {
        // Examinar valores não-null para inferir tipo
        for row in data {
            if let Some(value) = row.get(column) {
                if !matches!(value, DataValue::Null) {
                    return self.data_value_to_arrow_type(value);
                }
            }
        }
        
        // Se todos são null, usar String como padrão
        ArrowDataType::Utf8
    }

    /// Converte DataValue para tipo Arrow
    #[cfg(feature = "parquet")]
    fn data_value_to_arrow_type(&self, value: &DataValue) -> ArrowDataType {
        match value {
            DataValue::String(_) => ArrowDataType::Utf8,
            DataValue::Integer(_) => ArrowDataType::Int64,
            DataValue::Float(_) => ArrowDataType::Float64,
            DataValue::Boolean(_) => ArrowDataType::Boolean,
            DataValue::Array(_) => ArrowDataType::List(Arc::new(Field::new(
                "item", ArrowDataType::Utf8, true
            ))),
            DataValue::Object(_) => ArrowDataType::Utf8, // Serializar como JSON
            DataValue::Date(_) => ArrowDataType::Date32,
            DataValue::DateTime(_) => ArrowDataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None),
            DataValue::Timestamp(_) => ArrowDataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, Some("UTC".into())),
            DataValue::Null => ArrowDataType::Utf8,
        }
    }

    /// Converte dados para RecordBatch
    #[cfg(feature = "parquet")]
    fn create_record_batch(&self, data: &[DataRow], schema: &Schema) -> Result<RecordBatch> {
        let mut builders: Vec<Box<dyn ArrayBuilder>> = Vec::new();
        
        // Criar builders para cada campo
        for field in schema.fields() {
            let builder = self.create_array_builder(field.data_type(), data.len())?;
            builders.push(builder);
        }

        // Processar cada linha
        for row in data {
            for (field_idx, field) in schema.fields().iter().enumerate() {
                let builder = &mut builders[field_idx];
                
                if let Some(value) = row.get(field.name()) {
                    self.append_value_to_builder(builder, value, field.data_type())?;
                } else {
                    // Valor ausente - adicionar null
                    self.append_null_to_builder(builder, field.data_type())?;
                }
            }
        }

        // Finalizar arrays
        let arrays: Result<Vec<Arc<dyn Array>>> = builders.into_iter()
            .map(|mut builder| -> Result<Arc<dyn Array>> {
                Ok(builder.finish())
            })
            .collect();

        RecordBatch::try_new(Arc::new(schema.clone()), arrays?)
            .map_err(|e| crate::error::ETLError::Load(
                crate::error::LoadError::WriteError(format!("Erro ao criar RecordBatch: {}", e))
            ))
    }

    /// Adiciona valor nulo ao builder
    #[cfg(feature = "parquet")]
    fn append_null_to_builder(&self, builder: &mut Box<dyn ArrayBuilder>, data_type: &ArrowDataType) -> Result<()> {
        match data_type {
            ArrowDataType::Utf8 => {
                let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                builder.append_null();
            },
            ArrowDataType::Int64 => {
                let builder = builder.as_any_mut().downcast_mut::<Int64Builder>().unwrap();
                builder.append_null();
            },
            ArrowDataType::Float64 => {
                let builder = builder.as_any_mut().downcast_mut::<Float64Builder>().unwrap();
                builder.append_null();
            },
            ArrowDataType::Boolean => {
                let builder = builder.as_any_mut().downcast_mut::<BooleanBuilder>().unwrap();
                builder.append_null();
            },
            _ => {
                // Para outros tipos, tratar como string
                let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                builder.append_null();
            }
        }
        Ok(())
    }

    /// Adiciona valor ao builder apropriado
    #[cfg(feature = "parquet")]
    fn append_value_to_builder(&self, builder: &mut Box<dyn ArrayBuilder>, value: &DataValue, data_type: &ArrowDataType) -> Result<()> {
        match (value, data_type) {
            (DataValue::Null, _) => {
                self.append_null_to_builder(builder, data_type)?;
            },
            (DataValue::String(s), ArrowDataType::Utf8) => {
                let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                builder.append_value(s);
            },
            (DataValue::Integer(i), ArrowDataType::Int64) => {
                let builder = builder.as_any_mut().downcast_mut::<Int64Builder>().unwrap();
                builder.append_value(*i);
            },
            (DataValue::Float(f), ArrowDataType::Float64) => {
                let builder = builder.as_any_mut().downcast_mut::<Float64Builder>().unwrap();
                builder.append_value(*f);
            },
            (DataValue::Boolean(b), ArrowDataType::Boolean) => {
                let builder = builder.as_any_mut().downcast_mut::<BooleanBuilder>().unwrap();
                builder.append_value(*b);
            },
            (DataValue::Array(arr), ArrowDataType::List(_)) => {
                // Serializar array como JSON string
                let json_str = serde_json::to_string(arr)
                    .map_err(|e| crate::error::ETLError::Serialization(e))?;
                let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                builder.append_value(&json_str);
            },
            (DataValue::Object(obj), ArrowDataType::Utf8) => {
                // Serializar objeto como JSON string
                let json_str = serde_json::to_string(obj)
                    .map_err(|e| crate::error::ETLError::Serialization(e))?;
                let builder = builder.as_any_mut().downcast_mut::<StringBuilder>().unwrap();
                builder.append_value(&json_str);
            },
            _ => {
                return Err(crate::error::ETLError::Load(
                    crate::error::LoadError::WriteError(format!("Conversão não suportada: {:?} para {:?}", value, data_type))
                ));
            }
        }
        Ok(())
    }

    /// Cria array builder apropriado
    #[cfg(feature = "parquet")]
    fn create_array_builder(&self, data_type: &ArrowDataType, capacity: usize) -> Result<Box<dyn ArrayBuilder>> {
        match data_type {
            ArrowDataType::Utf8 => Ok(Box::new(StringBuilder::with_capacity(capacity, capacity * 10))),
            ArrowDataType::Int64 => Ok(Box::new(Int64Builder::with_capacity(capacity))),
            ArrowDataType::Float64 => Ok(Box::new(Float64Builder::with_capacity(capacity))),
            ArrowDataType::Boolean => Ok(Box::new(BooleanBuilder::with_capacity(capacity))),
            ArrowDataType::List(_) => Ok(Box::new(StringBuilder::with_capacity(capacity, capacity * 50))),
            _ => {
                return Err(crate::error::ETLError::Load(
                    crate::error::LoadError::WriteError(format!("Tipo de array não suportado: {:?}", data_type))
                ));
            }
        }
    }

    /// Converte tipo de compressão para Parquet
    #[cfg(feature = "parquet")]
    fn get_compression(&self) -> Compression {
        match self.compression {
            CompressionType::Uncompressed => Compression::UNCOMPRESSED,
            CompressionType::Snappy => Compression::SNAPPY,
            CompressionType::Gzip => Compression::GZIP(GzipLevel::default()),
            CompressionType::Lzo => Compression::LZO,
            CompressionType::Brotli => Compression::BROTLI(BrotliLevel::default()),
            CompressionType::Zstd => Compression::ZSTD(ZstdLevel::default()),
        }
    }
}

#[async_trait]
impl Loader for ParquetLoader {
    #[cfg(feature = "parquet")]
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let start_time = Instant::now();
        
        if data.is_empty() {
            warn!("Dados vazios recebidos para carregamento Parquet");
            return Ok(PipelineResult {
                rows_processed: 0,
                rows_successful: 0,
                rows_failed: 0,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                errors: Vec::new(),
            });
        }

        info!("Iniciando carregamento Parquet de {} registros para: {:?}", 
              data.len(), self.file_path);

        // Verificar se arquivo existe e overwrite
        if self.file_path.exists() && !self.overwrite {
            return Err(crate::error::ETLError::Load(
                crate::error::LoadError::DataConflict(format!("Arquivo já existe: {:?}", self.file_path))
            ));
        }

        // Inferir schema
        let schema = self.infer_schema(&data)?;
        debug!("Schema inferido: {:?}", schema);

        // Converter para RecordBatch
        let record_batch = self.create_record_batch(&data, &schema)?;

        // Configurar propriedades do writer
        let props = WriterProperties::builder()
            .set_compression(self.get_compression())
            .build();

        // Criar e escrever arquivo
        let file = File::create(&self.file_path)
            .map_err(|e| crate::error::ETLError::Load(
                crate::error::LoadError::WriteError(format!("Erro ao criar arquivo Parquet: {}", e))
            ))?;

        let mut writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props))
            .map_err(|e| crate::error::ETLError::Load(
                crate::error::LoadError::WriteError(format!("Erro ao criar ArrowWriter: {}", e))
            ))?;

        writer.write(&record_batch)
            .map_err(|e| crate::error::ETLError::Load(
                crate::error::LoadError::WriteError(format!("Erro ao escrever RecordBatch: {}", e))
            ))?;

        writer.close()
            .map_err(|e| crate::error::ETLError::Load(
                crate::error::LoadError::WriteError(format!("Erro ao finalizar arquivo Parquet: {}", e))
            ))?;

        let duration = start_time.elapsed();
        info!("Carregamento Parquet concluído: {} registros em {:?}", 
              data.len(), duration);

        Ok(PipelineResult {
            rows_processed: data.len(),
            rows_successful: data.len(),
            rows_failed: 0,
            execution_time_ms: duration.as_millis() as u64,
            errors: Vec::new(),
        })
    }

    #[cfg(not(feature = "parquet"))]
    async fn load(&self, _data: Vec<DataRow>) -> Result<PipelineResult> {
        Err(crate::error::ETLError::Config(
            crate::error::ConfigError::InvalidConfig("Feature 'parquet' requerida para ParquetLoader".to_string())
        ))
    }
}
