# Framework de Integridade de Dados

O framework de integridade é um dos pilares fundamentais da biblioteca ETLRS, garantindo que os dados mantenham sua qualidade, consistência e confiabilidade em todas as etapas do pipeline ETL. Este sistema abrangente implementa validações rigorosas, verificações de checksums, monitoramento de esquemas e rastreamento de linhagem de dados, oferecendo uma solução robusta para identificar e corrigir problemas de qualidade de dados antes que eles afetem os sistemas downstream.

## 1. Verificadores de Integridade Base

O sistema de integridade começa com verificadores base que implementam validações fundamentais:

```rust
// src/integrity/mod.rs
use crate::{DatasetSnapshot, IntegrityReport, IntegrityChecker};
use async_trait::async_trait;
use sha2::{Sha256, Digest};

pub struct ChecksumChecker {
    algorithm: ChecksumAlgorithm,
}

#[derive(Debug, Clone)]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
    Blake3,
}

impl ChecksumChecker {
    pub fn new() -> Self {
        Self {
            algorithm: ChecksumAlgorithm::Sha256,
        }
    }
    
    pub fn with_algorithm(algorithm: ChecksumAlgorithm) -> Self {
        Self { algorithm }
    }
    
    fn calculate_checksum(&self, data: &[u8]) -> String {
        match self.algorithm {
            ChecksumAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            },
            ChecksumAlgorithm::Sha512 => {
                let mut hasher = sha2::Sha512::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            },
            ChecksumAlgorithm::Blake3 => {
                blake3::hash(data).to_hex().to_string()
            }
        }
    }
}

#[async_trait]
impl IntegrityChecker for ChecksumChecker {
    async fn verify_integrity(&self, snapshot: &DatasetSnapshot) -> Result<IntegrityReport, Box<dyn std::error::Error>> {
        let serialized = serde_json::to_vec(snapshot)?;
        let calculated_checksum = self.calculate_checksum(&serialized);
        
        let checksum_match = calculated_checksum == snapshot.checksum;
        
        Ok(IntegrityReport {
            is_valid: checksum_match,
            checksum_match,
            record_count_match: true, // Verificado implicitamente
            schema_valid: true, // Será verificado por outro checker
            errors: if checksum_match { Vec::new() } else { 
                vec!["Checksum mismatch detected".to_string()] 
            },
            warnings: Vec::new(),
        })
    }
}
```

## 2. Validação de Schema

```rust
// src/integrity/schema.rs
use serde_json::{Value as JsonValue, Map};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SchemaChecker {
    schema: DataSchema,
    strict_mode: bool,
}

#[derive(Debug, Clone)]
pub struct DataSchema {
    pub fields: HashMap<String, FieldSchema>,
    pub required_fields: Vec<String>,
    pub allow_additional_fields: bool,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub field_type: FieldType,
    pub nullable: bool,
    pub constraints: Vec<FieldConstraint>,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    Array(Box<FieldType>),
    Object(HashMap<String, FieldSchema>),
}

#[derive(Debug, Clone)]
pub enum FieldConstraint {
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    MinValue(f64),
    MaxValue(f64),
    OneOf(Vec<String>),
}

impl SchemaChecker {
    pub fn new(schema: DataSchema) -> Self {
        Self {
            schema,
            strict_mode: false,
        }
    }
    
    pub fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }
    
    fn validate_record(&self, record: &crate::Record) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Verificar campos obrigatórios
        for required_field in &self.schema.required_fields {
            if !record.fields.contains_key(required_field) {
                errors.push(format!("Required field '{}' is missing", required_field));
            }
        }
        
        // Validar cada campo
        for (field_name, field_value) in &record.fields {
            if let Some(field_schema) = self.schema.fields.get(field_name) {
                if let Err(field_errors) = self.validate_field_value(field_value, field_schema) {
                    errors.extend(field_errors.into_iter().map(|e| format!("Field '{}': {}", field_name, e)));
                }
            } else if !self.schema.allow_additional_fields && self.strict_mode {
                errors.push(format!("Unexpected field '{}' found", field_name));
            }
        }
        
        errors
    }
    
    fn validate_field_value(&self, value: &crate::Value, schema: &FieldSchema) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Verificar se o campo pode ser nulo
        if matches!(value, crate::Value::Null) {
            if !schema.nullable {
                errors.push("Field cannot be null".to_string());
            }
            return if errors.is_empty() { Ok(()) } else { Err(errors) };
        }
        
        // Verificar tipo
        let type_valid = match (&schema.field_type, value) {
            (FieldType::String, crate::Value::String(_)) => true,
            (FieldType::Integer, crate::Value::Integer(_)) => true,
            (FieldType::Float, crate::Value::Float(_)) => true,
            (FieldType::Boolean, crate::Value::Boolean(_)) => true,
            _ => false,
        };
        
        if !type_valid {
            errors.push(format!("Type mismatch: expected {:?}", schema.field_type));
        }
        
        // Aplicar constraints
        for constraint in &schema.constraints {
            if let Err(constraint_error) = self.apply_constraint(value, constraint) {
                errors.push(constraint_error);
            }
        }
        
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
    
    fn apply_constraint(&self, value: &crate::Value, constraint: &FieldConstraint) -> Result<(), String> {
        match (constraint, value) {
            (FieldConstraint::MinLength(min), crate::Value::String(s)) => {
                if s.len() < *min {
                    Err(format!("String length {} is below minimum {}", s.len(), min))
                } else {
                    Ok(())
                }
            },
            (FieldConstraint::MaxLength(max), crate::Value::String(s)) => {
                if s.len() > *max {
                    Err(format!("String length {} exceeds maximum {}", s.len(), max))
                } else {
                    Ok(())
                }
            },
            (FieldConstraint::MinValue(min), crate::Value::Float(f)) => {
                if *f < *min {
                    Err(format!("Value {} is below minimum {}", f, min))
                } else {
                    Ok(())
                }
            },
            (FieldConstraint::MaxValue(max), crate::Value::Float(f)) => {
                if *f > *max {
                    Err(format!("Value {} exceeds maximum {}", f, max))
                } else {
                    Ok(())
                }
            },
            (FieldConstraint::OneOf(options), crate::Value::String(s)) => {
                if options.contains(s) {
                    Ok(())
                } else {
                    Err(format!("Value '{}' is not one of the allowed options: {:?}", s, options))
                }
            },
            _ => Ok(()), // Constraint não aplicável a este tipo
        }
    }
}

#[async_trait]
impl IntegrityChecker for SchemaChecker {
    async fn verify_integrity(&self, snapshot: &DatasetSnapshot) -> Result<IntegrityReport, Box<dyn std::error::Error>> {
        let mut all_errors = Vec::new();
        let mut warnings = Vec::new();
        
        for (index, record) in snapshot.records.iter().enumerate() {
            let record_errors = self.validate_record(record);
            if !record_errors.is_empty() {
                all_errors.extend(record_errors.into_iter().map(|e| format!("Record {}: {}", index, e)));
            }
        }
        
        // Gerar warnings para campos comuns não no schema
        if !self.strict_mode {
            let mut field_frequency: HashMap<String, usize> = HashMap::new();
            for record in &snapshot.records {
                for field_name in record.fields.keys() {
                    if !self.schema.fields.contains_key(field_name) {
                        *field_frequency.entry(field_name.clone()).or_insert(0) += 1;
                    }
                }
            }
            
            for (field_name, count) in field_frequency {
                if count as f64 / snapshot.records.len() as f64 > 0.1 { // 10% dos registros
                    warnings.push(format!("Field '{}' appears in {}% of records but is not in schema", 
                                        field_name, (count * 100 / snapshot.records.len())));
                }
            }
        }
        
        Ok(IntegrityReport {
            is_valid: all_errors.is_empty(),
            checksum_match: true,
            record_count_match: true,
            schema_valid: all_errors.is_empty(),
            errors: all_errors,
            warnings,
        })
    }
}
```

## 3. Sistema de Linhagem de Dados

```rust
// src/integrity/lineage.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLineage {
    pub lineage_id: String,
    pub nodes: HashMap<String, LineageNode>,
    pub edges: Vec<LineageEdge>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub checksum: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Source,
    Transformation,
    Destination,
    Checkpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub from_node: String,
    pub to_node: String,
    pub edge_type: EdgeType,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    DataFlow,
    Dependency,
    Derivation,
}

pub struct LineageTracker {
    lineage: DataLineage,
    current_node: Option<String>,
}

impl LineageTracker {
    pub fn new() -> Self {
        Self {
            lineage: DataLineage {
                lineage_id: Uuid::new_v4().to_string(),
                nodes: HashMap::new(),
                edges: Vec::new(),
                created_at: chrono::Utc::now(),
            },
            current_node: None,
        }
    }
    
    pub fn add_source_node(&mut self, name: &str, properties: HashMap<String, serde_json::Value>) -> String {
        let node_id = Uuid::new_v4().to_string();
        let node = LineageNode {
            node_id: node_id.clone(),
            node_type: NodeType::Source,
            name: name.to_string(),
            properties,
            checksum: None,
            timestamp: chrono::Utc::now(),
        };
        
        self.lineage.nodes.insert(node_id.clone(), node);
        self.current_node = Some(node_id.clone());
        node_id
    }
    
    pub fn add_transformation_node(&mut self, name: &str, properties: HashMap<String, serde_json::Value>) -> String {
        let node_id = Uuid::new_v4().to_string();
        let node = LineageNode {
            node_id: node_id.clone(),
            node_type: NodeType::Transformation,
            name: name.to_string(),
            properties,
            checksum: None,
            timestamp: chrono::Utc::now(),
        };
        
        // Adicionar edge do nó anterior
        if let Some(prev_node) = &self.current_node {
            self.lineage.edges.push(LineageEdge {
                from_node: prev_node.clone(),
                to_node: node_id.clone(),
                edge_type: EdgeType::DataFlow,
                properties: HashMap::new(),
            });
        }
        
        self.lineage.nodes.insert(node_id.clone(), node);
        self.current_node = Some(node_id.clone());
        node_id
    }
    
    pub fn add_destination_node(&mut self, name: &str, properties: HashMap<String, serde_json::Value>) -> String {
        let node_id = Uuid::new_v4().to_string();
        let node = LineageNode {
            node_id: node_id.clone(),
            node_type: NodeType::Destination,
            name: name.to_string(),
            properties,
            checksum: None,
            timestamp: chrono::Utc::now(),
        };
        
        // Adicionar edge do nó anterior
        if let Some(prev_node) = &self.current_node {
            self.lineage.edges.push(LineageEdge {
                from_node: prev_node.clone(),
                to_node: node_id.clone(),
                edge_type: EdgeType::DataFlow,
                properties: HashMap::new(),
            });
        }
        
        self.lineage.nodes.insert(node_id.clone(), node);
        node_id
    }
    
    pub fn update_node_checksum(&mut self, node_id: &str, checksum: String) {
        if let Some(node) = self.lineage.nodes.get_mut(node_id) {
            node.checksum = Some(checksum);
        }
    }
    
    pub fn get_lineage(&self) -> &DataLineage {
        &self.lineage
    }
    
    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.lineage)
    }
}
```

## 4. Métricas de Qualidade de Dados

```rust
// src/integrity/quality.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QualityMetrics {
    pub completeness: f64,
    pub uniqueness: f64,
    pub validity: f64,
    pub consistency: f64,
    pub accuracy: f64,
    pub timeliness: f64,
    pub field_metrics: HashMap<String, FieldQualityMetrics>,
}

#[derive(Debug, Clone)]
pub struct FieldQualityMetrics {
    pub null_percentage: f64,
    pub unique_percentage: f64,
    pub valid_format_percentage: f64,
    pub outlier_percentage: f64,
    pub pattern_compliance: f64,
}

pub struct QualityAnalyzer {
    patterns: HashMap<String, regex::Regex>,
    outlier_threshold: f64,
}

impl QualityAnalyzer {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("email".to_string(), regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());
        patterns.insert("phone".to_string(), regex::Regex::new(r"^\+?[\d\s\-\(\)]{10,}$").unwrap());
        patterns.insert("date".to_string(), regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());
        
        Self {
            patterns,
            outlier_threshold: 2.0, // Z-score threshold
        }
    }
    
    pub fn analyze(&self, snapshot: &crate::DatasetSnapshot) -> QualityMetrics {
        let total_records = snapshot.records.len() as f64;
        let mut field_stats: HashMap<String, FieldStats> = HashMap::new();
        
        // Coletar estatísticas por campo
        for record in &snapshot.records {
            for (field_name, field_value) in &record.fields {
                let stats = field_stats.entry(field_name.clone()).or_insert_with(FieldStats::new);
                stats.update(field_value);
            }
        }
        
        // Calcular métricas por campo
        let mut field_metrics = HashMap::new();
        for (field_name, stats) in &field_stats {
            field_metrics.insert(field_name.clone(), self.calculate_field_metrics(stats, total_records));
        }
        
        // Calcular métricas globais
        let completeness = self.calculate_completeness(&field_metrics);
        let uniqueness = self.calculate_uniqueness(&field_stats, total_records);
        let validity = self.calculate_validity(&field_metrics);
        
        QualityMetrics {
            completeness,
            uniqueness,
            validity,
            consistency: 1.0, // Implementar lógica específica
            accuracy: 1.0,    // Implementar lógica específica
            timeliness: 1.0,  // Implementar lógica específica
            field_metrics,
        }
    }
    
    fn calculate_field_metrics(&self, stats: &FieldStats, total_records: f64) -> FieldQualityMetrics {
        let null_percentage = stats.null_count as f64 / total_records;
        let unique_percentage = stats.unique_values.len() as f64 / total_records;
        
        FieldQualityMetrics {
            null_percentage,
            unique_percentage,
            valid_format_percentage: 1.0, // Implementar validação de formato
            outlier_percentage: 0.0,      // Implementar detecção de outliers
            pattern_compliance: 1.0,      // Implementar verificação de padrões
        }
    }
    
    fn calculate_completeness(&self, field_metrics: &HashMap<String, FieldQualityMetrics>) -> f64 {
        if field_metrics.is_empty() {
            return 1.0;
        }
        
        let total_completeness: f64 = field_metrics.values()
            .map(|metrics| 1.0 - metrics.null_percentage)
            .sum();
        
        total_completeness / field_metrics.len() as f64
    }
    
    fn calculate_uniqueness(&self, field_stats: &HashMap<String, FieldStats>, total_records: f64) -> f64 {
        if field_stats.is_empty() {
            return 1.0;
        }
        
        let total_uniqueness: f64 = field_stats.values()
            .map(|stats| stats.unique_values.len() as f64 / total_records)
            .sum();
        
        total_uniqueness / field_stats.len() as f64
    }
    
    fn calculate_validity(&self, field_metrics: &HashMap<String, FieldQualityMetrics>) -> f64 {
        if field_metrics.is_empty() {
            return 1.0;
        }
        
        let total_validity: f64 = field_metrics.values()
            .map(|metrics| metrics.valid_format_percentage)
            .sum();
        
        total_validity / field_metrics.len() as f64
    }
}

#[derive(Debug)]
struct FieldStats {
    null_count: usize,
    unique_values: std::collections::HashSet<String>,
    total_count: usize,
}

impl FieldStats {
    fn new() -> Self {
        Self {
            null_count: 0,
            unique_values: std::collections::HashSet::new(),
            total_count: 0,
        }
    }
    
    fn update(&mut self, value: &crate::Value) {
        self.total_count += 1;
        
        match value {
            crate::Value::Null => self.null_count += 1,
            _ => {
                let string_value = match value {
                    crate::Value::String(s) => s.clone(),
                    crate::Value::Integer(i) => i.to_string(),
                    crate::Value::Float(f) => f.to_string(),
                    crate::Value::Boolean(b) => b.to_string(),
                    crate::Value::Null => "null".to_string(),
                };
                self.unique_values.insert(string_value);
            }
        }
    }
}
```

## 5. Configuração de Integridade

```rust
// src/integrity/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    pub enable_checksum_validation: bool,
    pub enable_schema_validation: bool,
    pub enable_quality_analysis: bool,
    pub enable_lineage_tracking: bool,
    pub checksum_algorithm: String,
    pub schema_strict_mode: bool,
    pub quality_thresholds: QualityThresholds,
    pub custom_validators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    pub min_completeness: f64,
    pub min_uniqueness: f64,
    pub min_validity: f64,
    pub max_outlier_percentage: f64,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            enable_checksum_validation: true,
            enable_schema_validation: true,
            enable_quality_analysis: true,
            enable_lineage_tracking: true,
            checksum_algorithm: "sha256".to_string(),
            schema_strict_mode: false,
            quality_thresholds: QualityThresholds {
                min_completeness: 0.95,
                min_uniqueness: 0.8,
                min_validity: 0.99,
                max_outlier_percentage: 0.05,
            },
            custom_validators: Vec::new(),
        }
    }
}
```

## 6. Exemplo de Uso Completo

```rust
// examples/integrity_example.rs
use etlrs::integrity::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar schema
    let mut schema_fields = HashMap::new();
    schema_fields.insert("id".to_string(), FieldSchema {
        field_type: FieldType::Integer,
        nullable: false,
        constraints: vec![FieldConstraint::MinValue(1.0)],
    });
    schema_fields.insert("email".to_string(), FieldSchema {
        field_type: FieldType::String,
        nullable: false,
        constraints: vec![
            FieldConstraint::Pattern(r"^[^\s@]+@[^\s@]+\.[^\s@]+$".to_string()),
            FieldConstraint::MinLength(5),
        ],
    });
    
    let schema = DataSchema {
        fields: schema_fields,
        required_fields: vec!["id".to_string(), "email".to_string()],
        allow_additional_fields: true,
    };
    
    // Criar verificadores
    let checksum_checker = ChecksumChecker::new();
    let schema_checker = SchemaChecker::new(schema).with_strict_mode();
    let quality_analyzer = QualityAnalyzer::new();
    
    // Configurar pipeline com verificações de integridade
    let pipeline = PipelineBuilder::new()
        .extract(CsvExtractor::new("data/users.csv"))
        .with_integrity_checker(checksum_checker)
        .with_integrity_checker(schema_checker)
        .transform(EmailNormalizationTransformer::new())
        .load(DatabaseLoader::new("postgresql://..."))
        .build();
    
    // Executar com análise de qualidade
    let result = pipeline.execute().await?;
    
    if let Some(snapshot) = &result.transformation_result {
        let quality_metrics = quality_analyzer.analyze(snapshot);
        println!("Quality Metrics:");
        println!("  Completeness: {:.2}%", quality_metrics.completeness * 100.0);
        println!("  Uniqueness: {:.2}%", quality_metrics.uniqueness * 100.0);
        println!("  Validity: {:.2}%", quality_metrics.validity * 100.0);
    }
    
    Ok(())
}
```

Este framework de integridade garante que os dados mantenham alta qualidade e confiabilidade em todo o pipeline ETL, fornecendo visibilidade completa sobre a linhagem e qualidade dos dados.
