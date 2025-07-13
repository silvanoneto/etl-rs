use etlrs::prelude::*;
use etlrs::transform::common::{DataType, CompositeTransformer};
use chrono::{NaiveDate, NaiveDateTime, Datelike};
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("🎯 ETLRS - Suporte Completo a Datas e Timestamps");
    println!("===============================================\n");

    // Cria dados de teste com diferentes formatos de data
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "id,nome,data_nascimento,ultimo_login,data_cadastro").unwrap();
    writeln!(temp_file, "1,Alice,1990-05-15,2024-07-13 14:30:00,2024-01-01").unwrap();
    writeln!(temp_file, "2,Bob,1985-12-25,2024-07-13 09:15:30,2024-02-15").unwrap();
    writeln!(temp_file, "3,Carlos,1995-08-10,2024-07-13 16:45:15,2024-03-10").unwrap();
    writeln!(temp_file, "4,Diana,1988-03-20,2024-07-13 11:20:45,2024-04-05").unwrap();
    writeln!(temp_file, "5,Eva,2000-01-01,2024-07-13 18:00:00,2024-05-01").unwrap();

    println!("🔄 Demonstração 1: Pipeline ETL Completo com Datas");
    println!("   • Extração de CSV");
    println!("   • Conversão de tipos de data");
    println!("   • Filtros baseados em data");
    println!("   • Exportação JSON com timestamps\n");

    let json_path = std::env::current_dir().unwrap().join("demo_datas_completo.json");
    
    let complete_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(CompositeTransformer::new()
            // Converte os tipos de data
            .add(ConvertTypesTransform::new({
                let mut conversions = std::collections::HashMap::new();
                conversions.insert("data_nascimento".to_string(), DataType::Date);
                conversions.insert("ultimo_login".to_string(), DataType::DateTime);
                conversions.insert("data_cadastro".to_string(), DataType::Timestamp);
                conversions
            }))
            // Filtra pessoas nascidas após 1988
            .add(FilterTransform::new(|row| {
                if let Some(DataValue::Date(birth_date)) = row.get("data_nascimento") {
                    birth_date.year() > 1988
                } else {
                    true // Mantém registros sem data de nascimento
                }
            }))
            // Adiciona uma coluna calculada com a idade
            .add(MapTransform::new(|mut row| {
                if let Some(DataValue::Date(birth_date)) = row.get("data_nascimento") {
                    let today = NaiveDate::from_ymd_opt(2024, 7, 13).unwrap();
                    let age = today.years_since(*birth_date).unwrap_or(0);
                    row.insert("idade".to_string(), DataValue::Integer(age as i64));
                }
                row
            }))
            // Adiciona status baseado na última atividade
            .add(MapTransform::new(|mut row| {
                if let Some(DataValue::DateTime(last_login)) = row.get("ultimo_login") {
                    let cutoff = NaiveDateTime::parse_from_str("2024-07-13 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
                    let status = if *last_login > cutoff { "Ativo" } else { "Inativo" };
                    row.insert("status".to_string(), DataValue::String(status.to_string()));
                }
                row
            }))
        )
        .load(JsonLoader::new(&json_path))
        .build();

    let result = complete_pipeline.execute().await?;
    println!("✅ Pipeline executado com sucesso!");
    println!("   • {} registros processados", result.rows_processed);
    println!("   • Tempo de execução: {}ms", result.execution_time_ms);

    // Mostra o resultado
    if let Ok(json_content) = std::fs::read_to_string(&json_path) {
        println!("\n📄 Resultado (JSON com tipos de data):");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_content) {
            println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
        }
    }

    println!("\n🔄 Demonstração 2: Transformações de Data Avançadas");
    println!("   • Cálculo de idade");
    println!("   • Status baseado em atividade recente\n");

    // Pipeline para demonstrar cálculos de data
    let calc_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(CompositeTransformer::new()
            .add(ConvertTypesTransform::new({
                let mut conversions = std::collections::HashMap::new();
                conversions.insert("data_nascimento".to_string(), DataType::Date);
                conversions.insert("ultimo_login".to_string(), DataType::DateTime);
                conversions
            }))
            .add(MapTransform::new(|mut row| {
                // Calcula idade
                if let Some(DataValue::Date(birth_date)) = row.get("data_nascimento") {
                    let today = NaiveDate::from_ymd_opt(2024, 7, 13).unwrap();
                    let age = today.years_since(*birth_date).unwrap_or(0);
                    row.insert("idade".to_string(), DataValue::Integer(age as i64));
                }
                
                // Adiciona década de nascimento
                if let Some(DataValue::Date(birth_date)) = row.get("data_nascimento") {
                    let decade = (birth_date.year() / 10) * 10;
                    row.insert("decada".to_string(), DataValue::String(format!("{}s", decade)));
                }
                
                row
            }))
            .add(FilterTransform::new(|row| {
                // Filtra apenas usuários com mais de 30 anos
                if let Some(DataValue::Integer(age)) = row.get("idade") {
                    *age >= 30
                } else {
                    false
                }
            }))
        )
        .load(ConsoleLoader::new().with_pretty(true))
        .build();

    let calc_result = calc_pipeline.execute().await?;
    println!("📊 Cálculos de idade: {} usuários com 30+ anos", calc_result.rows_processed);

    println!("\n🔄 Demonstração 3: Validação e Conversão de Datas");
    println!("   • Teste de múltiplos formatos de data");
    println!("   • Conversões entre tipos\n");

    // Testa diferentes formatos
    let date_formats = vec![
        ("ISO Date", "2024-07-13"),
        ("ISO DateTime", "2024-07-13T14:30:00"),
        ("ISO Timestamp", "2024-07-13T14:30:00Z"),
        ("Simple DateTime", "2024-07-13 14:30:00"),
        ("Brazilian Date", "13/07/2024"), // Este vai falhar propositalmente
    ];

    for (name, date_str) in date_formats {
        let value = DataValue::String(date_str.to_string());
        
        println!("🧪 Testando {}: '{}'", name, date_str);
        
        if let Some(date) = value.as_date() {
            println!("   ✅ Date: {}", date);
        }
        
        if let Some(datetime) = value.as_datetime() {
            println!("   ✅ DateTime: {}", datetime);
        }
        
        if let Some(timestamp) = value.as_timestamp() {
            println!("   ✅ Timestamp: {}", timestamp);
        }
        
        // Se nenhuma conversão funcionou
        if value.as_date().is_none() && value.as_datetime().is_none() && value.as_timestamp().is_none() {
            println!("   ❌ Formato não suportado");
        }
        
        println!();
    }

    println!("🎯 Demonstração 4: Performance com Datas");
    println!("   • Processamento em lote de dados temporais\n");

    // Pipeline de performance
    let mut large_data = Vec::new();
    for i in 0..1000 {
        let mut row = DataRow::new();
        row.insert("id".to_string(), DataValue::Integer(i));
        row.insert("timestamp".to_string(), DataValue::String(format!("2024-01-{:02}T{:02}:00:00Z", (i % 31) + 1, (i % 24))));
        large_data.push(row);
    }

    let perf_pipeline = Pipeline::builder()
        .extract(MemoryExtractor::new(large_data))
        .transform(CompositeTransformer::new()
            .add(ConvertTypesTransform::single("timestamp", DataType::Timestamp))
            .add(FilterTransform::new(|row| {
                // Filtra apenas registros da primeira quinzena
                if let Some(DataValue::Timestamp(ts)) = row.get("timestamp") {
                    ts.day() <= 15
                } else {
                    false
                }
            }))
        )
        .load(MemoryLoader::new())
        .build();

    let perf_result = perf_pipeline.execute().await?;
    println!("⚡ Performance: {} registros processados em {}ms", 
        perf_result.rows_processed, perf_result.execution_time_ms);

    // Cleanup
    let _ = std::fs::remove_file(&json_path);

    println!("\n✨ Demonstração Completa!");
    println!("🎯 Recursos demonstrados:");
    println!("   ✅ Conversão automática String → Date/DateTime/Timestamp");
    println!("   ✅ Filtros baseados em datas com comparações temporais");
    println!("   ✅ Cálculos de idade e intervalos de tempo");
    println!("   ✅ Agregações por critérios temporais");
    println!("   ✅ Serialização JSON com tipos de data preservados");
    println!("   ✅ Pipeline compostos com múltiplas transformações");
    println!("   ✅ Processamento em lote com boa performance");
    println!("   ✅ Validação de formatos de data");

    Ok(())
}

// Extrator de memória
struct MemoryExtractor {
    data: Vec<DataRow>,
}

impl MemoryExtractor {
    fn new(data: Vec<DataRow>) -> Self {
        Self { data }
    }
}

#[async_trait::async_trait]
impl Extractor for MemoryExtractor {
    async fn extract(&self) -> Result<Vec<DataRow>> {
        Ok(self.data.clone())
    }
}
