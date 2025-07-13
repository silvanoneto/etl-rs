use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use crate::error::Result;
use crate::types::{DataRow, DataValue};
use crate::traits::Transformer;

/// Transformador que filtra linhas baseado em uma condição
#[derive(Clone)]
pub struct FilterTransform<F> {
    filter_fn: F,
}

impl<F> FilterTransform<F>
where
    F: Fn(&DataRow) -> bool + Send + Sync + Clone,
{
    pub fn new(filter_fn: F) -> Self {
        Self { filter_fn }
    }
}

#[async_trait]
impl<F> Transformer for FilterTransform<F>
where
    F: Fn(&DataRow) -> bool + Send + Sync + Clone,
{
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .filter(|row| (self.filter_fn)(row))
            .collect())
    }
}

/// Transformador que mapeia cada linha usando uma função
#[derive(Clone)]
pub struct MapTransform<F> {
    map_fn: F,
}

impl<F> MapTransform<F>
where
    F: Fn(DataRow) -> DataRow + Send + Sync + Clone,
{
    pub fn new(map_fn: F) -> Self {
        Self { map_fn }
    }
}

#[async_trait]
impl<F> Transformer for MapTransform<F>
where
    F: Fn(DataRow) -> DataRow + Send + Sync + Clone,
{
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .map(|row| (self.map_fn)(row))
            .collect())
    }
}

/// Transformador assíncrono que mapeia cada linha
pub struct AsyncMapTransform<F> {
    map_fn: F,
}

impl<F> AsyncMapTransform<F>
where
    F: Fn(DataRow) -> Pin<Box<dyn Future<Output = DataRow> + Send>> + Send + Sync,
{
    pub fn new(map_fn: F) -> Self {
        Self { map_fn }
    }
}

#[async_trait]
impl<F> Transformer for AsyncMapTransform<F>
where
    F: Fn(DataRow) -> Pin<Box<dyn Future<Output = DataRow> + Send>> + Send + Sync,
{
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        let futures: Vec<_> = data.into_iter()
            .map(|row| (self.map_fn)(row))
            .collect();
        
        Ok(futures::future::join_all(futures).await)
    }
}

/// Transformador que adiciona uma coluna com valor constante
#[derive(Debug, Clone)]
pub struct AddColumnTransform {
    column_name: String,
    value: DataValue,
}

impl AddColumnTransform {
    pub fn new(column_name: impl Into<String>, value: DataValue) -> Self {
        Self {
            column_name: column_name.into(),
            value,
        }
    }
}

#[async_trait]
impl Transformer for AddColumnTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .map(|mut row| {
                row.insert(self.column_name.clone(), self.value.clone());
                row
            })
            .collect())
    }
}

/// Transformador que remove colunas específicas
#[derive(Debug, Clone)]
pub struct RemoveColumnsTransform {
    columns: Vec<String>,
}

impl RemoveColumnsTransform {
    pub fn new(columns: Vec<String>) -> Self {
        Self { columns }
    }
    
    pub fn single(column: impl Into<String>) -> Self {
        Self {
            columns: vec![column.into()],
        }
    }
}

#[async_trait]
impl Transformer for RemoveColumnsTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .map(|mut row| {
                for column in &self.columns {
                    row.remove(column);
                }
                row
            })
            .collect())
    }
}

/// Transformador que renomeia colunas
#[derive(Debug, Clone)]
pub struct RenameColumnsTransform {
    mappings: std::collections::HashMap<String, String>,
}

impl RenameColumnsTransform {
    pub fn new(mappings: std::collections::HashMap<String, String>) -> Self {
        Self { mappings }
    }
    
    pub fn single(old_name: impl Into<String>, new_name: impl Into<String>) -> Self {
        let mut mappings = std::collections::HashMap::new();
        mappings.insert(old_name.into(), new_name.into());
        Self { mappings }
    }
}

#[async_trait]
impl Transformer for RenameColumnsTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .map(|row| {
                let mut new_row = DataRow::new();
                for (key, value) in row {
                    let new_key = self.mappings.get(&key).cloned().unwrap_or(key);
                    new_row.insert(new_key, value);
                }
                new_row
            })
            .collect())
    }
}

/// Transformador que converte tipos de dados
#[derive(Debug, Clone)]
pub struct ConvertTypesTransform {
    conversions: std::collections::HashMap<String, DataType>,
}

#[derive(Debug, Clone)]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Timestamp,
}

impl ConvertTypesTransform {
    pub fn new(conversions: std::collections::HashMap<String, DataType>) -> Self {
        Self { conversions }
    }
    
    pub fn single(column: impl Into<String>, data_type: DataType) -> Self {
        let mut conversions = std::collections::HashMap::new();
        conversions.insert(column.into(), data_type);
        Self { conversions }
    }
    
    fn convert_value(&self, value: &DataValue, target_type: &DataType) -> DataValue {
        match target_type {
            DataType::String => {
                value.as_string().map(DataValue::String).unwrap_or(DataValue::Null)
            }
            DataType::Integer => {
                value.as_integer().map(DataValue::Integer).unwrap_or(DataValue::Null)
            }
            DataType::Float => {
                value.as_float().map(DataValue::Float).unwrap_or(DataValue::Null)
            }
            DataType::Boolean => {
                value.as_boolean().map(DataValue::Boolean).unwrap_or(DataValue::Null)
            }
            DataType::Date => {
                value.as_date().map(DataValue::Date).unwrap_or(DataValue::Null)
            }
            DataType::DateTime => {
                value.as_datetime().map(DataValue::DateTime).unwrap_or(DataValue::Null)
            }
            DataType::Timestamp => {
                value.as_timestamp().map(DataValue::Timestamp).unwrap_or(DataValue::Null)
            }
        }
    }
}

#[async_trait]
impl Transformer for ConvertTypesTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter()
            .map(|mut row| {
                for (column, target_type) in &self.conversions {
                    if let Some(value) = row.get(column) {
                        let converted = self.convert_value(value, target_type);
                        row.insert(column.clone(), converted);
                    }
                }
                row
            })
            .collect())
    }
}

/// Transformador que agrega dados
#[derive(Debug, Clone)]
pub struct AggregateTransform {
    group_by: Vec<String>,
    aggregations: std::collections::HashMap<String, AggregateFunction>,
}

#[derive(Debug, Clone)]
pub enum AggregateFunction {
    Count,
    Sum,
    Average,
    Min,
    Max,
    First,
    Last,
}

impl AggregateTransform {
    pub fn new(
        group_by: Vec<String>,
        aggregations: std::collections::HashMap<String, AggregateFunction>,
    ) -> Self {
        Self {
            group_by,
            aggregations,
        }
    }
    
    fn apply_aggregation(&self, values: &[DataValue], func: &AggregateFunction) -> DataValue {
        match func {
            AggregateFunction::Count => DataValue::Integer(values.len() as i64),
            AggregateFunction::Sum => {
                let sum: f64 = values.iter()
                    .filter_map(|v| v.as_float())
                    .sum();
                DataValue::Float(sum)
            }
            AggregateFunction::Average => {
                let values: Vec<f64> = values.iter()
                    .filter_map(|v| v.as_float())
                    .collect();
                if values.is_empty() {
                    DataValue::Null
                } else {
                    DataValue::Float(values.iter().sum::<f64>() / values.len() as f64)
                }
            }
            AggregateFunction::Min => {
                values.iter()
                    .filter_map(|v| v.as_float())
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(DataValue::Float)
                    .unwrap_or(DataValue::Null)
            }
            AggregateFunction::Max => {
                values.iter()
                    .filter_map(|v| v.as_float())
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(DataValue::Float)
                    .unwrap_or(DataValue::Null)
            }
            AggregateFunction::First => {
                values.first().cloned().unwrap_or(DataValue::Null)
            }
            AggregateFunction::Last => {
                values.last().cloned().unwrap_or(DataValue::Null)
            }
        }
    }
}

#[async_trait]
impl Transformer for AggregateTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        use std::collections::HashMap;
        
        let mut groups: HashMap<Vec<DataValue>, Vec<DataRow>> = HashMap::new();
        
        // Agrupa os dados
        for row in data {
            let group_key: Vec<DataValue> = self.group_by.iter()
                .map(|col| row.get(col).cloned().unwrap_or(DataValue::Null))
                .collect();
            
            groups.entry(group_key).or_insert_with(Vec::new).push(row);
        }
        
        // Aplica as agregações
        let mut result = Vec::new();
        for (group_key, group_rows) in groups {
            let mut aggregated_row = DataRow::new();
            
            // Adiciona as chaves de agrupamento
            for (i, col) in self.group_by.iter().enumerate() {
                if let Some(value) = group_key.get(i) {
                    aggregated_row.insert(col.clone(), value.clone());
                }
            }
            
            // Aplica as agregações
            for (column, func) in &self.aggregations {
                let values: Vec<DataValue> = group_rows.iter()
                    .filter_map(|row| row.get(column).cloned())
                    .collect();
                
                let aggregated_value = self.apply_aggregation(&values, func);
                aggregated_row.insert(format!("{}_{:?}", column, func).to_lowercase(), aggregated_value);
            }
            
            result.push(aggregated_row);
        }
        
        Ok(result)
    }
}

/// Transformador que processa dados em paralelo
pub struct ParallelTransform<T> {
    inner: T,
    num_workers: usize,
}

impl<T> ParallelTransform<T>
where
    T: Transformer + Clone + Send + Sync + 'static,
{
    pub fn new(inner: T, num_workers: usize) -> Self {
        Self {
            inner,
            num_workers,
        }
    }
}

#[async_trait]
impl<T> Transformer for ParallelTransform<T>
where
    T: Transformer + Clone + Send + Sync + 'static,
{
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        let chunk_size = (data.len() + self.num_workers - 1) / self.num_workers;
        let chunks: Vec<Vec<DataRow>> = data.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        let futures: Vec<_> = chunks.into_iter()
            .map(|chunk| {
                let transformer = self.inner.clone();
                tokio::spawn(async move {
                    transformer.transform(chunk).await
                })
            })
            .collect();
        
        let results = futures::future::join_all(futures).await;
        let mut final_result = Vec::new();
        
        for result in results {
            match result {
                Ok(Ok(rows)) => final_result.extend(rows),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(crate::error::ETLError::Generic(anyhow::anyhow!(e))),
            }
        }
        
        Ok(final_result)
    }
}

/// Transformador que seleciona apenas colunas específicas
#[derive(Debug, Clone)]
pub struct SelectColumnsTransform {
    columns: Vec<String>,
}

impl SelectColumnsTransform {
    pub fn new(columns: Vec<&str>) -> Self {
        Self {
            columns: columns.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[async_trait]
impl Transformer for SelectColumnsTransform {
    async fn transform(&self, data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        Ok(data.into_iter().map(|row| {
            let mut new_row = DataRow::new();
            for col in &self.columns {
                if let Some(value) = row.get(col) {
                    new_row.insert(col.clone(), value.clone());
                }
            }
            new_row
        }).collect())
    }
}

/// Transformador que combina múltiplas transformações em sequência
pub struct CompositeTransformer {
    transformers: Vec<Box<dyn Transformer + Send + Sync>>,
}

impl CompositeTransformer {
    pub fn new() -> Self {
        Self {
            transformers: Vec::new(),
        }
    }
    
    pub fn add<T: Transformer + Send + Sync + 'static>(mut self, transformer: T) -> Self {
        self.transformers.push(Box::new(transformer));
        self
    }
}

#[async_trait]
impl Transformer for CompositeTransformer {
    async fn transform(&self, mut data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        for transformer in &self.transformers {
            data = transformer.transform(data).await?;
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_filter_transform() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("age".to_string(), DataValue::Integer(30));
                row
            },
            {
                let mut row = DataRow::new();
                row.insert("age".to_string(), DataValue::Integer(15));
                row
            },
        ];
        
        let transform = FilterTransform::new(|row| {
            row.get("age").and_then(|v| v.as_integer()).unwrap_or(0) >= 18
        });
        
        let result = transform.transform(data).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("age"), Some(&DataValue::Integer(30)));
    }
    
    #[tokio::test]
    async fn test_map_transform() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("name".to_string(), DataValue::String("Alice".to_string()));
                row
            },
        ];
        
        let transform = MapTransform::new(|mut row| {
            row.insert("processed".to_string(), DataValue::Boolean(true));
            row
        });
        
        let result = transform.transform(data).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("processed"), Some(&DataValue::Boolean(true)));
    }
    
    #[tokio::test]
    async fn test_add_column_transform() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("name".to_string(), DataValue::String("Alice".to_string()));
                row
            },
        ];
        
        let transform = AddColumnTransform::new("status", DataValue::String("active".to_string()));
        let result = transform.transform(data).await.unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("status"), Some(&DataValue::String("active".to_string())));
    }
    
    #[tokio::test]
    async fn test_aggregate_transform() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("category".to_string(), DataValue::String("A".to_string()));
                row.insert("value".to_string(), DataValue::Integer(10));
                row
            },
            {
                let mut row = DataRow::new();
                row.insert("category".to_string(), DataValue::String("A".to_string()));
                row.insert("value".to_string(), DataValue::Integer(20));
                row
            },
        ];
        
        let mut aggregations = HashMap::new();
        aggregations.insert("value".to_string(), AggregateFunction::Sum);
        
        let transform = AggregateTransform::new(
            vec!["category".to_string()],
            aggregations,
        );
        
        let result = transform.transform(data).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("value_sum"), Some(&DataValue::Float(30.0)));
    }
    
    #[tokio::test]
    async fn test_select_columns_transform() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("id".to_string(), DataValue::Integer(1));
                row.insert("name".to_string(), DataValue::String("Alice".to_string()));
                row.insert("age".to_string(), DataValue::Integer(30));
                row
            },
            {
                let mut row = DataRow::new();
                row.insert("id".to_string(), DataValue::Integer(2));
                row.insert("name".to_string(), DataValue::String("Bob".to_string()));
                row.insert("age".to_string(), DataValue::Integer(25));
                row
            },
        ];
        
        let transform = SelectColumnsTransform::new(vec!["id", "name"]);
        let result = transform.transform(data).await.unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get("id"), Some(&DataValue::Integer(1)));
        assert_eq!(result[0].get("name"), Some(&DataValue::String("Alice".to_string())));
        assert_eq!(result[0].get("age"), None);
    }
    
    #[tokio::test]
    async fn test_composite_transformer() {
        let data = vec![
            {
                let mut row = DataRow::new();
                row.insert("id".to_string(), DataValue::Integer(1));
                row.insert("value".to_string(), DataValue::Integer(10));
                row
            },
            {
                let mut row = DataRow::new();
                row.insert("id".to_string(), DataValue::Integer(2));
                row.insert("value".to_string(), DataValue::Integer(20));
                row
            },
        ];
        
        let transform = CompositeTransformer::new()
            .add(FilterTransform::new(|row| {
                row.get("value").and_then(|v| v.as_integer()).unwrap_or(0) > 15
            }))
            .add(MapTransform::new(|mut row| {
                row.insert("value_doubled".to_string(), DataValue::Integer(row.get("value").and_then(|v| v.as_integer()).unwrap_or(0) * 2));
                row
            }));
        
        let result = transform.transform(data).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("id"), Some(&DataValue::Integer(2)));
        assert_eq!(result[0].get("value_doubled"), Some(&DataValue::Integer(40)));
    }
}
