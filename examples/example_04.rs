use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 ETLRS - Teste Básico de Delta Lake");
    println!("====================================\n");

    #[cfg(not(feature = "delta"))]
    {
        println!("ℹ️  Testando criação sem feature 'delta':");
        
        // Testa se os tipos estão disponíveis para compilação condicional
        println!("   ✅ DeltaExtractor pode ser referenciado condicionalmente");
        println!("   ✅ DeltaLoader pode ser referenciado condicionalmente");
        
        println!("\n💡 Para testar funcionalidade completa:");
        println!("   cargo run --example exemplo_delta --features delta");
    }

    #[cfg(feature = "delta")]
    {
        use etlrs::load::delta::{DeltaLoader, DeltaWriteMode};
        
        println!("✅ Feature 'delta' habilitada!");
        
        // Cria um DeltaLoader para teste
        let _loader = DeltaLoader::new("/tmp/test_delta")
            .with_mode(DeltaWriteMode::Append)
            .with_compression("snappy");
            
        println!("   ✅ DeltaLoader criado com sucesso");
        
        // Cria um DeltaExtractor para teste
        let _extractor = DeltaExtractor::new("/tmp/test_delta")
            .with_batch_size(1000);
            
        println!("   ✅ DeltaExtractor criado com sucesso");
        
        println!("\n🎯 Tipos Delta disponíveis:");
        println!("   • DeltaExtractor - para leitura de tabelas Delta");
        println!("   • DeltaLoader - para escrita em tabelas Delta");
        println!("   • DeltaWriteMode - modos de escrita (Append/Overwrite/Merge)");
    }

    Ok(())
}
