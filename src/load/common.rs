//! # Common Utilities
//! 
//! Módulo para utilitários e helpers comuns para loaders.
//! 
//! Este módulo agora contém apenas utilitários auxiliares, pois os loaders específicos
//! foram movidos para seus próprios módulos:
//! - ConsoleLoader: `crate::load::console`
//! - MemoryLoader: `crate::load::memory`
//! - JsonLoader/JsonLinesLoader: `crate::load::json`
//! - ParquetLoader: `crate::load::parquet`

use std::collections::HashMap;
use crate::types::{DataRow, DataValue};

/// Utilitários para formatação de dados
pub struct DataFormatter;

impl DataFormatter {
    /// Formata um DataValue para exibição em texto
    pub fn format_value(value: &DataValue) -> String {
        match value {
            DataValue::String(s) => s.clone(),
            DataValue::Integer(i) => i.to_string(),
            DataValue::Float(f) => f.to_string(),
            DataValue::Boolean(b) => b.to_string(),
            DataValue::Null => "null".to_string(),
            DataValue::Array(arr) => {
                let formatted: Vec<String> = arr.iter().map(Self::format_value).collect();
                format!("[{}]", formatted.join(", "))
            },
            DataValue::Object(obj) => {
                let formatted: Vec<String> = obj.iter()
                    .map(|(k, v)| format!("{}: {}", k, Self::format_value(v)))
                    .collect();
                format!("{{{}}}", formatted.join(", "))
            },
            DataValue::Date(date) => date.format("%Y-%m-%d").to_string(),
            DataValue::DateTime(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            DataValue::Timestamp(ts) => ts.to_rfc3339(),
        }
    }
    
    /// Formata uma linha completa para exibição
    pub fn format_row(row: &DataRow) -> String {
        let formatted_fields: Vec<String> = row.iter()
            .map(|(key, value)| format!("{}: {}", key, Self::format_value(value)))
            .collect();
        formatted_fields.join(" | ")
    }
    
    /// Conta tipos de valores em uma coleção de linhas
    pub fn count_value_types(rows: &[DataRow]) -> HashMap<String, usize> {
        let mut type_counts = HashMap::new();
        
        for row in rows {
            for value in row.values() {
                let type_name = match value {
                    DataValue::String(_) => "String",
                    DataValue::Integer(_) => "Integer", 
                    DataValue::Float(_) => "Float",
                    DataValue::Boolean(_) => "Boolean",
                    DataValue::Null => "Null",
                    DataValue::Array(_) => "Array",
                    DataValue::Object(_) => "Object",
                    DataValue::Date(_) => "Date",
                    DataValue::DateTime(_) => "DateTime",
                    DataValue::Timestamp(_) => "Timestamp",
                };
                *type_counts.entry(type_name.to_string()).or_insert(0) += 1;
            }
        }
        
        type_counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_value() {
        assert_eq!(DataFormatter::format_value(&DataValue::String("test".to_string())), "test");
        assert_eq!(DataFormatter::format_value(&DataValue::Integer(42)), "42");
        assert_eq!(DataFormatter::format_value(&DataValue::Float(3.14)), "3.14");
        assert_eq!(DataFormatter::format_value(&DataValue::Boolean(true)), "true");
        assert_eq!(DataFormatter::format_value(&DataValue::Null), "null");
    }
    
    #[test] 
    fn test_format_row() {
        let mut row = HashMap::new();
        row.insert("name".to_string(), DataValue::String("test".to_string()));
        row.insert("age".to_string(), DataValue::Integer(25));
        
        let formatted = DataFormatter::format_row(&row);
        // A ordem pode variar devido ao HashMap, então vamos verificar se contém as partes
        assert!(formatted.contains("name: test"));
        assert!(formatted.contains("age: 25"));
        assert!(formatted.contains(" | "));
    }
    
    #[test]
    fn test_count_value_types() {
        let mut row1 = HashMap::new();
        row1.insert("str".to_string(), DataValue::String("test".to_string()));
        row1.insert("num".to_string(), DataValue::Integer(42));
        
        let mut row2 = HashMap::new();
        row2.insert("str2".to_string(), DataValue::String("test2".to_string()));
        row2.insert("bool".to_string(), DataValue::Boolean(true));
        
        let rows = vec![row1, row2];
        let counts = DataFormatter::count_value_types(&rows);
        
        assert_eq!(counts.get("String"), Some(&2));
        assert_eq!(counts.get("Integer"), Some(&1));
        assert_eq!(counts.get("Boolean"), Some(&1));
    }
}
