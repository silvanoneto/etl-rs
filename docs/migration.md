# 🔄 Guia de Migração

## Visão Geral

Este guia aborda estratégias e procedimentos para migração entre versões da biblioteca ETLRS, incluindo breaking changes, migração de dados, atualizações de configuração e rollback procedures. Cobrimos desde migrações simples até cenários complexos de multi-ambiente.

## Estratégias de Migração

### 1. Tipos de Migração

#### Migração de Versão Minor (Backward Compatible)
```toml
# Cargo.toml - De 1.1.0 para 1.2.0
[dependencies]
etlrs = "1.2.0"  # Era 1.1.0

# Mudanças típicas:
# - Novas funcionalidades
# - Melhorias de performance
# - Bug fixes
# - Depreciações com warnings
```

#### Migração de Versão Major (Breaking Changes)
```toml
# Cargo.toml - De 1.x.x para 2.0.0
[dependencies]
etlrs = "2.0.0"  # Era 1.x.x

# Mudanças típicas:
# - API changes
# - Remoção de funcionalidades depreciadas
# - Mudanças em estruturas de dados
# - Novos requisitos de sistema
```

### 2. Plano de Migração

```rust
// migration/migration_plan.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub from_version: String,
    pub to_version: String,
    pub migration_type: MigrationType,
    pub steps: Vec<MigrationStep>,
    pub rollback_steps: Vec<MigrationStep>,
    pub validation_checks: Vec<ValidationCheck>,
    pub estimated_duration: std::time::Duration,
    pub downtime_required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MigrationType {
    MinorUpgrade,
    MajorUpgrade,
    Patch,
    Hotfix,
    Rollback,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStep {
    pub id: String,
    pub description: String,
    pub step_type: StepType,
    pub command: Option<String>,
    pub config_changes: Option<HashMap<String, serde_json::Value>>,
    pub data_migration: Option<DataMigration>,
    pub validation: Option<String>,
    pub timeout: std::time::Duration,
    pub can_rollback: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StepType {
    ConfigUpdate,
    DataMigration,
    SchemaUpdate,
    ServiceRestart,
    Validation,
    Cleanup,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataMigration {
    pub source_table: String,
    pub target_table: String,
    pub transformation_script: String,
    pub batch_size: usize,
    pub parallel_workers: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub query: String,
    pub expected_result: serde_json::Value,
    pub critical: bool,
}

pub struct MigrationExecutor {
    current_version: String,
    target_version: String,
    dry_run: bool,
}

impl MigrationExecutor {
    pub fn new(current: &str, target: &str, dry_run: bool) -> Self {
        Self {
            current_version: current.to_string(),
            target_version: target.to_string(),
            dry_run,
        }
    }
    
    pub async fn execute_migration(&self, plan: &MigrationPlan) -> Result<MigrationResult, MigrationError> {
        let mut result = MigrationResult::new();
        
        info!("Starting migration from {} to {}", 
            self.current_version, self.target_version);
        
        if self.dry_run {
            info!("Running in DRY RUN mode - no changes will be made");
        }
        
        // Pre-migration validation
        self.validate_pre_conditions(plan).await?;
        
        // Execute migration steps
        for (index, step) in plan.steps.iter().enumerate() {
            info!("Executing step {}/{}: {}", 
                index + 1, plan.steps.len(), step.description);
            
            let step_result = self.execute_step(step).await?;
            result.completed_steps.push(step_result);
            
            // Validation after each critical step
            if let Some(validation) = &step.validation {
                self.validate_step(validation).await?;
            }
        }
        
        // Post-migration validation
        self.validate_post_conditions(plan).await?;
        
        info!("Migration completed successfully");
        Ok(result)
    }
    
    async fn execute_step(&self, step: &MigrationStep) -> Result<StepResult, MigrationError> {
        let start_time = std::time::Instant::now();
        
        if self.dry_run {
            info!("DRY RUN: Would execute step: {}", step.description);
            return Ok(StepResult {
                step_id: step.id.clone(),
                success: true,
                duration: start_time.elapsed(),
                message: "Dry run - not executed".to_string(),
            });
        }
        
        match step.step_type {
            StepType::ConfigUpdate => self.update_config(step).await,
            StepType::DataMigration => self.migrate_data(step).await,
            StepType::SchemaUpdate => self.update_schema(step).await,
            StepType::ServiceRestart => self.restart_service(step).await,
            StepType::Validation => self.validate_step(&step.id).await.map(|_| StepResult {
                step_id: step.id.clone(),
                success: true,
                duration: start_time.elapsed(),
                message: "Validation passed".to_string(),
            }),
            StepType::Cleanup => self.cleanup(step).await,
        }
    }
    
    async fn migrate_data(&self, step: &MigrationStep) -> Result<StepResult, MigrationError> {
        let start_time = std::time::Instant::now();
        
        if let Some(data_migration) = &step.data_migration {
            info!("Migrating data from {} to {}", 
                data_migration.source_table, data_migration.target_table);
            
            let migrator = DataMigrator::new(
                data_migration.batch_size,
                data_migration.parallel_workers,
            );
            
            migrator.migrate(
                &data_migration.source_table,
                &data_migration.target_table,
                &data_migration.transformation_script,
            ).await?;
        }
        
        Ok(StepResult {
            step_id: step.id.clone(),
            success: true,
            duration: start_time.elapsed(),
            message: "Data migration completed".to_string(),
        })
    }
}

#[derive(Debug)]
pub struct MigrationResult {
    pub completed_steps: Vec<StepResult>,
    pub total_duration: std::time::Duration,
    pub rollback_info: Option<RollbackInfo>,
}

#[derive(Debug)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub message: String,
}
```

## Guias de Migração por Versão

### 1. Migração 1.x → 2.0

#### Breaking Changes

```rust
// migration/v2_0_migration.rs

// ANTES (v1.x)
use etlrs::pipeline::Pipeline;
use etlrs::config::Config;

let config = Config {
    batch_size: 1000,
    parallel_tasks: 4,
    // ...
};

let pipeline = Pipeline::new(config);

// DEPOIS (v2.0)
use etlrs::pipeline::{Pipeline, PipelineBuilder};
use etlrs::config::{Config, PipelineConfig};

let config = Config::builder()
    .pipeline(PipelineConfig::builder()
        .batch_size(1000)
        .max_parallel_tasks(4)
        .build())
    .build();

let pipeline = PipelineBuilder::new()
    .with_config(config)
    .build()?;
```

#### Migração de Configuração

```rust
// migration/config_v2.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ConfigV1 {
    pub database_url: String,
    pub batch_size: usize,
    pub parallel_tasks: usize,
    pub log_level: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigV2 {
    pub database: DatabaseConfig,
    pub pipeline: PipelineConfig,
    pub observability: ObservabilityConfig,
}

pub struct ConfigMigrator;

impl ConfigMigrator {
    pub fn migrate_v1_to_v2(v1_config: ConfigV1) -> Result<ConfigV2, MigrationError> {
        Ok(ConfigV2 {
            database: DatabaseConfig {
                url: v1_config.database_url,
                pool_size: 10, // Novo campo com default
                timeout: std::time::Duration::from_secs(30), // Novo campo
            },
            pipeline: PipelineConfig {
                batch_size: v1_config.batch_size,
                max_parallel_tasks: v1_config.parallel_tasks,
                retry_attempts: 3, // Novo campo
                timeout: std::time::Duration::from_secs(300), // Novo campo
            },
            observability: ObservabilityConfig {
                log_level: v1_config.log_level.parse()?,
                enable_metrics: true, // Novo campo
                enable_tracing: true, // Novo campo
                metrics_port: 9090, // Novo campo
            },
        })
    }
}

// Script de migração de config
pub async fn migrate_config_file(input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Lê config v1
    let v1_content = tokio::fs::read_to_string(input_path).await?;
    let v1_config: ConfigV1 = toml::from_str(&v1_content)?;
    
    // Migra para v2
    let v2_config = ConfigMigrator::migrate_v1_to_v2(v1_config)?;
    
    // Salva config v2
    let v2_content = toml::to_string_pretty(&v2_config)?;
    tokio::fs::write(output_path, v2_content).await?;
    
    info!("Configuration migrated from {} to {}", input_path, output_path);
    Ok(())
}
```

### 2. Migração de Schema de Banco

```sql
-- migrations/001_v2_schema_updates.sql

-- Adicionar novas colunas
ALTER TABLE pipelines 
ADD COLUMN retry_attempts INTEGER DEFAULT 3,
ADD COLUMN timeout_seconds INTEGER DEFAULT 300,
ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
ADD COLUMN updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

-- Criar nova tabela para métricas
CREATE TABLE pipeline_metrics (
    id SERIAL PRIMARY KEY,
    pipeline_id INTEGER REFERENCES pipelines(id),
    metric_name VARCHAR(255) NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    labels JSONB
);

CREATE INDEX idx_pipeline_metrics_pipeline_id ON pipeline_metrics(pipeline_id);
CREATE INDEX idx_pipeline_metrics_timestamp ON pipeline_metrics(timestamp);
CREATE INDEX idx_pipeline_metrics_name ON pipeline_metrics(metric_name);

-- Migrar dados existentes
UPDATE pipelines 
SET retry_attempts = 3, timeout_seconds = 300 
WHERE retry_attempts IS NULL;

-- Criar trigger para updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_pipelines_updated_at 
    BEFORE UPDATE ON pipelines 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

```rust
// migration/schema_migrator.rs
use sqlx::{PgPool, Row};

pub struct SchemaMigrator {
    pool: PgPool,
}

impl SchemaMigrator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    pub async fn migrate_to_v2(&self) -> Result<(), sqlx::Error> {
        info!("Starting schema migration to v2.0");
        
        // Check current schema version
        let current_version = self.get_schema_version().await?;
        if current_version >= 2 {
            info!("Schema already at version 2 or higher");
            return Ok(());
        }
        
        // Start transaction
        let mut tx = self.pool.begin().await?;
        
        // Execute migration scripts
        self.execute_migration_script(&mut tx, "001_v2_schema_updates.sql").await?;
        
        // Update schema version
        sqlx::query("INSERT INTO schema_versions (version, applied_at) VALUES (2, CURRENT_TIMESTAMP)")
            .execute(&mut tx)
            .await?;
        
        // Commit transaction
        tx.commit().await?;
        
        info!("Schema migration to v2.0 completed successfully");
        Ok(())
    }
    
    async fn get_schema_version(&self) -> Result<i32, sqlx::Error> {
        // Create schema_versions table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_versions (
                version INTEGER PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&self.pool)
        .await?;
        
        // Get current version
        let row = sqlx::query("SELECT COALESCE(MAX(version), 0) as version FROM schema_versions")
            .fetch_one(&self.pool)
            .await?;
            
        Ok(row.get("version"))
    }
    
    async fn execute_migration_script(
        &self, 
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        script_path: &str
    ) -> Result<(), sqlx::Error> {
        let script_content = tokio::fs::read_to_string(format!("migrations/{}", script_path))
            .await
            .map_err(|e| sqlx::Error::Io(e))?;
        
        // Split script into individual statements
        for statement in script_content.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() && !statement.starts_with("--") {
                sqlx::query(statement).execute(tx).await?;
            }
        }
        
        Ok(())
    }
}
```

### 3. Migração de Dados

```rust
// migration/data_migrator.rs
use sqlx::{PgPool, Row};
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct DataMigrator {
    pool: PgPool,
    batch_size: usize,
    max_parallel: usize,
}

impl DataMigrator {
    pub fn new(pool: PgPool, batch_size: usize, max_parallel: usize) -> Self {
        Self {
            pool,
            batch_size,
            max_parallel,
        }
    }
    
    pub async fn migrate_pipeline_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting pipeline data migration");
        
        // Get total count for progress tracking
        let total_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pipelines_old"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let total_count = total_count.0 as usize;
        info!("Total records to migrate: {}", total_count);
        
        // Create semaphore for controlling parallelism
        let semaphore = Arc::new(Semaphore::new(self.max_parallel));
        let mut tasks = Vec::new();
        
        // Process data in batches
        for offset in (0..total_count).step_by(self.batch_size) {
            let pool = self.pool.clone();
            let semaphore = semaphore.clone();
            let batch_size = self.batch_size;
            
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                Self::migrate_batch(pool, offset, batch_size).await
            });
            
            tasks.push(task);
        }
        
        // Wait for all batches to complete
        for task in tasks {
            task.await??;
        }
        
        info!("Pipeline data migration completed");
        Ok(())
    }
    
    async fn migrate_batch(
        pool: PgPool,
        offset: usize,
        batch_size: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = pool.begin().await?;
        
        // Fetch batch of old records
        let old_records = sqlx::query(
            "SELECT id, name, config, status, created_at 
             FROM pipelines_old 
             ORDER BY id 
             LIMIT $1 OFFSET $2"
        )
        .bind(batch_size as i64)
        .bind(offset as i64)
        .fetch_all(&mut tx)
        .await?;
        
        for record in old_records {
            let id: i32 = record.get("id");
            let name: String = record.get("name");
            let old_config: String = record.get("config");
            let status: String = record.get("status");
            let created_at: chrono::DateTime<chrono::Utc> = record.get("created_at");
            
            // Transform old config to new format
            let new_config = Self::transform_config(&old_config)?;
            
            // Insert into new table
            sqlx::query(
                "INSERT INTO pipelines (id, name, config, status, retry_attempts, timeout_seconds, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                 ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 config = EXCLUDED.config,
                 status = EXCLUDED.status,
                 updated_at = CURRENT_TIMESTAMP"
            )
            .bind(id)
            .bind(name)
            .bind(new_config)
            .bind(status)
            .bind(3) // default retry_attempts
            .bind(300) // default timeout_seconds
            .bind(created_at)
            .execute(&mut tx)
            .await?;
        }
        
        tx.commit().await?;
        
        info!("Migrated batch starting at offset {}", offset);
        Ok(())
    }
    
    fn transform_config(old_config: &str) -> Result<String, serde_json::Error> {
        let old_json: serde_json::Value = serde_json::from_str(old_config)?;
        
        // Transform old format to new format
        let new_json = serde_json::json!({
            "pipeline": {
                "batch_size": old_json.get("batch_size").unwrap_or(&serde_json::json!(1000)),
                "max_parallel_tasks": old_json.get("parallel_tasks").unwrap_or(&serde_json::json!(4)),
                "retry_attempts": 3,
                "timeout": 300
            },
            "database": {
                "url": old_json.get("database_url").unwrap_or(&serde_json::json!("")),
                "pool_size": 10,
                "timeout": 30
            },
            "observability": {
                "log_level": old_json.get("log_level").unwrap_or(&serde_json::json!("info")),
                "enable_metrics": true,
                "enable_tracing": true,
                "metrics_port": 9090
            }
        });
        
        serde_json::to_string(&new_json)
    }
}
```

## Scripts de Migração

### 1. Script Principal de Migração

```bash
#!/bin/bash
# scripts/migrate.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
FROM_VERSION=""
TO_VERSION=""
DRY_RUN=false
BACKUP_DB=true
ROLLBACK=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Options:
    --from VERSION      Current version (required)
    --to VERSION        Target version (required)
    --dry-run          Run in dry-run mode (no changes made)
    --no-backup        Skip database backup
    --rollback         Rollback to previous version
    -h, --help         Show this help message

Examples:
    $0 --from 1.5.0 --to 2.0.0
    $0 --from 1.5.0 --to 2.0.0 --dry-run
    $0 --rollback --to 1.5.0
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --from)
            FROM_VERSION="$2"
            shift 2
            ;;
        --to)
            TO_VERSION="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-backup)
            BACKUP_DB=false
            shift
            ;;
        --rollback)
            ROLLBACK=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate arguments
if [ -z "$TO_VERSION" ]; then
    log_error "Target version is required"
    usage
    exit 1
fi

if [ "$ROLLBACK" = false ] && [ -z "$FROM_VERSION" ]; then
    log_error "From version is required for upgrades"
    usage
    exit 1
fi

# Main migration function
main() {
    log_info "Starting ETLRS migration"
    log_info "Target version: $TO_VERSION"
    
    if [ "$DRY_RUN" = true ]; then
        log_warning "Running in DRY-RUN mode - no changes will be made"
    fi
    
    if [ "$ROLLBACK" = true ]; then
        log_warning "Performing ROLLBACK to version $TO_VERSION"
    else
        log_info "Upgrading from version $FROM_VERSION"
    fi
    
    # Pre-flight checks
    log_info "Running pre-flight checks..."
    run_preflight_checks
    
    # Backup database
    if [ "$BACKUP_DB" = true ] && [ "$DRY_RUN" = false ]; then
        log_info "Creating database backup..."
        create_database_backup
    fi
    
    # Stop services
    if [ "$DRY_RUN" = false ]; then
        log_info "Stopping services..."
        stop_services
    fi
    
    # Execute migration
    if [ "$ROLLBACK" = true ]; then
        execute_rollback
    else
        execute_migration
    fi
    
    # Start services
    if [ "$DRY_RUN" = false ]; then
        log_info "Starting services..."
        start_services
    fi
    
    # Post-migration validation
    log_info "Running post-migration validation..."
    validate_migration
    
    log_success "Migration completed successfully!"
}

run_preflight_checks() {
    # Check if database is accessible
    if ! cargo run --bin etlrs -- health-check >/dev/null 2>&1; then
        log_error "Database health check failed"
        exit 1
    fi
    
    # Check available disk space
    AVAILABLE_SPACE=$(df . | tail -1 | awk '{print $4}')
    MIN_SPACE=1048576  # 1GB in KB
    
    if [ "$AVAILABLE_SPACE" -lt "$MIN_SPACE" ]; then
        log_error "Insufficient disk space. Available: ${AVAILABLE_SPACE}KB, Required: ${MIN_SPACE}KB"
        exit 1
    fi
    
    # Check if backup directory exists
    if [ "$BACKUP_DB" = true ]; then
        mkdir -p "$PROJECT_ROOT/backups"
    fi
    
    log_success "Pre-flight checks passed"
}

create_database_backup() {
    BACKUP_FILE="$PROJECT_ROOT/backups/etlrs_backup_$(date +%Y%m%d_%H%M%S).sql"
    
    if command -v pg_dump >/dev/null 2>&1; then
        pg_dump "$DATABASE_URL" > "$BACKUP_FILE"
        log_success "Database backup created: $BACKUP_FILE"
    else
        log_warning "pg_dump not found, skipping database backup"
    fi
}

stop_services() {
    # Stop using systemctl if available
    if command -v systemctl >/dev/null 2>&1; then
        if systemctl is-active --quiet etlrs; then
            sudo systemctl stop etlrs
            log_info "Stopped etlrs service"
        fi
    fi
    
    # Stop using docker-compose if available
    if [ -f "$PROJECT_ROOT/docker-compose.yml" ]; then
        cd "$PROJECT_ROOT"
        docker-compose down
        log_info "Stopped docker services"
    fi
}

start_services() {
    # Start using systemctl if available
    if command -v systemctl >/dev/null 2>&1; then
        if systemctl list-unit-files etlrs.service >/dev/null 2>&1; then
            sudo systemctl start etlrs
            log_info "Started etlrs service"
        fi
    fi
    
    # Start using docker-compose if available
    if [ -f "$PROJECT_ROOT/docker-compose.yml" ]; then
        cd "$PROJECT_ROOT"
        docker-compose up -d
        log_info "Started docker services"
    fi
}

execute_migration() {
    log_info "Executing migration from $FROM_VERSION to $TO_VERSION"
    
    cd "$PROJECT_ROOT"
    
    # Run migration tool
    if [ "$DRY_RUN" = true ]; then
        cargo run --bin etlrs-migrate -- \
            --from "$FROM_VERSION" \
            --to "$TO_VERSION" \
            --dry-run
    else
        cargo run --bin etlrs-migrate -- \
            --from "$FROM_VERSION" \
            --to "$TO_VERSION"
    fi
    
    log_success "Migration execution completed"
}

execute_rollback() {
    log_info "Executing rollback to version $TO_VERSION"
    
    cd "$PROJECT_ROOT"
    
    # Run rollback tool
    if [ "$DRY_RUN" = true ]; then
        cargo run --bin etlrs-migrate -- \
            --rollback \
            --to "$TO_VERSION" \
            --dry-run
    else
        cargo run --bin etlrs-migrate -- \
            --rollback \
            --to "$TO_VERSION"
    fi
    
    log_success "Rollback execution completed"
}

validate_migration() {
    # Wait for services to start
    sleep 10
    
    # Run health check
    if cargo run --bin etlrs -- health-check; then
        log_success "Health check passed"
    else
        log_error "Health check failed"
        exit 1
    fi
    
    # Run migration validation
    if cargo run --bin etlrs -- validate-migration; then
        log_success "Migration validation passed"
    else
        log_error "Migration validation failed"
        exit 1
    fi
}

# Execute main function
main
```

### 2. Ferramenta CLI de Migração

```rust
// src/bin/etlrs-migrate.rs
use clap::{App, Arg, SubCommand};
use etlrs::migration::{MigrationExecutor, MigrationPlan};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let matches = App::new("ETLRS Migration Tool")
        .version("2.0.0")
        .about("Migrates ETLRS between versions")
        .arg(Arg::with_name("from")
            .long("from")
            .value_name("VERSION")
            .help("Source version")
            .takes_value(true))
        .arg(Arg::with_name("to")
            .long("to")
            .value_name("VERSION")
            .help("Target version")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("dry-run")
            .long("dry-run")
            .help("Run in dry-run mode"))
        .arg(Arg::with_name("rollback")
            .long("rollback")
            .help("Perform rollback"))
        .get_matches();
    
    let from_version = matches.value_of("from");
    let to_version = matches.value_of("to").unwrap();
    let dry_run = matches.is_present("dry-run");
    let rollback = matches.is_present("rollback");
    
    if rollback {
        execute_rollback(to_version, dry_run).await?;
    } else {
        let from_version = from_version.ok_or("From version required for migration")?;
        execute_migration(from_version, to_version, dry_run).await?;
    }
    
    Ok(())
}

async fn execute_migration(
    from: &str,
    to: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading migration plan for {} -> {}", from, to);
    
    let plan = load_migration_plan(from, to)?;
    let executor = MigrationExecutor::new(from, to, dry_run);
    
    let result = executor.execute_migration(&plan).await?;
    
    info!("Migration completed in {:?}", result.total_duration);
    Ok(())
}

async fn execute_rollback(
    to: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing rollback to version {}", to);
    
    // Implementation for rollback
    // This would load the appropriate rollback plan and execute it
    
    Ok(())
}

fn load_migration_plan(from: &str, to: &str) -> Result<MigrationPlan, Box<dyn std::error::Error>> {
    let plan_file = format!("migrations/plans/{}_{}.json", from, to);
    let plan_content = std::fs::read_to_string(&plan_file)?;
    let plan: MigrationPlan = serde_json::from_str(&plan_content)?;
    
    Ok(plan)
}
```

## Procedimentos de Rollback

### 1. Rollback Automático

```rust
// migration/rollback.rs
use std::collections::HashMap;

pub struct RollbackManager {
    rollback_history: HashMap<String, RollbackPoint>,
}

#[derive(Debug, Clone)]
pub struct RollbackPoint {
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub config_backup: String,
    pub schema_version: i32,
    pub data_snapshot: Option<String>,
}

impl RollbackManager {
    pub fn new() -> Self {
        Self {
            rollback_history: HashMap::new(),
        }
    }
    
    pub async fn create_rollback_point(&mut self, version: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating rollback point for version {}", version);
        
        let rollback_point = RollbackPoint {
            version: version.to_string(),
            timestamp: chrono::Utc::now(),
            config_backup: self.backup_config().await?,
            schema_version: self.get_schema_version().await?,
            data_snapshot: self.create_data_snapshot().await?,
        };
        
        self.rollback_history.insert(version.to_string(), rollback_point);
        self.save_rollback_history().await?;
        
        info!("Rollback point created for version {}", version);
        Ok(())
    }
    
    pub async fn rollback_to(&self, version: &str) -> Result<(), Box<dyn std::error::Error>> {
        let rollback_point = self.rollback_history.get(version)
            .ok_or_else(|| format!("No rollback point found for version {}", version))?;
        
        info!("Rolling back to version {} (created at {})", 
            version, rollback_point.timestamp);
        
        // Rollback configuration
        self.restore_config(&rollback_point.config_backup).await?;
        
        // Rollback schema
        self.rollback_schema(rollback_point.schema_version).await?;
        
        // Rollback data if snapshot exists
        if let Some(snapshot) = &rollback_point.data_snapshot {
            self.restore_data_snapshot(snapshot).await?;
        }
        
        info!("Rollback to version {} completed", version);
        Ok(())
    }
    
    async fn backup_config(&self) -> Result<String, Box<dyn std::error::Error>> {
        let config_content = tokio::fs::read_to_string("config/config.toml").await?;
        Ok(config_content)
    }
    
    async fn restore_config(&self, backup: &str) -> Result<(), Box<dyn std::error::Error>> {
        tokio::fs::write("config/config.toml", backup).await?;
        Ok(())
    }
    
    async fn get_schema_version(&self) -> Result<i32, Box<dyn std::error::Error>> {
        // Implementation to get current schema version
        Ok(2) // Placeholder
    }
    
    async fn rollback_schema(&self, target_version: i32) -> Result<(), Box<dyn std::error::Error>> {
        info!("Rolling back schema to version {}", target_version);
        // Implementation to rollback schema
        Ok(())
    }
    
    async fn create_data_snapshot(&self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // Create data snapshot if needed
        Ok(None) // Placeholder
    }
    
    async fn restore_data_snapshot(&self, snapshot: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Restoring data snapshot: {}", snapshot);
        // Implementation to restore data
        Ok(())
    }
    
    async fn save_rollback_history(&self) -> Result<(), Box<dyn std::error::Error>> {
        let history_json = serde_json::to_string_pretty(&self.rollback_history)?;
        tokio::fs::write("rollback_history.json", history_json).await?;
        Ok(())
    }
}
```

### 2. Script de Rollback de Emergência

```bash
#!/bin/bash
# scripts/emergency-rollback.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
TARGET_VERSION=""
BACKUP_DATE=""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

usage() {
    cat << EOF
Emergency Rollback Script

Usage: $0 --version VERSION [--backup-date YYYYMMDD_HHMMSS]

Options:
    --version VERSION           Target version to rollback to
    --backup-date DATE          Specific backup date to restore
    -h, --help                  Show this help

Examples:
    $0 --version 1.5.0
    $0 --version 1.5.0 --backup-date 20231201_140000
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            TARGET_VERSION="$2"
            shift 2
            ;;
        --backup-date)
            BACKUP_DATE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

if [ -z "$TARGET_VERSION" ]; then
    log_error "Target version is required"
    usage
    exit 1
fi

main() {
    log_warning "EMERGENCY ROLLBACK TO VERSION $TARGET_VERSION"
    log_warning "This will stop all services and restore from backup"
    
    read -p "Are you sure you want to continue? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        echo "Rollback cancelled"
        exit 0
    fi
    
    # Stop all services immediately
    log_warning "Stopping all services..."
    stop_all_services
    
    # Find and restore backup
    if [ -n "$BACKUP_DATE" ]; then
        BACKUP_FILE="$PROJECT_ROOT/backups/etlrs_backup_${BACKUP_DATE}.sql"
    else
        # Find most recent backup
        BACKUP_FILE=$(ls -t "$PROJECT_ROOT/backups/etlrs_backup_"*.sql 2>/dev/null | head -1)
    fi
    
    if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
        log_error "No backup file found"
        exit 1
    fi
    
    log_warning "Restoring database from: $BACKUP_FILE"
    restore_database "$BACKUP_FILE"
    
    # Restore configuration
    CONFIG_BACKUP="$PROJECT_ROOT/backups/config_${TARGET_VERSION}.toml"
    if [ -f "$CONFIG_BACKUP" ]; then
        log_warning "Restoring configuration"
        cp "$CONFIG_BACKUP" "$PROJECT_ROOT/config/config.toml"
    fi
    
    # Start services
    log_warning "Starting services..."
    start_services
    
    # Verify rollback
    log_warning "Verifying rollback..."
    if cargo run --bin etlrs -- health-check; then
        log_success "Emergency rollback completed successfully"
    else
        log_error "Health check failed after rollback"
        exit 1
    fi
}

stop_all_services() {
    # Force stop systemd service
    if command -v systemctl >/dev/null 2>&1; then
        sudo systemctl stop etlrs || true
    fi
    
    # Force stop docker containers
    if command -v docker >/dev/null 2>&1; then
        docker stop $(docker ps -q --filter "name=etlrs") || true
    fi
    
    # Kill any remaining processes
    pkill -f etlrs || true
}

restore_database() {
    local backup_file="$1"
    
    # Drop and recreate database
    psql "$DATABASE_URL" -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;" || {
        log_error "Failed to reset database"
        exit 1
    }
    
    # Restore from backup
    psql "$DATABASE_URL" < "$backup_file" || {
        log_error "Failed to restore database"
        exit 1
    }
    
    log_success "Database restored from backup"
}

start_services() {
    if command -v systemctl >/dev/null 2>&1; then
        sudo systemctl start etlrs || true
    fi
    
    if [ -f "$PROJECT_ROOT/docker-compose.yml" ]; then
        cd "$PROJECT_ROOT"
        docker-compose up -d || true
    fi
}

# Execute main function
main
```

Este guia de migração fornece uma base completa para gerenciar migrações da biblioteca ETLRS de forma segura e confiável, incluindo procedimentos de rollback para situações de emergência.
