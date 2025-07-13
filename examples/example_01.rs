use etlrs::prelude::*;
use etlrs::transform::common::DataType;
use std::collections::HashMap;

/// Exemplo avançado com múltiplas transformações e configurações
#[tokio::main]
async fn main() -> Result<()> {
    // Configura logging estruturado
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .init();
    
    println!("🚀 Exemplo avançado de pipeline ETL com ETLRS");
    println!("============================================");
    
    // Configuração personalizada
    let config = ETLConfig::builder()
        .batch_size(100)
        .parallel_workers(4)
        .timeout_seconds(300)
        .enable_metrics(true)
        .enable_logging(true)
        .log_level("info")
        .memory_limit_mb(512)
        .build()?;
    
    // Pipeline com múltiplas transformações
    let pipeline = Pipeline::with_config(config)
        .extract(CsvExtractor::new("examples/data/sales.csv"))
        .transform(
            // Cadeia de transformações
            ChainedTransform::new(vec![
                // 1. Filtra registros válidos
                Box::new(FilterTransform::new(|row| {
                    let amount = row.get("amount")
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.0);
                    
                    let product = row.get("product")
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    
                    amount > 0.0 && !product.is_empty()
                })),
                
                // 2. Adiciona campos calculados
                Box::new(MapTransform::new(|mut row| {
                    // Calcula desconto baseado no valor
                    if let Some(DataValue::Float(amount)) = row.get("amount").cloned() {
                        let discount = if amount > 1000.0 { 0.1 } else { 0.0 };
                        row.insert("discount".to_string(), DataValue::Float(discount));
                        row.insert("final_amount".to_string(), DataValue::Float(amount * (1.0 - discount)));
                    }
                    
                    // Adiciona timestamp de processamento
                    row.insert("processed_at".to_string(), 
                        DataValue::String(chrono::Utc::now().to_rfc3339()));
                    
                    row
                })),
                
                // 3. Converte tipos
                Box::new(ConvertTypesTransform::new({
                    let mut conversions = HashMap::new();
                    conversions.insert("customer_id".to_string(), DataType::Integer);
                    conversions.insert("product".to_string(), DataType::String);
                    conversions
                })),
                
                // 4. Renomeia colunas
                Box::new(RenameColumnsTransform::new({
                    let mut mappings = HashMap::new();
                    mappings.insert("customer_id".to_string(), "client_id".to_string());
                    mappings.insert("final_amount".to_string(), "total_value".to_string());
                    mappings
                })),
            ])
        )
        .load(MultiLoader::new(vec![
            // Salva em JSON
            Box::new(JsonLoader::new("examples/output/processed_sales.json").with_pretty(true)),
            
            // Salva em JSONL
            Box::new(JsonLinesLoader::new("examples/output/processed_sales.jsonl")),
            
            // Mostra no console
            Box::new(ConsoleLoader::new().with_pretty(false)),
        ]))
        .build();
    
    // Executa o pipeline
    println!("📊 Executando pipeline avançado...");
    let result = pipeline.execute().await?;
    
    // Relatório detalhado
    println!("\n📈 Relatório de Execução:");
    println!("========================");
    println!("✅ Registros processados: {}", result.rows_processed);
    println!("✅ Registros bem-sucedidos: {}", result.rows_successful);
    println!("❌ Registros com falha: {}", result.rows_failed);
    println!("⏱️  Tempo de execução: {}ms", result.execution_time_ms);
    println!("📊 Taxa de sucesso: {:.2}%", result.success_rate() * 100.0);
    
    if result.has_errors() {
        println!("\n⚠️  Erros encontrados:");
        for (i, error) in result.errors.iter().enumerate() {
            println!("   {}. {}", i + 1, error);
        }
    }
    
    // Métricas detalhadas
    let metrics = pipeline.get_metrics().await;
    println!("\n📊 Métricas Detalhadas:");
    println!("======================");
    println!("🔢 Total de execuções: {}", metrics.executions.len());
    println!("📝 Total de registros processados: {}", metrics.total_rows_processed);
    println!("⏱️  Tempo total de execução: {}ms", metrics.total_execution_time_ms);
    println!("💯 Taxa de sucesso geral: {:.2}%", metrics.success_rate * 100.0);
    
    if let Some(last_execution) = metrics.executions.last() {
        println!("📅 Última execução: {:?}", last_execution.timestamp);
        println!("⚙️  Configuração usada:");
        println!("   - Batch size: {}", last_execution.config_snapshot.pipeline.batch_size);
        println!("   - Workers: {}", last_execution.config_snapshot.pipeline.parallel_workers);
        println!("   - Timeout: {}s", last_execution.config_snapshot.pipeline.timeout_seconds);
    }
    
    // Throughput
    if result.execution_time_ms > 0 {
        let throughput = (result.rows_processed as f64 / result.execution_time_ms as f64) * 1000.0;
        println!("🚀 Throughput: {:.2} registros/segundo", throughput);
    }
    
    println!("\n🎯 Arquivos de saída criados:");
    println!("   - examples/output/processed_sales.json");
    println!("   - examples/output/processed_sales.jsonl");
    
    Ok(())
}

/// Transformador que encadeia múltiplas transformações
struct ChainedTransform {
    transforms: Vec<Box<dyn Transformer>>,
}

impl ChainedTransform {
    fn new(transforms: Vec<Box<dyn Transformer>>) -> Self {
        Self { transforms }
    }
}

#[async_trait::async_trait]
impl Transformer for ChainedTransform {
    async fn transform(&self, mut data: Vec<DataRow>) -> Result<Vec<DataRow>> {
        for transform in &self.transforms {
            data = transform.transform(data).await?;
        }
        Ok(data)
    }
}

/// Loader que escreve em múltiplos destinos
struct MultiLoader {
    loaders: Vec<Box<dyn Loader>>,
}

impl MultiLoader {
    fn new(loaders: Vec<Box<dyn Loader>>) -> Self {
        Self { loaders }
    }
}

#[async_trait::async_trait]
impl Loader for MultiLoader {
    async fn load(&self, data: Vec<DataRow>) -> Result<PipelineResult> {
        let mut final_result = PipelineResult::new();
        
        for loader in &self.loaders {
            let result = loader.load(data.clone()).await?;
            final_result.rows_processed = result.rows_processed;
            final_result.rows_successful += result.rows_successful;
            final_result.rows_failed += result.rows_failed;
            final_result.errors.extend(result.errors);
        }
        
        Ok(final_result)
    }
    
    async fn finalize(&self) -> Result<()> {
        for loader in &self.loaders {
            loader.finalize().await?;
        }
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        for loader in &self.loaders {
            if !loader.health_check().await? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Cria dados de exemplo para vendas
#[allow(dead_code)]
async fn create_sample_sales_data() -> Result<()> {
    use std::fs;
    use std::io::Write;
    
    fs::create_dir_all("examples/data")?;
    fs::create_dir_all("examples/output")?;
    
    let mut file = fs::File::create("examples/data/sales.csv")?;
    writeln!(file, "id,customer_id,product,amount,date")?;
    writeln!(file, "1,1001,Laptop,1500.00,2023-01-15")?;
    writeln!(file, "2,1002,Mouse,25.50,2023-01-16")?;
    writeln!(file, "3,1003,Keyboard,75.00,2023-01-17")?;
    writeln!(file, "4,1001,Monitor,300.00,2023-01-18")?;
    writeln!(file, "5,1004,Tablet,450.00,2023-01-19")?;
    writeln!(file, "6,1002,Headphones,120.00,2023-01-20")?;
    writeln!(file, "7,1005,Smartphone,800.00,2023-01-21")?;
    writeln!(file, "8,1003,Webcam,60.00,2023-01-22")?;
    writeln!(file, "9,1004,Printer,250.00,2023-01-23")?;
    writeln!(file, "10,1006,Speaker,85.00,2023-01-24")?;
    
    println!("📁 Dados de exemplo de vendas criados em examples/data/sales.csv");
    
    Ok(())
}
