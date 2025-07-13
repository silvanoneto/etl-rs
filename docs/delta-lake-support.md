# Suporte Delta Lake no ETLRS

## Visão Geral

O ETLRS oferece suporte nativo ao formato Delta Lake através da feature opcional `delta`. Esta funcionalidade permite extrair e carregar dados de/para tabelas Delta Lake com todas as vantagens ACID que o formato oferece.

## Características

### Delta Extractor
- **Time Travel**: Acesso a versões históricas das tabelas
- **Predicates**: Filtragem eficiente durante a leitura
- **Batch Processing**: Processamento em lotes para grandes volumes
- **Schema Evolution**: Suporte a evolução de schema

### Delta Loader
- **Modos de Escrita**: Append, Overwrite, Merge
- **Transações ACID**: Garantia de integridade de dados
- **Particionamento**: Suporte a tabelas particionadas
- **Otimizações**: Compactação e otimização automática

## Habilitando Delta Lake

### No Cargo.toml

```toml
[dependencies]
etlrs = { version = "0.1", features = ["delta"] }
```

### Ou via linha de comando

```bash
cargo run --features delta
```

## Exemplos de Uso

### Extraindo Dados

```rust
use etlrs::extract::DeltaExtractor;

#[cfg(feature = "delta")]
async fn extrair_dados() -> Result<(), Box<dyn std::error::Error>> {
    let extractor = DeltaExtractor::new("./data/tabela_delta")?;
    
    // Extração básica
    let dados = extractor.extract().await?;
    
    // Time travel - versão específica
    let dados_v1 = extractor.extract_version(1).await?;
    
    // Filtragem com predicates
    let dados_filtrados = extractor
        .with_predicate("idade > 18")
        .extract()
        .await?;
    
    Ok(())
}
```

### Carregando Dados

```rust
use etlrs::load::{DeltaLoader, WriteMode};

#[cfg(feature = "delta")]
async fn carregar_dados() -> Result<(), Box<dyn std::error::Error>> {
    let mut loader = DeltaLoader::new("./data/tabela_saida")?;
    
    // Append mode
    loader.set_mode(WriteMode::Append);
    loader.load(dados).await?;
    
    // Overwrite mode
    loader.set_mode(WriteMode::Overwrite);
    loader.load(dados).await?;
    
    Ok(())
}
```

### Pipeline Completo

```rust
use etlrs::{
    extract::DeltaExtractor,
    load::DeltaLoader,
    pipeline::Pipeline,
    transform::FilterTransform,
};

#[cfg(feature = "delta")]
async fn pipeline_delta() -> Result<(), Box<dyn std::error::Error>> {
    // Configurar extrator
    let extractor = DeltaExtractor::new("./input/tabela_fonte")?
        .with_predicate("data >= '2024-01-01'");
    
    // Configurar transformações
    let transform = FilterTransform::new(|row| {
        row.get("status").map_or(false, |v| v == "ativo")
    });
    
    // Configurar loader
    let loader = DeltaLoader::new("./output/tabela_destino")?;
    
    // Executar pipeline
    let mut pipeline = Pipeline::new()
        .with_extractor(Box::new(extractor))
        .with_transform(Box::new(transform))
        .with_loader(Box::new(loader));
    
    pipeline.execute().await?;
    
    Ok(())
}
```

## Compilação Condicional

O suporte Delta Lake é implementado usando compilação condicional, permitindo que o código seja escrito de forma segura:

```rust
#[cfg(feature = "delta")]
use etlrs::extract::DeltaExtractor;

#[cfg(feature = "delta")]
fn usar_delta() {
    // Código específico para Delta Lake
}

#[cfg(not(feature = "delta"))]
fn usar_alternativo() {
    // Código alternativo quando Delta não está disponível
}
```

## Requisitos

- **Rust**: 1.70+
- **Features**: Requer as features `arrow` e `parquet`
- **Sistema**: Suporte a sistemas Unix/Linux/Windows

## Dependências

Quando a feature `delta` é habilitada, as seguintes dependências são incluídas:

- `deltalake`: 0.19+ (core Delta Lake)
- `arrow`: 51.0+ (formato Arrow)
- `parquet`: 50.0+ (formato Parquet)

## Performance

### Otimizações Implementadas

1. **Lazy Loading**: Carregamento sob demanda
2. **Column Pruning**: Leitura apenas das colunas necessárias
3. **Predicate Pushdown**: Filtragem no nível de arquivo
4. **Batch Processing**: Processamento em lotes configuráveis

### Benchmarks

Os benchmarks estão disponíveis em `benches/pipeline_benchmark.rs`:

```bash
cargo bench --features delta
```

## Resolução de Problemas

### Erro de Compilação

Se encontrar erros de compilação com Arrow, certifique-se de que:

1. A versão do Rust está atualizada
2. As features necessárias estão habilitadas
3. O cache do Cargo está limpo: `cargo clean`

### Problemas de Performance

Para melhor performance:

1. Use predicates para reduzir dados lidos
2. Configure batch_size apropriado
3. Considere particionamento das tabelas
4. Monitore uso de memória

## Exemplos Completos

Veja os exemplos na pasta `examples/`:

- `exemplo_delta.rs`: Exemplo completo com todas as funcionalidades
- `teste_delta_basico.rs`: Teste básico de compatibilidade

## Roadmap

### Próximas Funcionalidades

- [ ] Merge operations mais avançadas
- [ ] Suporte a Z-ordering
- [ ] Vacuum automático
- [ ] Métricas detalhadas
- [ ] Streaming writes

### Melhorias de Performance

- [ ] Otimização de memory usage
- [ ] Paralelização de writes
- [ ] Compressão adaptativa
- [ ] Cache inteligente

## Contribuindo

Para contribuir com o suporte Delta Lake:

1. Certifique-se de que os testes passam com e sem a feature
2. Adicione testes para novas funcionalidades
3. Documente mudanças de API
4. Atualize benchmarks quando aplicável

## Licença

O suporte Delta Lake segue a mesma licença do projeto principal (Apache 2.0).
