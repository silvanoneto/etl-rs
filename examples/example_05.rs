use etlrs::extract::parquet::ParquetExtractor;
use etlrs::load::parquet::{ParquetLoader, CompressionType};
use etlrs::traits::{Extractor, Loader};
use etlrs::types::DataValue;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Inicializar logger
    tracing_subscriber::fmt::init();

    println!("🚀 Testando suporte Parquet com Arrow/Parquet 55.2.0");

    // Criar dados de teste
    let mut dados_teste = Vec::new();
    
    for i in 1..=5 {
        let mut row = HashMap::new();
        row.insert("id".to_string(), DataValue::Integer(i));
        row.insert("nome".to_string(), DataValue::String(format!("Usuario_{}", i)));
        row.insert("idade".to_string(), DataValue::Integer(20 + i));
        row.insert("ativo".to_string(), DataValue::Boolean(i % 2 == 0));
        row.insert("salario".to_string(), DataValue::Float(1000.0 + (i as f64) * 500.0));
        dados_teste.push(row);
    }

    println!("📝 Criados {} registros de teste", dados_teste.len());

    // Teste 1: Salvar arquivo Parquet
    let arquivo_saida = "examples/output/test_parquet_55.parquet";
    
    let loader = ParquetLoader::new(arquivo_saida)?
        .with_compression(CompressionType::Snappy)
        .with_overwrite(true)
        .with_metadata("created_by", "ETLRS Test")
        .with_metadata("arrow_version", "55.2.0");

    println!("💾 Salvando dados em Parquet...");
    let resultado_save = loader.load(dados_teste.clone()).await?;
    
    println!("✅ Arquivo Parquet salvo com sucesso!");
    println!("   📊 Registros processados: {}", resultado_save.rows_processed);
    println!("   ✔️  Registros bem-sucedidos: {}", resultado_save.rows_successful);
    println!("   ⏱️  Tempo de execução: {}ms", resultado_save.execution_time_ms);

    // Teste 2: Ler arquivo Parquet
    let extractor = ParquetExtractor::new(arquivo_saida)?
        .with_columns(vec!["id".to_string(), "nome".to_string(), "salario".to_string()])
        .with_batch_size(1000);

    println!("\n📖 Lendo dados do Parquet...");
    let dados_lidos = extractor.extract().await?;
    
    println!("✅ Arquivo Parquet lido com sucesso!");
    println!("   📊 Registros extraídos: {}", dados_lidos.len());
    
    // Mostrar alguns dados lidos
    println!("\n📋 Primeiros registros extraídos:");
    for (i, row) in dados_lidos.iter().take(3).enumerate() {
        println!("   Registro {}: {:?}", i + 1, row);
    }

    // Teste 3: Diferentes compressões
    println!("\n🗜️ Testando diferentes algoritmos de compressão...");
    
    let compressoes = vec![
        ("uncompressed", CompressionType::Uncompressed),
        ("snappy", CompressionType::Snappy),
        ("gzip", CompressionType::Gzip),
        ("brotli", CompressionType::Brotli),
        ("zstd", CompressionType::Zstd),
    ];

    for (nome, compressao) in compressoes {
        let arquivo = format!("examples/output/test_{}.parquet", nome);
        let loader = ParquetLoader::new(&arquivo)?
            .with_compression(compressao)
            .with_overwrite(true);
            
        let resultado = loader.load(dados_teste.clone()).await?;
        
        // Verificar tamanho do arquivo
        let tamanho = std::fs::metadata(&arquivo)?.len();
        
        println!("   {} -> {} bytes ({}ms)", 
                 nome, tamanho, resultado.execution_time_ms);
    }

    println!("\n🎉 Todos os testes de Parquet foram bem-sucedidos!");
    println!("   ✅ Arrow/Parquet 55.2.0 está funcionando perfeitamente");
    println!("   ✅ Extração e carregamento funcionais");
    println!("   ✅ Suporte a múltiplas compressões");
    println!("   ✅ Inferência automática de schema");
    println!("   ✅ Projeção de colunas");

    Ok(())
}
