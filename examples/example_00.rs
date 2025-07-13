use etlrs::prelude::*;

/// Exemplo básico de pipeline CSV para JSON
#[tokio::main]
async fn main() -> Result<()> {
    // Configura logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 Exemplo básico de pipeline ETL com ETLRS");
    println!("==========================================");
    
    // Cria um pipeline simples: CSV → Filtro → JSON
    let pipeline = Pipeline::builder()
        .extract(CsvExtractor::new("examples/data/users.csv"))
        .transform(FilterTransform::new(|row| {
            // Filtra apenas usuários ativos com idade >= 18
            let is_active = row.get("active")
                .and_then(|v| v.as_boolean())
                .unwrap_or(false);
            
            let age = row.get("age")
                .and_then(|v| v.as_integer())
                .unwrap_or(0);
            
            is_active && age >= 18
        }))
        .load(JsonLoader::new("examples/output/filtered_users.json").with_pretty(true))
        .enable_metrics(true)
        .enable_logging(true)
        .build();
    
    // Executa o pipeline
    println!("📊 Executando pipeline...");
    let result = pipeline.execute().await?;
    
    // Mostra resultados
    println!("✅ Pipeline executado com sucesso!");
    println!("   - Registros processados: {}", result.rows_processed);
    println!("   - Registros bem-sucedidos: {}", result.rows_successful);
    println!("   - Registros com falha: {}", result.rows_failed);
    println!("   - Tempo de execução: {}ms", result.execution_time_ms);
    println!("   - Taxa de sucesso: {:.2}%", result.success_rate() * 100.0);
    
    if result.has_errors() {
        println!("⚠️  Erros encontrados:");
        for error in &result.errors {
            println!("   - {}", error);
        }
    }
    
    // Obtém métricas do pipeline
    let metrics = pipeline.get_metrics().await;
    println!("📈 Métricas do pipeline:");
    println!("   - Total de execuções: {}", metrics.executions.len());
    println!("   - Total de registros processados: {}", metrics.total_rows_processed);
    println!("   - Tempo total de execução: {}ms", metrics.total_execution_time_ms);
    println!("   - Taxa de sucesso geral: {:.2}%", metrics.success_rate * 100.0);
    
    println!("🎯 Arquivo de saída criado: examples/output/filtered_users.json");
    
    Ok(())
}

/// Cria dados de exemplo se não existirem
#[allow(dead_code)]
async fn create_sample_data() -> Result<()> {
    use std::fs;
    use std::io::Write;
    
    // Cria diretório de dados se não existir
    fs::create_dir_all("examples/data")?;
    fs::create_dir_all("examples/output")?;
    
    // Cria arquivo CSV de exemplo
    let mut file = fs::File::create("examples/data/users.csv")?;
    writeln!(file, "id,name,age,active,email")?;
    writeln!(file, "1,Alice Silva,28,true,alice@email.com")?;
    writeln!(file, "2,Bob Santos,17,true,bob@email.com")?;
    writeln!(file, "3,Carol Oliveira,34,false,carol@email.com")?;
    writeln!(file, "4,David Lima,25,true,david@email.com")?;
    writeln!(file, "5,Eva Costa,16,false,eva@email.com")?;
    writeln!(file, "6,Felipe Souza,42,true,felipe@email.com")?;
    writeln!(file, "7,Gisele Ferreira,30,true,gisele@email.com")?;
    writeln!(file, "8,Hugo Pereira,19,false,hugo@email.com")?;
    writeln!(file, "9,Isabela Rodrigues,27,true,isabela@email.com")?;
    writeln!(file, "10,João Martins,15,true,joao@email.com")?;
    
    println!("📁 Dados de exemplo criados em examples/data/users.csv");
    
    Ok(())
}
