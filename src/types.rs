use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeSet};
use std::hash::{Hash, Hasher};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// Representa uma linha de dados genérica
pub type DataRow = HashMap<String, DataValue>;

/// Valores de dados suportados
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Array(Vec<DataValue>),
    Object(HashMap<String, DataValue>),
    /// Data sem horário (YYYY-MM-DD)
    Date(NaiveDate),
    /// Data e horário sem timezone (YYYY-MM-DD HH:MM:SS)
    DateTime(NaiveDateTime),
    /// Timestamp com timezone UTC
    Timestamp(DateTime<Utc>),
}

impl Eq for DataValue {}

impl Hash for DataValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            DataValue::String(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            DataValue::Integer(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            DataValue::Float(f) => {
                2u8.hash(state);
                // Para f64, convertemos para bits para hash
                f.to_bits().hash(state);
            }
            DataValue::Boolean(b) => {
                3u8.hash(state);
                b.hash(state);
            }
            DataValue::Null => {
                4u8.hash(state);
            }
            DataValue::Array(arr) => {
                5u8.hash(state);
                arr.hash(state);
            }
            DataValue::Object(obj) => {
                6u8.hash(state);
                // Para HashMap, ordenamos as chaves antes de fazer hash
                let mut sorted_keys: Vec<_> = obj.keys().collect();
                sorted_keys.sort();
                for key in sorted_keys {
                    key.hash(state);
                    obj[key].hash(state);
                }
            }
            DataValue::Date(date) => {
                7u8.hash(state);
                date.hash(state);
            }
            DataValue::DateTime(dt) => {
                8u8.hash(state);
                dt.hash(state);
            }
            DataValue::Timestamp(ts) => {
                9u8.hash(state);
                ts.hash(state);
            }
        }
    }
}

impl PartialOrd for DataValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DataValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            (DataValue::Null, DataValue::Null) => Ordering::Equal,
            (DataValue::Null, _) => Ordering::Less,
            (_, DataValue::Null) => Ordering::Greater,

            (DataValue::Boolean(a), DataValue::Boolean(b)) => a.cmp(b),
            (DataValue::Boolean(_), _) => Ordering::Less,
            (_, DataValue::Boolean(_)) => Ordering::Greater,

            (DataValue::Integer(a), DataValue::Integer(b)) => a.cmp(b),
            (DataValue::Integer(a), DataValue::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .unwrap_or(Ordering::Equal),
            (DataValue::Integer(_), _) => Ordering::Less,

            (DataValue::Float(a), DataValue::Integer(b)) => a
                .partial_cmp(&(*b as f64))
                .unwrap_or(Ordering::Equal),
            (DataValue::Float(a), DataValue::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (DataValue::Float(_), _) => Ordering::Less,

            (DataValue::String(a), DataValue::String(b)) => a.cmp(b),
            (DataValue::String(_), _) => Ordering::Less,

            (DataValue::Array(a), DataValue::Array(b)) => a.cmp(b),
            (DataValue::Array(_), _) => Ordering::Less,

            (DataValue::Object(a), DataValue::Object(b)) => {
                // Para objetos, comparamos as chaves ordenadas
                let a_keys: BTreeSet<_> = a.keys().collect();
                let b_keys: BTreeSet<_> = b.keys().collect();

                match a_keys.cmp(&b_keys) {
                    Ordering::Equal => {
                        // Se as chaves são iguais, comparamos os valores
                        for key in a_keys {
                            match a[key].cmp(&b[key]) {
                                Ordering::Equal => continue,
                                other => return other,
                            }
                        }
                        Ordering::Equal
                    }
                    other => other,
                }
            }
            (DataValue::Object(_), DataValue::Date(_)) => Ordering::Less,
            (DataValue::Object(_), DataValue::DateTime(_)) => Ordering::Less,
            (DataValue::Object(_), DataValue::Timestamp(_)) => Ordering::Less,
            (DataValue::Object(_), _) => Ordering::Greater,

            (DataValue::Date(a), DataValue::Date(b)) => a.cmp(b),
            (DataValue::Date(_), DataValue::DateTime(_)) => Ordering::Less,
            (DataValue::Date(_), DataValue::Timestamp(_)) => Ordering::Less,
            (DataValue::Date(_), _) => Ordering::Greater,

            (DataValue::DateTime(a), DataValue::DateTime(b)) => a.cmp(b),
            (DataValue::DateTime(_), DataValue::Timestamp(_)) => Ordering::Less,
            (DataValue::DateTime(_), _) => Ordering::Greater,

            (DataValue::Timestamp(a), DataValue::Timestamp(b)) => a.cmp(b),
            (DataValue::Timestamp(_), _) => Ordering::Greater,
        }
    }
}

impl From<String> for DataValue {
    fn from(value: String) -> Self {
        DataValue::String(value)
    }
}

impl From<&str> for DataValue {
    fn from(value: &str) -> Self {
        DataValue::String(value.to_string())
    }
}

impl From<i64> for DataValue {
    fn from(value: i64) -> Self {
        DataValue::Integer(value)
    }
}

impl From<f64> for DataValue {
    fn from(value: f64) -> Self {
        DataValue::Float(value)
    }
}

impl From<bool> for DataValue {
    fn from(value: bool) -> Self {
        DataValue::Boolean(value)
    }
}

impl From<NaiveDate> for DataValue {
    fn from(value: NaiveDate) -> Self {
        DataValue::Date(value)
    }
}

impl From<NaiveDateTime> for DataValue {
    fn from(value: NaiveDateTime) -> Self {
        DataValue::DateTime(value)
    }
}

impl From<DateTime<Utc>> for DataValue {
    fn from(value: DateTime<Utc>) -> Self {
        DataValue::Timestamp(value)
    }
}

impl DataValue {
    /// Converte para string se possível
    pub fn as_string(&self) -> Option<String> {
        match self {
            DataValue::String(s) => Some(s.clone()),
            DataValue::Integer(i) => Some(i.to_string()),
            DataValue::Float(f) => Some(f.to_string()),
            DataValue::Boolean(b) => Some(b.to_string()),
            DataValue::Date(d) => Some(d.format("%Y-%m-%d").to_string()),
            DataValue::DateTime(dt) => Some(dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            DataValue::Timestamp(ts) => Some(ts.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
            DataValue::Null => None,
            _ => None,
        }
    }

    /// Converte para inteiro se possível
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DataValue::Integer(i) => Some(*i),
            DataValue::String(s) => s.parse().ok(),
            DataValue::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// Converte para float se possível
    pub fn as_float(&self) -> Option<f64> {
        match self {
            DataValue::Float(f) => Some(*f),
            DataValue::Integer(i) => Some(*i as f64),
            DataValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Converte para boolean se possível
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            DataValue::Boolean(b) => Some(*b),
            DataValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "y" => Some(true),
                "false" | "0" | "no" | "n" => Some(false),
                _ => None,
            },
            DataValue::Integer(i) => Some(*i != 0),
            _ => None,
        }
    }

    /// Converte para data (NaiveDate) se possível
    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            DataValue::Date(d) => Some(*d),
            DataValue::DateTime(dt) => Some(dt.date()),
            DataValue::Timestamp(ts) => Some(ts.naive_utc().date()),
            DataValue::String(s) => {
                // Tenta parsear diferentes formatos de data
                if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    Some(date)
                } else if let Ok(date) = NaiveDate::parse_from_str(s, "%d/%m/%Y") {
                    Some(date)
                } else if let Ok(date) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
                    Some(date)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converte para datetime (NaiveDateTime) se possível
    pub fn as_datetime(&self) -> Option<NaiveDateTime> {
        match self {
            DataValue::DateTime(dt) => Some(*dt),
            DataValue::Timestamp(ts) => Some(ts.naive_utc()),
            DataValue::Date(d) => Some(d.and_hms_opt(0, 0, 0).unwrap_or_default()),
            DataValue::String(s) => {
                // Tenta parsear diferentes formatos de datetime
                if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                    Some(dt)
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%d/%m/%Y %H:%M:%S") {
                    Some(dt)
                } else if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
                    Some(dt)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converte para timestamp (DateTime<Utc>) se possível
    pub fn as_timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            DataValue::Timestamp(ts) => Some(*ts),
            DataValue::DateTime(dt) => Some(DateTime::from_naive_utc_and_offset(*dt, Utc)),
            DataValue::Date(d) => {
                let dt = d.and_hms_opt(0, 0, 0).unwrap_or_default();
                Some(DateTime::from_naive_utc_and_offset(dt, Utc))
            }
            DataValue::String(s) => {
                // Tenta parsear timestamp ISO 8601
                if let Ok(ts) = DateTime::parse_from_rfc3339(s) {
                    Some(ts.with_timezone(&Utc))
                } else if let Ok(ts) = s.parse::<DateTime<Utc>>() {
                    Some(ts)
                } else if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    // Se for só uma data, converte para timestamp no início do dia
                    let dt = date.and_hms_opt(0, 0, 0).unwrap_or_default();
                    Some(DateTime::from_naive_utc_and_offset(dt, Utc))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Verifica se é nulo
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }
}

/// Resultado de uma operação de pipeline
#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub rows_processed: usize,
    pub rows_successful: usize,
    pub rows_failed: usize,
    pub execution_time_ms: u64,
    pub errors: Vec<String>,
}

impl PipelineResult {
    pub fn new() -> Self {
        Self {
            rows_processed: 0,
            rows_successful: 0,
            rows_failed: 0,
            execution_time_ms: 0,
            errors: Vec::new(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.rows_processed == 0 {
            0.0
        } else {
            self.rows_successful as f64 / self.rows_processed as f64
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Estados do pipeline para rastreamento de execução
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineState {
    Idle,
    Extracting,
    Transforming,
    Loading,
    Completed,
    Failed(String),
}

impl Default for PipelineState {
    fn default() -> Self {
        PipelineState::Idle
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineState::Idle => write!(f, "Ocioso"),
            PipelineState::Extracting => write!(f, "Extraindo"),
            PipelineState::Transforming => write!(f, "Transformando"),
            PipelineState::Loading => write!(f, "Carregando"),
            PipelineState::Completed => write!(f, "Concluído"),
            PipelineState::Failed(error) => write!(f, "Falhou: {}", error),
        }
    }
}

/// Eventos do pipeline para monitoramento externo
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Pipeline iniciado
    Started { 
        pipeline_id: String,
        timestamp: std::time::SystemTime,
    },
    /// Estado alterado
    StateChanged { 
        pipeline_id: String,
        old_state: PipelineState,
        new_state: PipelineState,
        timestamp: std::time::SystemTime,
    },
    /// Batch processado
    BatchProcessed { 
        pipeline_id: String,
        batch_number: usize,
        rows_count: usize,
        timestamp: std::time::SystemTime,
    },
    /// Erro ocorreu
    Error { 
        pipeline_id: String,
        error: String,
        timestamp: std::time::SystemTime,
    },
    /// Pipeline concluído
    Completed { 
        pipeline_id: String,
        result: PipelineResult,
        timestamp: std::time::SystemTime,
    },
}
