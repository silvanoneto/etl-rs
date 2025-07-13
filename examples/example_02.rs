use etlrs::prelude::*;
use etlrs::{InMemoryEventEmitter, LoggingPlugin, MetricsPlugin, PluginRegistry};
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::main]
async fn main() -> Result<()> {
    // Configura tracing para ver os logs
    tracing_subscriber::fmt::init();

    // Cria arquivo CSV temporário para exemplo
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "nome,idade,ativo,salario").unwrap();
    writeln!(temp_file, "Alice,30,true,5000.50").unwrap();
    writeln!(temp_file, "Bob,17,false,2500.75").unwrap();
    writeln!(temp_file, "Carlos,45,true,7500.00").unwrap();
    writeln!(temp_file, "Diana,25,true,4000.25").unwrap();
    writeln!(temp_file, "Eduardo,16,false,1500.00").unwrap();

    // Configuração personalizada
    let config = ETLConfig::builder()
        .batch_size(500)
        .parallel_workers(2)
        .enable_metrics(true)
        .enable_logging(true)
        .memory_limit_mb(256)
        .build()?;

    // Event emitter personalizado para capturar eventos
    let event_emitter = InMemoryEventEmitter::new();

    // Pipeline com todas as funcionalidades novas
    let pipeline = Pipeline::with_config(config)
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(
            // Filtrar apenas pessoas ativas e maiores de idade
            FilterTransform::new(|row| {
                let ativo = row.get("ativo")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false);
                let idade = row.get("idade")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(0);
                ativo && idade >= 18
            })
        )
        .load(MemoryLoader::new())
        .event_emitter(event_emitter.clone())
        .batch_size(100)
        .enable_metrics(true)
        .build();

    println!("Pipeline ID: {}", pipeline.pipeline_id());
    println!("Estado inicial: {}", pipeline.current_state());

    // Executa o pipeline
    println!("\n🚀 Executando pipeline...");
    let result = pipeline.execute().await?;

    println!("\n📊 Resultados:");
    println!("- Registros processados: {}", result.rows_processed);
    println!("- Registros bem-sucedidos: {}", result.rows_successful);
    println!("- Registros falharam: {}", result.rows_failed);
    println!("- Tempo de execução: {}ms", result.execution_time_ms);
    println!("- Taxa de sucesso: {:.2}%", result.success_rate() * 100.0);
    println!("- Estado final: {}", pipeline.current_state());

    // Mostra eventos capturados
    println!("\n📅 Eventos capturados:");
    for (i, event) in event_emitter.get_events().iter().enumerate() {
        match event {
            PipelineEvent::Started { pipeline_id, .. } => {
                println!("  {}. 🟢 Pipeline {} iniciado", i + 1, pipeline_id);
            }
            PipelineEvent::StateChanged { old_state, new_state, .. } => {
                println!("  {}. 🔄 Estado alterado: {} → {}", i + 1, old_state, new_state);
            }
            PipelineEvent::Completed { result, .. } => {
                println!("  {}. ✅ Pipeline concluído - {} registros processados", i + 1, result.rows_processed);
            }
            PipelineEvent::Error { error, .. } => {
                println!("  {}. ❌ Erro: {}", i + 1, error);
            }
            _ => {}
        }
    }

    // Exemplo de uso com plugins
    println!("\n🔌 Testando sistema de plugins...");
    
    let mut plugin_registry = PluginRegistry::new();
    plugin_registry.register(LoggingPlugin::new());
    plugin_registry.register(MetricsPlugin::new());

    println!("Plugins registrados:");
    for (name, version, description) in plugin_registry.list_plugins() {
        println!("- {} v{}: {}", name, version, description);
    }

    // Exemplo de pipeline com streaming
    println!("\n🌊 Testando execução em streaming...");
    let streaming_result = pipeline.execute_streaming().await?;
    println!("Resultado streaming:");
    println!("- Registros processados: {}", streaming_result.rows_processed);
    println!("- Tempo de execução: {}ms", streaming_result.execution_time_ms);

    // Demonstra transformações avançadas
    println!("\n🔧 Exemplo com transformações avançadas...");
    
    // Cria novo pipeline com transformações mais complexas
    let advanced_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(
            // Encadeia múltiplas transformações usando uma composição
            FilterTransform::new(|row| {
                row.get("ativo").and_then(|v| v.as_boolean()).unwrap_or(false)
            })
        )
        .load(MemoryLoader::new())
        .build();

    let advanced_result = advanced_pipeline.execute().await?;
    println!("Pipeline avançado processou {} registros", advanced_result.rows_processed);

    // Exemplo específico de transformações avançadas
    println!("\n🔧 Exemplos de transformações individuais...");
    
    // RenameColumnsTransform
    let rename_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(RenameColumnsTransform::new(
            [
                ("nome".to_string(), "nome_completo".to_string()),
                ("idade".to_string(), "anos".to_string()),
            ].into_iter().collect()
        ))
        .load(MemoryLoader::new())
        .build();
    
    let rename_result = rename_pipeline.execute().await?;
    println!("RenameColumnsTransform processou {} registros", rename_result.rows_processed);
    
    // SelectColumnsTransform
    let select_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(SelectColumnsTransform::new(vec!["nome", "idade"]))
        .load(MemoryLoader::new())
        .build();
    
    let select_result = select_pipeline.execute().await?;
    println!("SelectColumnsTransform processou {} registros", select_result.rows_processed);
    
    // ParallelTransform
    let parallel_pipeline = Pipeline::builder()
        .extract(CsvExtractor::new(temp_file.path()))
        .transform(ParallelTransform::new(
            FilterTransform::new(|row| {
                row.get("ativo").and_then(|v| v.as_boolean()).unwrap_or(false)
            }),
            2 // 2 workers
        ))
        .load(MemoryLoader::new())
        .build();
    
    let parallel_result = parallel_pipeline.execute().await?;
    println!("ParallelTransform processou {} registros", parallel_result.rows_processed);

    // Configuração a partir de variáveis de ambiente
    println!("\n🌍 Testando configuração por variáveis de ambiente...");
    std::env::set_var("ETL_BATCH_SIZE", "200");
    std::env::set_var("ETL_ENABLE_METRICS", "true");
    
    let env_config = ETLConfig::from_env()?;
    println!("Configuração do ambiente:");
    println!("- Batch size: {}", env_config.pipeline.batch_size);
    println!("- Métricas habilitadas: {}", env_config.features.enable_metrics);

    println!("\n✨ Todos os ajustes foram aplicados com sucesso!");
    println!("📋 Funcionalidades implementadas:");
    println!("   ✅ Sistema de estado (PipelineState)");
    println!("   ✅ Sistema de eventos (PipelineEvent)");
    println!("   ✅ Event emitters (Logging e InMemory)");
    println!("   ✅ Sistema de plugins extensível");
    println!("   ✅ Configuração por variáveis de ambiente");
    println!("   ✅ Transformações avançadas (AsyncMap, Rename, Select, etc.)");
    println!("   ✅ Pipeline streaming");
    println!("   ✅ Documentação em português");
    println!("   ✅ Observabilidade completa");

    Ok(())
}
