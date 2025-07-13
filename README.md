# 🚀 ETLRS - Framework ETL Moderno em Rust

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: Apache](https://img.shields.io/badge/License-Apache-yellow.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/v/etlrs.svg)](https://crates.io/crates/etlrs)

Uma biblioteca ETL moderna, segura e de alta performance construída em Rust, com foco em type-safety, observabilidade e facilidade de uso para processamento de dados.

## ✨ Características Principais

- 🚄 **Alta Performance**: Processamento assíncrono com Tokio e paralelização
- 🔒 **Type Safety**: Sistema de tipos rigoroso com validação em tempo de compilação  
- � **Múltiplos Formatos**: CSV, JSON, JSON Lines, Parquet (opcional)
- 🔄 **Processamento em Lotes**: Suporte para datasets grandes com batch processing
- � **API Fluente**: Builder pattern intuitivo para construção de pipelines
- 📈 **Observabilidade**: Métricas integradas, eventos e logging estruturado
- �️ **Extensível**: Sistema de traits para componentes customizados
- 🧪 **Testável**: Componentes de teste (MemoryLoader, ConsoleLoader)

## 📚 Documentação

### 🎯 Início Rápido
- [Visão Geral do Sistema](./docs/context/01-visao-geral.md) - Introdução e objetivos do ETLRS
- [Estrutura do Projeto](./docs/context/02-estrutura-projeto.md) - Organização e componentes
- [Dependências e Setup](./docs/context/03-dependencias.md) - Configuração inicial

### 🏗️ Arquitetura e Design  
- [Arquitetura Principal](./docs/context/04-arquitetura-principal.md) - Componentes core
- [Framework de Integridade](./docs/context/05-framework-integridade.md) - Validação e qualidade
- [Recursos Avançados](./docs/context/06-recursos-avancados.md) - Features enterprise

### � Guias Técnicos
- [Guia de Performance](./docs/performance.md) - Otimização e tuning
- [Troubleshooting](./docs/troubleshooting.md) - Resolução de problemas
- [Manutenção e Gestão](./docs/context/07-manutencao-gestao.md) - Operações contínuas

## 🚀 Quick Start

### Instalação

```toml
[dependencies]
etlrs = "0.1.0"

# Para suporte Parquet
etlrs = { version = "0.1.0", features = ["parquet"] }

# Para suporte CSV
etlrs = { version = "0.1.0", features = ["csv"] }
```

### Exemplo Básico

```rust
use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Configura logging
    tracing_subscriber::fmt::init();
    
    // Criar pipeline ETL simples
    let pipeline = Pipeline::builder()
        .extract(CsvExtractor::new("data/users.csv"))
        .transform(FilterTransform::new(|row| {
            row.get("age")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) >= 18
        }))
        .load(JsonLoader::new("data/adults.json").with_pretty(true))
        .build();
    
    // Executar pipeline
    let result = pipeline.execute().await?;
    
    println!("Pipeline executado com sucesso!");
    println!("Registros processados: {}", result.rows_processed);
    println!("Registros bem-sucedidos: {}", result.rows_successful);
    println!("Tempo de execução: {}ms", result.execution_time_ms);
    
    Ok(())
}
```

### Exemplo com Múltiplas Transformações

```rust
use etlrs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let pipeline = Pipeline::builder()
        .extract(CsvExtractor::new("sales.csv"))
        .transform(CompositeTransformer::new()
            // Filtrar vendas acima de R$ 100
            .add(FilterTransform::new(|row| {
                row.get("amount")
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.0) > 100.0
            }))
            // Adicionar coluna calculada
            .add(MapTransform::new(|mut row| {
                if let Some(amount) = row.get("amount").and_then(|v| v.as_float()) {
                    row.insert("tax".to_string(), DataValue::Float(amount * 0.1));
                }
                row
            }))
            // Renomear colunas
            .add(RenameColumnsTransform::new(
                [("amount".to_string(), "valor".to_string())]
                    .into_iter().collect()
            ))
        )
        .load(JsonLinesLoader::new("vendas_processadas.jsonl"))
        .build();

    let result = pipeline.execute().await?;
    println!("Processadas {} vendas", result.rows_processed);
    
    Ok(())
}
```

## 🛠️ Instalação e Setup

### Pré-requisitos

- Rust 1.75+ (stable)
- Cargo

### Clone e Build

```bash
# Clone o repositório
git clone https://github.com/silvanoneto/etlrs.git
cd etlrs

# Build do projeto
cargo build --release

# Executar testes
cargo test

# Executar testes com features
cargo test --features parquet
```

### Features Disponíveis

```toml
[dependencies]
etlrs = { version = "0.1.0", features = ["csv", "parquet"] }

# Features suportadas:
# - csv: Suporte para arquivos CSV
# - parquet: Suporte para arquivos Parquet (Apache Arrow 55.2.0)
# - delta: Suporte para Delta Lake (experimental)
```

## 📊 Componentes Disponíveis

### 🔌 Extractors (Extração)
- **CsvExtractor**: Leitura de arquivos CSV com configuração flexível
- **JsonExtractor**: Leitura de arquivos JSON e JSON Lines
- **ParquetExtractor**: Leitura de arquivos Parquet (feature `parquet`)

### 🔄 Transformers (Transformação)
- **FilterTransform**: Filtragem de dados com predicados customizados
- **MapTransform**: Transformação de dados linha por linha
- **AsyncMapTransform**: Transformação assíncrona para operações I/O
- **AddColumnTransform**: Adição de colunas com valores constantes
- **RemoveColumnsTransform**: Remoção de colunas específicas
- **SelectColumnsTransform**: Seleção de colunas específicas
- **RenameColumnsTransform**: Renomeação de colunas
- **ConvertTypesTransform**: Conversão de tipos de dados
- **AggregateTransform**: Agregações (count, sum, avg, min, max)
- **CompositeTransformer**: Combinação de múltiplas transformações
- **ParallelTransform**: Processamento paralelo

### 📁 Loaders (Carregamento)
- **JsonLoader**: Escrita para arquivos JSON
- **JsonLinesLoader**: Escrita para arquivos JSON Lines
- **ConsoleLoader**: Output para console (útil para debug)
- **MemoryLoader**: Armazenamento em memória (útil para testes)
- **ParquetLoader**: Escrita para arquivos Parquet (feature `parquet`)

### 🎯 Casos de Uso Implementados

#### 1. Processamento de CSV para JSON
```rust
let pipeline = Pipeline::builder()
    .extract(CsvExtractor::new("input.csv"))
    .transform(FilterTransform::new(|row| {
        row.get("status") == Some(&DataValue::String("active".to_string()))
    }))
    .load(JsonLoader::new("output.json").with_pretty(true))
    .build();
```

#### 2. Agregação de Dados
```rust
let pipeline = Pipeline::builder()
    .extract(CsvExtractor::new("sales.csv"))
    .transform(AggregateTransform::new(
        vec!["product_category".to_string()], // group by
        [("amount".to_string(), AggregateFunction::Sum)].into()
    ))
    .load(JsonLoader::new("summary.json"))
    .build();
```

#### 3. Processamento em Lotes
```rust
// Para datasets grandes
let result = pipeline.execute_batch(1000).await?;
```

#### 4. Multiple Outputs
```rust
// Usando exemplo de MultiLoader personalizado
let multi_loader = MultiLoader::new(vec![
    Box::new(JsonLoader::new("output.json")),
    Box::new(ConsoleLoader::new()),
    Box::new(MemoryLoader::new()),
]);
```

## 🧪 Testing

```bash
# Executar todos os testes
cargo test

# Testes com features específicas
cargo test --features parquet

# Testes com output verbose
cargo test -- --nocapture

# Executar exemplos
cargo run --example basic_pipeline
cargo run --example advanced_pipeline
```

## 🎯 Benchmarks

```bash
# Executar benchmarks
cargo bench

# Benchmark específico
cargo bench pipeline_throughput
```

## 📈 Observabilidade

### Métricas Integradas
```rust
// Acessar métricas do pipeline
let metrics = pipeline.get_metrics().await;
println!("Execuções: {}", metrics.executions.len());
println!("Taxa de sucesso: {:.2}%", metrics.success_rate * 100.0);
println!("Total processado: {}", metrics.total_rows_processed);
```

### Estados do Pipeline
```rust
// Verificar estado atual
let state = pipeline.current_state();
println!("Estado: {:?}", state); // Idle, Extracting, Transforming, Loading, etc.
```

### Eventos e Logging
```rust
// Configurar logging estruturado
tracing_subscriber::fmt::init();

// Pipeline emite eventos automaticamente para PipelineEvent::Started, 
// PipelineEvent::Completed, PipelineEvent::Error, etc.
```

## 🤝 Contribuindo

Contribuições são bem-vindas! Para contribuir:

1. Fork o projeto
2. Crie sua feature branch (`git checkout -b feature/MinhaFeature`)
3. Faça commit das suas mudanças (`git commit -m 'Adiciona MinhaFeature'`)
4. Push para a branch (`git push origin feature/MinhaFeature`)
5. Abra um Pull Request

### Desenvolvimento

```bash
# Instalar dependências de desenvolvimento
cargo install cargo-watch cargo-expand

# Executar verificações
cargo fmt --check
cargo clippy -- -D warnings

# Watch mode para desenvolvimento
cargo watch -x test
```

## 📊 Status do Projeto

### Recursos Implementados ✅

- [x] Core ETL Engine com Pipeline Builder
- [x] Extractors: CSV, JSON, Parquet
- [x] Transformers: Filter, Map, Aggregate, Select, Rename, etc.
- [x] Loaders: JSON, JSON Lines, Console, Memory, Parquet
- [x] Batch Processing
- [x] Métricas e Observabilidade
- [x] Error Handling robusto
- [x] Sistema de Eventos
- [x] Processamento Paralelo
- [x] Type-safe DataValue System

### Roadmap Futuro 🚧

- [ ] Database Connectors (PostgreSQL, MySQL)
- [ ] Cloud Storage (S3, Azure Blob)
- [ ] Streaming Processing
- [ ] Delta Lake Integration completa
- [ ] Schema Evolution
- [ ] Data Quality Framework
- [ ] Web Dashboard
- [ ] Kubernetes Operator

### Versões Suportadas

| Versão | Status | Rust Mínimo |
|--------|--------|-------------|
| 0.1.x  | ✅ Ativa | 1.75+ |

## 📝 Licença

Este projeto está licenciado sob a Apache License 2.0 - veja o arquivo [LICENSE](LICENSE) para detalhes.

## 🙏 Agradecimentos

- Comunidade Rust por ferramentas e bibliotecas incríveis
- Projeto Apache Arrow para suporte Parquet
- Contribuidores e usuários que fornecem feedback valioso

## 📞 Contato

- **Autor**: Silvano Neto
- **Email**: dev@silvanoneto.com
- **GitHub**: [@silvanoneto](https://github.com/silvanoneto)

---

<p align="center">
  Feito com ❤️ e Rust 🦀
  <br>
  <a href="#-etlrs---framework-etl-moderno-em-rust">Voltar ao topo</a>
</p>
