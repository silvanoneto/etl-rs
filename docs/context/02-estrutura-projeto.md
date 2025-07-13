# 🏗️ Estrutura do Projeto

## Configuração Inicial

### 1. Inicializar Projeto Rust

```bash
# Criar diretório do projeto
mkdir etlrs && cd etlrs

# Inicializar biblioteca Rust
cargo init --lib

# Configurar como workspace se necessário
cargo init --name etlrs --lib
```

### 2. Configuração de Workspace (Opcional)

Para projetos maiores, considere usar um workspace:

```toml
# Cargo.toml (raiz)
[workspace]
members = [
    "etlrs-core",
    "etlrs-connectors", 
    "etlrs-ai",
    "etlrs-streaming",
    "etlrs-cloud"
]

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ... outras dependências compartilhadas
```

### 3. Estrutura de Diretórios Recomendada

```
etlrs/
├── src/
│   ├── lib.rs                    # Ponto de entrada principal da biblioteca
│   ├── prelude.rs               # Importações comuns para facilitar uso
│   ├── config.rs                # Configuração central do sistema
│   ├── error.rs                 # Sistema de erros centralizado
│   ├── types.rs                 # Tipos de dados fundamentais
│   ├── utils.rs                 # Funções utilitárias compartilhadas
│   │
│   ├── extract/                 # Módulos de extração de dados
│   │   ├── mod.rs              # Definições de traits e tipos comuns
│   │   ├── csv.rs              # Extração de arquivos CSV/TSV
│   │   ├── json.rs             # Extração de JSON/JSONL/NDJSON
│   │   ├── xml.rs              # Extração de XML/SOAP
│   │   ├── yaml.rs             # Extração de YAML
│   │   ├── toml.rs             # Extração de TOML
│   │   ├── parquet.rs          # Extração de Apache Parquet
│   │   ├── avro.rs             # Extração de Apache Avro
│   │   ├── arrow.rs            # Extração de Apache Arrow
│   │   ├── orc.rs              # Extração de Apache ORC
│   │   ├── delta.rs            # Extração de Delta Lake
│   │   ├── iceberg.rs          # Extração de Apache Iceberg
│   │   ├── protobuf.rs         # Extração de Protocol Buffers
│   │   ├── msgpack.rs          # Extração de MessagePack
│   │   ├── excel.rs            # Extração de Excel (XLS/XLSX)
│   │   ├── database.rs         # Extração de bancos relacionais
│   │   ├── postgresql.rs       # Extração específica PostgreSQL
│   │   ├── mysql.rs            # Extração específica MySQL
│   │   ├── sqlite.rs           # Extração específica SQLite
│   │   ├── oracle.rs           # Extração específica Oracle
│   │   ├── mssql.rs            # Extração específica SQL Server
│   │   ├── nosql.rs            # Extração de bancos NoSQL
│   │   ├── mongodb.rs          # Extração específica MongoDB
│   │   ├── cassandra.rs        # Extração específica Cassandra
│   │   ├── redis.rs            # Extração específica Redis
│   │   ├── elasticsearch.rs    # Extração de Elasticsearch
│   │   ├── solr.rs             # Extração de Apache Solr
│   │   ├── api.rs              # Extração via APIs REST/GraphQL
│   │   ├── kafka.rs            # Extração de Apache Kafka
│   │   ├── pulsar.rs           # Extração de Apache Pulsar
│   │   ├── rabbitmq.rs         # Extração de RabbitMQ
│   │   ├── sqs.rs              # Extração de Amazon SQS
│   │   ├── file_system.rs      # Extração de sistemas de arquivos
│   │   ├── ftp.rs              # Extração via FTP/SFTP
│   │   ├── s3.rs               # Extração de Amazon S3
│   │   ├── gcs.rs              # Extração de Google Cloud Storage
│   │   ├── azure_blob.rs       # Extração de Azure Blob Storage
│   │   ├── hdfs.rs             # Extração de Hadoop HDFS
│   │   ├── streaming.rs        # Extração de streams em tempo real
│   │   ├── log_files.rs        # Extração de arquivos de log
│   │   ├── syslog.rs           # Extração de Syslog
│   │   ├── email.rs            # Extração de emails (IMAP/POP3)
│   │   ├── ldap.rs             # Extração de LDAP/Active Directory
│   │   ├── metrics.rs          # Extração de métricas (Prometheus)
│   │   ├── blockchain.rs       # Extração de dados blockchain
│   │   ├── hdf5.rs             # Extração de HDF5
│   │   ├── netcdf.rs           # Extração de NetCDF
│   │   ├── geojson.rs          # Extração de GeoJSON
│   │   ├── shapefile.rs        # Extração de Shapefiles
│   │   ├── raster.rs           # Extração de dados raster (GeoTIFF)
│   │   ├── dicom.rs            # Extração de DICOM (imagens médicas)
│   │   ├── fits.rs             # Extração de FITS (astronomia)
│   │   ├── binary.rs           # Extração de formatos binários customizados
│   │   ├── fixed_width.rs      # Extração de arquivos fixed-width
│   │   ├── edifact.rs          # Extração de EDI/EDIFACT
│   │   ├── hl7.rs              # Extração de HL7 (healthcare)
│   │   ├── swift.rs            # Extração de mensagens SWIFT
│   │   └── pcap.rs             # Extração de arquivos PCAP (network)
│   │
│   ├── transform/               # Módulos de transformação de dados
│   │   ├── mod.rs              # Definições de traits e pipeline builder
│   │   ├── filters.rs          # Filtros de dados (where, having, etc.)
│   │   ├── aggregations.rs     # Agregações (sum, avg, count, etc.)
│   │   ├── mapping.rs          # Mapeamento e renomeação de campos
│   │   ├── validation.rs       # Validação de dados e regras de negócio
│   │   ├── cleaning.rs         # Limpeza de dados (deduplicação, etc.)
│   │   ├── enrichment.rs       # Enriquecimento de dados
│   │   ├── normalization.rs    # Normalização de formatos
│   │   ├── windowing.rs        # Transformações com janelas temporais
│   │   └── custom.rs           # Transformações personalizadas
│   │
│   ├── load/                    # Módulos de carregamento de dados
│   │   ├── mod.rs              # Definições de traits e tipos comuns
│   │   ├── csv.rs              # Carregamento para CSV/TSV
│   │   ├── json.rs             # Carregamento para JSON/JSONL/NDJSON
│   │   ├── xml.rs              # Carregamento para XML
│   │   ├── yaml.rs             # Carregamento para YAML
│   │   ├── toml.rs             # Carregamento para TOML
│   │   ├── parquet.rs          # Carregamento para Apache Parquet
│   │   ├── avro.rs             # Carregamento para Apache Avro
│   │   ├── arrow.rs            # Carregamento para Apache Arrow
│   │   ├── orc.rs              # Carregamento para Apache ORC
│   │   ├── delta.rs            # Carregamento para Delta Lake
│   │   ├── iceberg.rs          # Carregamento para Apache Iceberg
│   │   ├── protobuf.rs         # Carregamento para Protocol Buffers
│   │   ├── msgpack.rs          # Carregamento para MessagePack
│   │   ├── excel.rs            # Carregamento para Excel (XLS/XLSX)
│   │   ├── database.rs         # Carregamento para bancos relacionais
│   │   ├── postgresql.rs       # Carregamento específico PostgreSQL
│   │   ├── mysql.rs            # Carregamento específico MySQL
│   │   ├── sqlite.rs           # Carregamento específico SQLite
│   │   ├── oracle.rs           # Carregamento específico Oracle
│   │   ├── mssql.rs            # Carregamento específico SQL Server
│   │   ├── nosql.rs            # Carregamento para bancos NoSQL
│   │   ├── mongodb.rs          # Carregamento específico MongoDB
│   │   ├── cassandra.rs        # Carregamento específico Cassandra
│   │   ├── redis.rs            # Carregamento específico Redis
│   │   ├── elasticsearch.rs    # Carregamento para Elasticsearch
│   │   ├── solr.rs             # Carregamento para Apache Solr
│   │   ├── api.rs              # Carregamento via APIs REST/GraphQL
│   │   ├── kafka.rs            # Carregamento para Apache Kafka
│   │   ├── pulsar.rs           # Carregamento para Apache Pulsar
│   │   ├── rabbitmq.rs         # Carregamento para RabbitMQ
│   │   ├── sqs.rs              # Carregamento para Amazon SQS
│   │   ├── s3.rs               # Carregamento para Amazon S3
│   │   ├── gcs.rs              # Carregamento para Google Cloud Storage
│   │   ├── azure_blob.rs       # Carregamento para Azure Blob Storage
│   │   ├── hdfs.rs             # Carregamento para Hadoop HDFS
│   │   ├── warehouse.rs        # Carregamento para data warehouses
│   │   ├── redshift.rs         # Carregamento para Amazon Redshift
│   │   ├── snowflake.rs        # Carregamento para Snowflake
│   │   ├── bigquery.rs         # Carregamento para Google BigQuery
│   │   ├── databricks.rs       # Carregamento para Databricks
│   │   ├── lake.rs             # Carregamento para data lakes
│   │   ├── hive.rs             # Carregamento para Apache Hive
│   │   ├── spark.rs            # Carregamento para Apache Spark
│   │   ├── flink.rs            # Carregamento para Apache Flink
│   │   ├── batch.rs            # Carregamento em lotes otimizado
│   │   ├── streaming.rs        # Carregamento em tempo real
│   │   ├── email.rs            # Carregamento via email (SMTP)
│   │   ├── ftp.rs              # Carregamento via FTP/SFTP
│   │   ├── webhook.rs          # Carregamento via webhooks
│   │   ├── syslog.rs           # Carregamento para Syslog
│   │   ├── metrics.rs          # Carregamento de métricas (Prometheus)
│   │   ├── time_series.rs      # Carregamento para bancos de séries temporais
│   │   ├── influxdb.rs         # Carregamento para InfluxDB
│   │   ├── timescaledb.rs      # Carregamento para TimescaleDB
│   │   ├── clickhouse.rs       # Carregamento para ClickHouse
│   │   ├── geojson.rs          # Carregamento para GeoJSON
│   │   ├── shapefile.rs        # Carregamento para Shapefiles
│   │   ├── raster.rs           # Carregamento de dados raster
│   │   ├── hdf5.rs             # Carregamento para HDF5
│   │   ├── netcdf.rs           # Carregamento para NetCDF
│   │   ├── dicom.rs            # Carregamento para DICOM
│   │   ├── fits.rs             # Carregamento para FITS
│   │   ├── binary.rs           # Carregamento para formatos binários
│   │   ├── fixed_width.rs      # Carregamento para fixed-width
│   │   ├── edifact.rs          # Carregamento para EDI/EDIFACT
│   │   ├── hl7.rs              # Carregamento para HL7
│   │   ├── swift.rs            # Carregamento para mensagens SWIFT
│   │   └── report.rs           # Carregamento para relatórios (PDF/HTML)
│   │
│   ├── pipeline/                # Orquestração de pipeline
│   │   ├── mod.rs              # Definições principais do pipeline
│   │   ├── builder.rs          # Pattern builder para pipelines
│   │   ├── executor.rs         # Executor de pipeline assíncrono
│   │   ├── scheduler.rs        # Agendamento de pipelines
│   │   ├── dependency.rs       # Gestão de dependências entre pipelines
│   │   ├── retry.rs            # Lógica de retry e recuperação
│   │   ├── parallelism.rs      # Processamento paralelo
│   │   └── optimization.rs     # Otimizações automáticas de pipeline
│   │
│   ├── integrity/               # Módulos de integridade de dados
│   │   ├── mod.rs              # Definições de integridade
│   │   ├── checksum.rs         # Cálculo de checksums (SHA256, CRC32, MD5)
│   │   ├── validator.rs        # Validação de integridade cross-stage
│   │   ├── schema.rs           # Validação de schema e estrutura
│   │   ├── audit.rs            # Trilha de auditoria detalhada
│   │   ├── verification.rs     # Verificação de integridade end-to-end
│   │   ├── rollback.rs         # Mecanismos de rollback
│   │   └── reconciliation.rs   # Reconciliação de dados
│   │
│   ├── ai/                      # Recursos de IA integrados
│   │   ├── mod.rs              # Definições de IA
│   │   ├── profiling.rs        # Profiling inteligente de dados
│   │   ├── schema_generation.rs # Geração automática de schemas
│   │   ├── anomaly_detection.rs # Detecção de anomalias com ML
│   │   ├── quality_prediction.rs # Predição de qualidade de dados
│   │   ├── optimization.rs     # Otimização de pipeline com IA
│   │   ├── classification.rs   # Classificação automática de dados
│   │   └── feature_engineering.rs # Engenharia de features automática
│   │
│   ├── streaming/               # Processamento em tempo real
│   │   ├── mod.rs              # Definições de streaming
│   │   ├── engine.rs           # Motor de processamento de stream
│   │   ├── windowing.rs        # Estratégias de janelas temporais
│   │   ├── watermarking.rs     # Gestão de watermarks
│   │   ├── backpressure.rs     # Controle de backpressure
│   │   ├── state.rs            # Gestão de estado em streaming
│   │   └── triggers.rs         # Triggers de processamento
│   │
│   ├── cep/                     # Processamento de Eventos Complexos
│   │   ├── mod.rs              # Definições de CEP
│   │   ├── engine.rs           # Motor de processamento de eventos
│   │   ├── patterns.rs         # Padrões de eventos e regras
│   │   ├── rules.rs            # Engine de regras
│   │   ├── correlation.rs      # Correlação de eventos
│   │   └── alerts.rs           # Sistema de alertas
│   │
│   ├── lineage/                 # Rastreamento de linhagem de dados
│   │   ├── mod.rs              # Definições de linhagem
│   │   ├── tracker.rs          # Rastreamento de transformações
│   │   ├── graph.rs            # Grafo de linhagem
│   │   ├── visualization.rs    # Visualização de linhagem
│   │   ├── impact_analysis.rs  # Análise de impacto de mudanças
│   │   └── metadata.rs         # Metadados de linhagem
│   │
│   ├── governance/              # Governança de dados empresarial
│   │   ├── mod.rs              # Definições de governança
│   │   ├── framework.rs        # Framework de governança
│   │   ├── policies.rs         # Políticas de dados
│   │   ├── compliance.rs       # Compliance (GDPR, HIPAA, SOX)
│   │   ├── privacy.rs          # Privacidade e anonimização
│   │   ├── access_control.rs   # Controle de acesso
│   │   └── data_catalog.rs     # Catálogo de dados
│   │
│   ├── cloud/                   # Suporte multi-nuvem robusto
│   │   ├── mod.rs              # Definições multi-nuvem
│   │   ├── connectors.rs       # Conectores abstratos
│   │   ├── aws.rs              # Amazon Web Services
│   │   ├── azure.rs            # Microsoft Azure
│   │   ├── gcp.rs              # Google Cloud Platform
│   │   ├── kubernetes.rs       # Kubernetes deployments
│   │   ├── terraform.rs        # Integração com Terraform
│   │   └── cost_optimizer.rs   # Otimização de custos
│   │
│   ├── ml/                      # Integração com Machine Learning
│   │   ├── mod.rs              # Definições de ML
│   │   ├── integration.rs      # Integração com frameworks ML
│   │   ├── automl.rs           # AutoML para ETL
│   │   ├── models.rs           # Gestão de modelos
│   │   ├── training.rs         # Pipeline de treino
│   │   ├── inference.rs        # Pipeline de inferência
│   │   ├── model_registry.rs   # Registro de modelos
│   │   └── feature_store.rs    # Feature store integrado
│   │
│   ├── analytics/               # Analytics avançado
│   │   ├── mod.rs              # Definições de analytics
│   │   ├── engine.rs           # Motor de analytics
│   │   ├── statistics.rs       # Estatísticas descritivas e inferenciais
│   │   ├── time_series.rs      # Análise de séries temporais
│   │   ├── forecasting.rs      # Previsão de demanda
│   │   ├── clustering.rs       # Análise de clusters
│   │   ├── correlation.rs      # Análise de correlação
│   │   └── reporting.rs        # Relatórios automatizados
│   │
│   ├── security/                # Segurança avançada
│   │   ├── mod.rs              # Definições de segurança
│   │   ├── zero_trust.rs       # Arquitetura Zero Trust
│   │   ├── encryption.rs       # Criptografia avançada
│   │   ├── compliance.rs       # Compliance de segurança
│   │   ├── authentication.rs   # Autenticação multi-fator
│   │   ├── authorization.rs    # Autorização granular
│   │   ├── secrets.rs          # Gestão de secrets
│   │   └── vulnerability.rs    # Escaneamento de vulnerabilidades
│   │
│   ├── observability/           # Monitoramento e observabilidade
│   │   ├── mod.rs              # Definições de observabilidade
│   │   ├── tracing.rs          # Rastreamento distribuído
│   │   ├── metrics.rs          # Métricas customizadas
│   │   ├── logging.rs          # Logging estruturado
│   │   ├── monitoring.rs       # Monitoramento de saúde
│   │   ├── alerting.rs         # Sistema de alertas
│   │   ├── dashboards.rs       # Dashboards em tempo real
│   │   └── profiling.rs        # Profiling de performance
│   │
│   ├── formats/                 # Formatos de dados avançados
│   │   ├── mod.rs              # Definições de formatos
│   │   ├── parquet.rs          # Apache Parquet
│   │   ├── avro.rs             # Apache Avro
│   │   ├── arrow.rs            # Apache Arrow
│   │   ├── orc.rs              # Apache ORC
│   │   ├── protobuf.rs         # Protocol Buffers
│   │   ├── msgpack.rs          # MessagePack
│   │   ├── delta.rs            # Delta Lake
│   │   ├── iceberg.rs          # Apache Iceberg
│   │   ├── hdf5.rs             # HDF5 (scientific data)
│   │   ├── netcdf.rs           # NetCDF (climate/weather data)
│   │   ├── fits.rs             # FITS (astronomy data)
│   │   ├── dicom.rs            # DICOM (medical imaging)
│   │   ├── geospatial.rs       # Formatos geoespaciais
│   │   ├── binary.rs           # Formatos binários customizados
│   │   └── compression.rs      # Compressão avançada (LZ4, ZSTD, etc.)
│   │
│   ├── distributed/             # Processamento distribuído
│   │   ├── mod.rs              # Definições distribuídas
│   │   ├── coordinator.rs      # Coordenador de cluster
│   │   ├── scheduler.rs        # Agendador distribuído
│   │   ├── scaling.rs          # Auto-scaling dinâmico
│   │   ├── consensus.rs        # Algoritmos de consenso
│   │   ├── replication.rs      # Replicação de dados
│   │   ├── partitioning.rs     # Estratégias de particionamento
│   │   └── fault_tolerance.rs  # Tolerância a falhas
│   │
│   ├── testing/                 # Testes avançados
│   │   ├── mod.rs              # Definições de testes
│   │   ├── data_quality.rs     # Testes de qualidade de dados
│   │   ├── property_based.rs   # Testes baseados em propriedades
│   │   ├── integration.rs      # Testes de integração
│   │   ├── performance.rs      # Testes de performance
│   │   ├── chaos.rs            # Chaos engineering
│   │   ├── mocking.rs          # Mocking de dependências
│   │   └── fixtures.rs         # Fixtures de teste
│   │
│   ├── connectors/              # Conectores especializados
│   │   ├── mod.rs              # Definições de conectores
│   │   ├── salesforce.rs       # Salesforce CRM
│   │   ├── sap.rs              # SAP ERP
│   │   ├── oracle.rs           # Oracle Database
│   │   ├── mainframe.rs        # Sistemas mainframe
│   │   ├── erp.rs              # Sistemas ERP genéricos
│   │   ├── crm.rs              # Sistemas CRM genéricos
│   │   ├── social_media.rs     # APIs de redes sociais
│   │   ├── blockchain.rs       # Dados de blockchain
│   │   ├── iot.rs              # Internet of Things
│   │   ├── sensor.rs           # Dados de sensores
│   │   ├── network.rs          # Dados de rede (SNMP, NetFlow)
│   │   ├── financial.rs        # Dados financeiros especializados
│   │   ├── healthcare.rs       # Dados de saúde (HL7, FHIR)
│   │   └── automotive.rs       # Dados automotivos (CAN bus)
│   │
│   ├── deployment/              # Deployment e DevOps
│   │   ├── mod.rs              # Definições de deployment
│   │   ├── docker.rs           # Containerização Docker
│   │   ├── kubernetes.rs       # Orchestração Kubernetes
│   │   ├── helm.rs             # Charts Helm
│   │   ├── ci_cd.rs            # Pipelines CI/CD
│   │   ├── terraform.rs        # Infrastructure as Code
│   │   └── monitoring.rs       # Monitoramento de deployment
│   │
│   └── examples/                # Exemplos práticos
│       ├── mod.rs              # Índice de exemplos
│       ├── basic_etl.rs        # ETL básico
│       ├── streaming_etl.rs    # ETL em tempo real
│       ├── ml_pipeline.rs      # Pipeline ML
│       ├── data_quality.rs     # Qualidade de dados
│       ├── governance.rs       # Governança de dados
│       ├── multi_cloud.rs      # Multi-nuvem
│       └── enterprise.rs       # Exemplo empresarial completo
│
├── benches/                     # Benchmarks de performance
│   ├── pipeline_benchmark.rs   # Benchmark de pipeline
│   ├── transform_benchmark.rs  # Benchmark de transformações
│   ├── integrity_benchmark.rs  # Benchmark de integridade
│   └── streaming_benchmark.rs  # Benchmark de streaming
│
├── docs/                        # Documentação adicional
│   ├── architecture.md         # Arquitetura do sistema
│   ├── performance.md          # Guia de performance
│   ├── security.md             # Guia de segurança
│   ├── deployment.md           # Guia de deployment
│   ├── troubleshooting.md      # Solução de problemas
│   ├── migration.md            # Guia de migração
│   └── api/                    # Documentação da API
│
├── scripts/                     # Scripts utilitários
│   ├── build.sh               # Script de build
│   ├── test.sh                # Script de testes
│   ├── benchmark.sh           # Script de benchmarks
│   ├── deploy.sh              # Script de deployment
│   └── setup_dev.sh           # Setup do ambiente dev
│
├── docker/                      # Arquivos Docker
│   ├── Dockerfile             # Dockerfile principal
│   ├── docker-compose.yml     # Compose para desenvolvimento
│   └── kubernetes/            # Manifests Kubernetes
│
├── .github/                     # GitHub Actions
│   └── workflows/
│       ├── ci.yml             # CI/CD pipeline
│       ├── security.yml       # Scans de segurança
│       └── release.yml        # Release automation
|
├── docs/
│   ├── context/                # Contexto do projeto
│   │   ├── 01-visao-geral.md      # Visão geral do sistema
│   │   ├── 02-estrutura-projeto.md # Estrutura do projeto (este documento)
│   │   ├── 03-dependencias.md     # Dependências e bibliotecas
│   │   ├── 04-arquitetura-principal.md # Arquitetura detalhada
│   │   ├── 05-framework-integridade.md # Framework de integridade
│   │   ├── 06-recursos-avancados.md   # Recursos avançados
│   │   └── 07-manutencao-gestao.md   # Manutenção e gestão
│   ├── architecture.md         # Arquitetura do sistema
│   ├── performance.md          # Guia de performance
│   ├── security.md             # Guia de segurança
│   ├── deployment.md           # Guia de deployment
│   ├── troubleshooting.md      # Solução de problemas
│   ├── migration.md            # Guia de migração
│   └── api/                    # Documentação da API
│
├── Cargo.toml                  # Dependências e metadados
├── Cargo.lock                  # Lock de dependências
├── README.md                   # Documentação do projeto
├── LICENSE                     # Licença do projeto
├── CHANGELOG.md               # Histórico de mudanças
├── CONTRIBUTING.md            # Guia para contribuidores
└── SECURITY.md                # Políticas de segurança

### 4. Configuração de Desenvolvimento

As ferramentas abaixo auxiliam no desenvolvimento, automação, inspeção e qualidade do código:

- `cargo-expand`: Expande macros para facilitar a depuração de código gerado.
- `cargo-audit`: Verifica vulnerabilidades conhecidas nas dependências do projeto.
- `cargo-deny`: Enforça políticas de dependências, analisando licenças, vulnerabilidades e duplicatas nas dependências.
- `cargo-husky`: Gerencia hooks de pre-commit para garantir padrões de qualidade antes de commits.

```bash
# Instalar ferramentas de desenvolvimento
cargo install cargo-watch
cargo install cargo-expand
cargo install cargo-audit
cargo install cargo-deny

# Configurar pre-commit hooks
cargo install cargo-husky

# Inicialize e configure hooks de pre-commit com cargo-husky
cargo husky init

# Exemplo: adicionar um hook de lint antes do commit
echo 'cargo clippy --all-targets --all-features -- -D warnings' > .husky/pre-commit
chmod +x .husky/pre-commit
```

### 5. Configuração de Qualidade de Código

As aliases abaixo facilitam tarefas comuns de qualidade de código:
- `lint`: Executa o Clippy em todos os targets e features, tratando warnings como erros.
- `test-workspace`: Executa todos os testes do workspace com todas as features habilitadas.
- `test-all`: Executa todos os testes do workspace com todas as features habilitadas.

```toml
# .cargo/config.toml
[build]
rustflags = ["-D", "warnings"]

[alias]
lint = "clippy --all-targets --all-features -- -D warnings"
fmt-check = "fmt --all -- --check"
test-all = "test --all-features --workspace"
```

### 6. Estrutura de Configuração

```rust
// src/config.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ETLConfig {
    pub pipeline: PipelineConfig,
    pub integrity: IntegrityConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub max_parallel_tasks: usize,
    pub retry_attempts: u32,
    pub timeout_seconds: u64,
    pub checkpoint_interval: u64,
}

// Exemplos de campos para cada struct de configuração.
// Consulte src/config.rs para definições completas e atualizadas.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    pub enable_checksums: bool,
    pub checksum_type: String,
    pub enable_schema_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_threads: usize,
    pub batch_size: usize,
    pub enable_parallelism: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_encryption: bool,
    pub allowed_roles: Vec<String>,
    pub audit_log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub tracing_level: String,
}
```
