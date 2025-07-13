# Dependências Principais

A seleção de dependências abaixo foi feita para garantir alta performance, segurança, extensibilidade e integração com os principais recursos da arquitetura ETLRS, cobrindo desde processamento assíncrono, conectividade com bancos de dados, validação, observabilidade, até suporte a formatos avançados e machine learning.

## Adicionar ao Cargo.toml

> **Nota:** As dependências abaixo incluem todas as bibliotecas referenciadas nos exemplos de código deste documento, garantindo que o projeto compile corretamente.

```toml
[dependencies]
# Runtime assíncrono
tokio = { version = "1.0", features = ["full"] }
# Processamento de dados
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
csv = "1.1"
# Conectividade com banco de dados
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "mysql", "sqlite"] }
# Tratamento de erros
thiserror = "1.0"
anyhow = "1.0"
# Cliente HTTP
reqwest = { version = "0.11", features = ["json"] }
# Sistema de logging
tracing = "0.1"
tracing-subscriber = "0.3"
# Configuração
config = "0.13"
# Performance
rayon = "1.5"
# Integridade de dados
sha2 = "0.10"
md5 = "0.7"
crc32fast = "1.3"
uuid = { version = "1.0", features = ["v4"] }
# Validação de schema
jsonschema = "0.17"
# Serialização para checksums
bincode = "1.3"
# Formatos avançados
apache-avro = "0.16"
parquet = "50.0"
arrow = "50.0"
# Machine Learning
candle-core = "0.4"
candle-nn = "0.4"
# Streaming
tokio-stream = "0.1"
# Processamento de grafos
petgraph = "0.6"
# Observabilidade
opentelemetry = "0.21"
tracing-opentelemetry = "0.22"
prometheus = "0.13"
# SDKs de nuvem
aws-sdk-s3 = "1.0"
azure_storage = "0.19"
google-cloud-storage = "0.18"
# Sistemas distribuídos
etcd-rs = "1.1"
# Framework web para dashboard
warp = "0.3"
# Séries temporais
polars = "0.35"
# Segurança
ring = "0.17"
# Processamento de eventos complexos
regex = "1.0"

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"
tempfile = "3.0"

[[bench]]
name = "pipeline_benchmark"
harness = false
```

## Categorias de Dependências

### Runtime e Async
- **tokio**: Runtime assíncrono principal para todas as operações I/O
- **tokio-stream**: Extensões para processamento de streams
- **tokio-test**: Utilitários para testes assíncronos

### Serialização e Dados
- **serde**: Framework principal de serialização/deserialização
- **serde_json**: Suporte para formato JSON
- **csv**: Processamento eficiente de arquivos CSV
- **bincode**: Serialização binária rápida para checksums

### Bancos de Dados
- **sqlx**: Driver SQL assíncrono com suporte a múltiplos bancos
  - PostgreSQL, MySQL, SQLite
  - Pool de conexões automático
  - Preparação de statements

### Tratamento de Erros
- **thiserror**: Macros para criação de tipos de erro customizados
- **anyhow**: Handling simplificado de erros em aplicações

### HTTP e APIs
- **reqwest**: Cliente HTTP assíncrono com suporte a JSON
- **warp**: Framework web para dashboards e APIs

### Observabilidade
- **tracing**: Framework de instrumentação estruturada
- **tracing-subscriber**: Coletores e formatadores de traces
- **tracing-opentelemetry**: Integração com OpenTelemetry
- **opentelemetry**: Padrão de observabilidade distribuída
- **prometheus**: Métricas para monitoramento

### Performance
- **rayon**: Paralelismo de dados automático
- **polars**: DataFrames de alta performance para análise

### Integridade e Segurança
- **sha2**: Algoritmos de hash SHA-256/SHA-512
- **md5**: Hash MD5 para compatibilidade
- **crc32fast**: Verificação rápida de CRC32
- **ring**: Primitivas criptográficas seguras
- **uuid**: Geração de identificadores únicos

### Formatos Avançados
- **apache-avro**: Serialização schema-based
- **parquet**: Formato colunar para analytics
- **arrow**: Formato de memória para processamento eficiente

### Machine Learning
- **candle-core**: Framework ML nativo em Rust
- **candle-nn**: Redes neurais para IA integrada

### Cloud e Distribuído
- **aws-sdk-s3**: SDK oficial da AWS para S3
- **azure_storage**: Cliente para Azure Storage
- **google-cloud-storage**: Cliente para Google Cloud Storage
- **etcd-rs**: Cliente para etcd (coordenação distribuída)

### Validação
- **jsonschema**: Validação de schemas JSON
- **regex**: Expressões regulares para validação de padrões

### Processamento de Grafos
- **petgraph**: Algoritmos de grafos para linhagem de dados

### Configuração
- **config**: Sistema de configuração hierárquica

### Desenvolvimento
- **criterion**: Benchmarking estatístico preciso
- **tempfile**: Criação de arquivos temporários para testes

## Dependências Opcionais por Feature

Dependendo das features habilitadas, dependências adicionais podem ser incluídas:

### Feature: "kafka"
```toml
rdkafka = { version = "0.36", features = ["cmake-build"] }
```

### Feature: "mongodb"
```toml
mongodb = "2.8"
```

### Feature: "redis"
```toml
redis = { version = "0.24", features = ["tokio-comp"] }
```

### Feature: "elasticsearch"
```toml
elasticsearch = "8.5"
```

### Feature: "blockchain"
```toml
web3 = "0.19"
ethers = "2.0"
```

### Feature: "ml-advanced"
```toml
ort = "1.16"  # ONNX Runtime
tch = "0.13"  # PyTorch bindings
```

### Feature: "encryption"
```toml
age = "0.9"
chacha20poly1305 = "0.10"
```

## Considerações de Performance

### Otimizações de Compilação
```toml
[profile.release]
codegen-units = 1
lto = true
panic = "abort"
strip = true

[profile.bench]
debug = true
```

### Features Condicionais
```toml
[features]
default = ["basic"]
basic = ["csv", "json", "sqlite"]
full = ["basic", "kafka", "mongodb", "redis", "ml-advanced"]
cloud = ["aws", "azure", "gcp"]
enterprise = ["full", "cloud", "encryption", "compliance"]
```

Isso permite que usuários escolham apenas as dependências necessárias para seus casos de uso específicos, reduzindo o tempo de compilação e o tamanho do binário final.
