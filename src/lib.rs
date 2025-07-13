//! # ETLRS - Biblioteca ETL para Rust
//! 
//! Uma biblioteca de alta performance para pipelines ETL (Extract, Transform, Load)
//! construída em Rust com foco em segurança, extensibilidade e facilidade de uso.
//! 
//! ## Características Principais
//! 
//! - 🚄 **Alta Performance**: Processamento paralelo e assíncrono com Tokio
//! - 🔒 **Segurança**: Memory safety garantida pelo Rust
//! - 🔌 **Extensível**: Sistema de traits para adicionar novos conectores
//! - 📊 **Múltiplos Formatos**: CSV, JSON, Parquet, Avro, e mais
//! - ☁️ **Cloud Ready**: Suporte para AWS, Azure e GCP
//! - 🎯 **Type Safe**: Validação em tempo de compilação
//! 
//! ## Exemplo Rápido
//! 
//! ```rust,no_run
//! use etlrs::prelude::*;
//! 
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Pipeline simples: CSV → Filtro → JSON
//!     let pipeline = Pipeline::builder()
//!         .extract(CsvExtractor::new("users.csv"))
//!         .transform(FilterTransform::new(|row| {
//!             row.get("active") == Some(&DataValue::Boolean(true))
//!         }))
//!         .load(JsonLoader::new("output.json"))
//!         .build();
//!     
//!     pipeline.execute().await?;
//!     Ok(())
//! }
//! ```
//! 
//! ## Arquitetura
//! 
//! A biblioteca é estruturada em três componentes principais:
//! 
//! ### Extractors
//! Responsáveis por extrair dados de diferentes fontes como CSV, JSON e outros formatos.
//! 
//! ### Transformers
//! Aplicam transformações aos dados incluindo filtragem, mapeamento e agregações.
//! 
//! ### Loaders
//! Carregam dados para destinos como arquivos JSON, console ou memória.

pub mod config;
pub mod error;
pub mod traits;
pub mod types;
pub mod extract;
pub mod transform;
pub mod load;
pub mod pipeline;
pub mod events;
pub mod plugins;

// Re-exports para facilitar o uso
pub use config::ETLConfig;
pub use error::{ETLError, Result};
pub use types::{DataRow, DataValue, PipelineResult, PipelineState, PipelineEvent};
pub use traits::*;
pub use pipeline::Pipeline;
pub use events::{LoggingEventEmitter, InMemoryEventEmitter};
pub use plugins::{Plugin, PluginRegistry, PluginContext, LoggingPlugin, MetricsPlugin};

/// Prelude com imports mais comuns
pub mod prelude {
    pub use crate::config::ETLConfig;
    pub use crate::error::{ETLError, Result};
    pub use crate::types::{DataRow, DataValue, PipelineResult, PipelineState, PipelineEvent};
    pub use crate::traits::{Extractor, Transformer, Loader, EventEmitter};
    pub use crate::plugins::Plugin;
    pub use crate::pipeline::Pipeline;
    pub use crate::events::{LoggingEventEmitter, InMemoryEventEmitter};
    pub use crate::plugins::{PluginRegistry, PluginContext, LoggingPlugin, MetricsPlugin};
    
    // Extractors
    #[cfg(feature = "csv")]
    pub use crate::extract::csv::CsvExtractor;
    
    pub use crate::extract::json::{JsonExtractor, JsonLinesExtractor};
    
    #[cfg(feature = "delta")]
    pub use crate::extract::delta::DeltaExtractor;
    
    #[cfg(feature = "parquet")]
    pub use crate::extract::parquet::ParquetExtractor;
    
    // Transformers
    pub use crate::transform::common::{
        FilterTransform, MapTransform, AddColumnTransform, 
        RemoveColumnsTransform, RenameColumnsTransform, SelectColumnsTransform,
        ConvertTypesTransform, AggregateTransform, ParallelTransform, AsyncMapTransform,
        CompositeTransformer
    };
    
    // Loaders
    pub use crate::load::common::DataFormatter;
    pub use crate::load::console::ConsoleLoader;
    pub use crate::load::memory::MemoryLoader;
    pub use crate::load::json::{JsonLoader, JsonLinesLoader};
    
    #[cfg(feature = "delta")]
    pub use crate::load::delta::{DeltaLoader, DeltaWriteMode};
    
    #[cfg(feature = "parquet")]
    pub use crate::load::parquet::{ParquetLoader, CompressionType};
}

/// Informações sobre a versão da biblioteca
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Informações sobre a biblioteca
pub fn about() -> &'static str {
    env!("CARGO_PKG_DESCRIPTION")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
    
    #[test]
    fn test_about() {
        assert!(!about().is_empty());
    }
}
