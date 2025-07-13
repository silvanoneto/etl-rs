# 🚀 Guia de Deployment

## Visão Geral

Este guia aborda as estratégias e melhores práticas para fazer deployment da biblioteca ETLRS em diferentes ambientes, desde desenvolvimento local até produção enterprise em multi-cloud. Cobrimos containerização, orquestração, CI/CD, monitoramento e estratégias de rollback.

## Estratégias de Deployment

### 1. Ambientes

```yaml
# environments.yml
environments:
  development:
    replicas: 1
    resources:
      cpu: "100m"
      memory: "256Mi"
    monitoring: basic
    security: relaxed
    
  staging:
    replicas: 2
    resources:
      cpu: "500m"
      memory: "1Gi"
    monitoring: standard
    security: standard
    
  production:
    replicas: 5
    resources:
      cpu: "2000m"
      memory: "4Gi"
    monitoring: comprehensive
    security: strict
    high_availability: true
    backup: enabled
```

### 2. Tipos de Deployment

#### Blue-Green Deployment
```yaml
# blue-green-deployment.yml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: etlrs-pipeline
spec:
  replicas: 5
  strategy:
    blueGreen:
      activeService: etlrs-active
      previewService: etlrs-preview
      autoPromotionEnabled: false
      scaleDownDelaySeconds: 30
      prePromotionAnalysis:
        templates:
        - templateName: success-rate
        args:
        - name: service-name
          value: etlrs-preview
      postPromotionAnalysis:
        templates:
        - templateName: success-rate
        args:
        - name: service-name
          value: etlrs-active
```

#### Canary Deployment
```yaml
# canary-deployment.yml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: etlrs-pipeline
spec:
  replicas: 10
  strategy:
    canary:
      steps:
      - setWeight: 10
      - pause: {duration: 60s}
      - setWeight: 20
      - pause: {duration: 60s}
      - setWeight: 50
      - pause: {duration: 60s}
      - setWeight: 100
      analysis:
        templates:
        - templateName: error-rate
        - templateName: response-time
        startingStep: 2
        interval: 30s
```

## Containerização

### 1. Dockerfile Otimizado

```dockerfile
# Dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/etlrs*

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false etlrs

# Copy binary
COPY --from=builder /app/target/release/etlrs /usr/local/bin/etlrs

# Copy configuration
COPY config/ /etc/etlrs/

# Set ownership
RUN chown -R etlrs:etlrs /etc/etlrs

# Switch to non-root user
USER etlrs

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
  CMD etlrs health-check || exit 1

EXPOSE 8080

CMD ["etlrs", "server", "--config", "/etc/etlrs/config.yml"]
```

### 2. Docker Compose para Desenvolvimento

```yaml
# docker-compose.yml
version: '3.8'

services:
  etlrs:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
      - "9090:9090"  # Metrics
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://etlrs:password@postgres:5432/etlrs
      - REDIS_URL=redis://redis:6379
    volumes:
      - ./config:/etc/etlrs
      - ./data:/data
    depends_on:
      - postgres
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "etlrs", "health-check"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=etlrs
      - POSTGRES_USER=etlrs
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./sql/init.sql:/docker-entrypoint-initdb.d/init.sql
    ports:
      - "5432:5432"
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./monitoring/grafana/datasources:/etc/grafana/provisioning/datasources

volumes:
  postgres_data:
  redis_data:
  prometheus_data:
  grafana_data:
```

## Kubernetes

### 1. Manifests Base

```yaml
# k8s/namespace.yml
apiVersion: v1
kind: Namespace
metadata:
  name: etlrs
  labels:
    name: etlrs
    environment: production

---
# k8s/configmap.yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: etlrs-config
  namespace: etlrs
data:
  config.yml: |
    pipeline:
      max_parallel_tasks: 10
      retry_attempts: 3
      timeout_seconds: 300
    integrity:
      enable_checksums: true
      checksum_type: "sha256"
    performance:
      max_threads: 8
      batch_size: 10000
    observability:
      enable_metrics: true
      metrics_port: 9090
      tracing_level: "info"

---
# k8s/secret.yml
apiVersion: v1
kind: Secret
metadata:
  name: etlrs-secrets
  namespace: etlrs
type: Opaque
data:
  database-url: cG9zdGdyZXNxbDovL2V0bHJzOnBhc3N3b3JkQHBvc3RncmVzOjU0MzIvZXRscnM=
  redis-url: cmVkaXM6Ly9yZWRpczozNzk=
  jwt-secret: c3VwZXJfc2VjcmV0X2tleQ==

---
# k8s/deployment.yml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: etlrs-deployment
  namespace: etlrs
  labels:
    app: etlrs
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: etlrs
  template:
    metadata:
      labels:
        app: etlrs
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 2000
      containers:
      - name: etlrs
        image: etlrs:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: etlrs-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: etlrs-secrets
              key: redis-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: etlrs-secrets
              key: jwt-secret
        volumeMounts:
        - name: config
          mountPath: /etc/etlrs
          readOnly: true
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          successThreshold: 1
          failureThreshold: 3
      volumes:
      - name: config
        configMap:
          name: etlrs-config
      - name: data
        persistentVolumeClaim:
          claimName: etlrs-data-pvc

---
# k8s/service.yml
apiVersion: v1
kind: Service
metadata:
  name: etlrs-service
  namespace: etlrs
  labels:
    app: etlrs
spec:
  type: ClusterIP
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
    name: http
  - port: 9090
    targetPort: 9090
    protocol: TCP
    name: metrics
  selector:
    app: etlrs

---
# k8s/ingress.yml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: etlrs-ingress
  namespace: etlrs
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
  - hosts:
    - etlrs.example.com
    secretName: etlrs-tls
  rules:
  - host: etlrs.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: etlrs-service
            port:
              number: 80

---
# k8s/pvc.yml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: etlrs-data-pvc
  namespace: etlrs
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: fast-ssd
  resources:
    requests:
      storage: 100Gi
```

### 2. Helm Chart

```yaml
# helm/etlrs/Chart.yaml
apiVersion: v2
name: etlrs
description: A Helm chart for ETLRS pipeline
type: application
version: 0.1.0
appVersion: "1.0.0"
dependencies:
- name: postgresql
  version: 12.x.x
  repository: https://charts.bitnami.com/bitnami
  condition: postgresql.enabled
- name: redis
  version: 17.x.x
  repository: https://charts.bitnami.com/bitnami
  condition: redis.enabled

---
# helm/etlrs/values.yaml
# Default values for etlrs
replicaCount: 3

image:
  repository: etlrs
  pullPolicy: IfNotPresent
  tag: "latest"

imagePullSecrets: []
nameOverride: ""
fullnameOverride: ""

serviceAccount:
  create: true
  annotations: {}
  name: ""

podAnnotations:
  prometheus.io/scrape: "true"
  prometheus.io/port: "9090"
  prometheus.io/path: "/metrics"

podSecurityContext:
  fsGroup: 2000
  runAsNonRoot: true
  runAsUser: 1000

securityContext:
  allowPrivilegeEscalation: false
  capabilities:
    drop:
    - ALL
  readOnlyRootFilesystem: true

service:
  type: ClusterIP
  port: 80
  targetPort: 8080

ingress:
  enabled: true
  className: "nginx"
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
  hosts:
    - host: etlrs.local
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: etlrs-tls
      hosts:
        - etlrs.local

resources:
  limits:
    cpu: 1000m
    memory: 2Gi
  requests:
    cpu: 500m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

nodeSelector: {}
tolerations: []
affinity: {}

persistence:
  enabled: true
  storageClass: "fast-ssd"
  size: 100Gi

postgresql:
  enabled: true
  auth:
    postgresPassword: "password"
    username: "etlrs"
    password: "password"
    database: "etlrs"

redis:
  enabled: true
  auth:
    enabled: false

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
    interval: 30s
    path: /metrics
    port: metrics

config:
  pipeline:
    max_parallel_tasks: 10
    retry_attempts: 3
    timeout_seconds: 300
  integrity:
    enable_checksums: true
    checksum_type: "sha256"
  performance:
    max_threads: 8
    batch_size: 10000
  observability:
    enable_metrics: true
    metrics_port: 9090
    tracing_level: "info"
```

## CI/CD Pipeline

### 1. GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: etlrs_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
      
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379

    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Run tests
      run: cargo test --all-features
      env:
        DATABASE_URL: postgres://postgres:postgres@localhost:5432/etlrs_test
        REDIS_URL: redis://localhost:6379
    
    - name: Run integration tests
      run: cargo test --test integration -- --nocapture
      env:
        DATABASE_URL: postgres://postgres:postgres@localhost:5432/etlrs_test
        REDIS_URL: redis://localhost:6379
    
    - name: Generate test coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --out xml
    
    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        file: ./cobertura.xml
        
  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
    
    - name: Security audit
      run: |
        cargo install cargo-audit
        cargo audit
    
    - name: Dependency check
      run: |
        cargo install cargo-deny
        cargo deny check

  build:
    needs: [test, security]
    runs-on: ubuntu-latest
    
    steps:
    - name: Checkout repository
      uses: actions/checkout@v4
    
    - name: Log in to Container Registry
      uses: docker/login-action@v2
      with:
        registry: ${{ env.REGISTRY }}
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
    
    - name: Extract metadata
      id: meta
      uses: docker/metadata-action@v4
      with:
        images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
        tags: |
          type=ref,event=branch
          type=ref,event=pr
          type=sha,prefix={{branch}}-
          type=raw,value=latest,enable={{is_default_branch}}
    
    - name: Build and push Docker image
      uses: docker/build-push-action@v4
      with:
        context: .
        push: true
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}
        cache-from: type=gha
        cache-to: type=gha,mode=max

---
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [ main ]
    tags: [ 'v*' ]

jobs:
  deploy-staging:
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    environment: staging
    
    steps:
    - name: Checkout
      uses: actions/checkout@v4
    
    - name: Configure kubectl
      uses: azure/k8s-set-context@v3
      with:
        method: kubeconfig
        kubeconfig: ${{ secrets.KUBE_CONFIG_STAGING }}
    
    - name: Deploy to staging
      run: |
        helm upgrade --install etlrs-staging ./helm/etlrs \
          --namespace etlrs-staging \
          --create-namespace \
          --set image.tag=${{ github.sha }} \
          --set ingress.hosts[0].host=etlrs-staging.example.com \
          --values ./helm/etlrs/values-staging.yaml
    
    - name: Wait for deployment
      run: |
        kubectl rollout status deployment/etlrs-staging -n etlrs-staging --timeout=300s
    
    - name: Run smoke tests
      run: |
        ./scripts/smoke-tests.sh https://etlrs-staging.example.com

  deploy-production:
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    environment: production
    needs: [deploy-staging]
    
    steps:
    - name: Checkout
      uses: actions/checkout@v4
    
    - name: Configure kubectl
      uses: azure/k8s-set-context@v3
      with:
        method: kubeconfig
        kubeconfig: ${{ secrets.KUBE_CONFIG_PRODUCTION }}
    
    - name: Deploy to production
      run: |
        helm upgrade --install etlrs ./helm/etlrs \
          --namespace etlrs \
          --create-namespace \
          --set image.tag=${{ github.ref_name }} \
          --set ingress.hosts[0].host=etlrs.example.com \
          --values ./helm/etlrs/values-production.yaml
    
    - name: Wait for deployment
      run: |
        kubectl rollout status deployment/etlrs -n etlrs --timeout=600s
    
    - name: Run production tests
      run: |
        ./scripts/production-tests.sh https://etlrs.example.com
```

### 2. GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - test
  - security
  - build
  - deploy

variables:
  DOCKER_DRIVER: overlay2
  DOCKER_TLS_CERTDIR: "/certs"
  REGISTRY: $CI_REGISTRY
  IMAGE_NAME: $CI_PROJECT_PATH

.rust_cache: &rust_cache
  cache:
    key: ${CI_COMMIT_REF_SLUG}
    paths:
      - target/
      - ~/.cargo/

test:
  stage: test
  image: rust:1.75
  services:
    - postgres:15
    - redis:7
  variables:
    POSTGRES_DB: etlrs_test
    POSTGRES_USER: postgres
    POSTGRES_PASSWORD: postgres
    DATABASE_URL: postgres://postgres:postgres@postgres:5432/etlrs_test
    REDIS_URL: redis://redis:6379
  <<: *rust_cache
  before_script:
    - apt-get update && apt-get install -y postgresql-client
    - rustup component add rustfmt clippy
  script:
    - cargo fmt --all -- --check
    - cargo clippy --all-targets --all-features -- -D warnings
    - cargo test --all-features
    - cargo test --test integration
  coverage: '/^\d+\.\d+% coverage/'
  artifacts:
    reports:
      coverage_report:
        coverage_format: cobertura
        path: cobertura.xml

security:
  stage: security
  image: rust:1.75
  <<: *rust_cache
  script:
    - cargo install cargo-audit
    - cargo audit
    - cargo install cargo-deny
    - cargo deny check
  allow_failure: false

build:
  stage: build
  image: docker:24
  services:
    - docker:24-dind
  before_script:
    - docker login -u $CI_REGISTRY_USER -p $CI_REGISTRY_PASSWORD $CI_REGISTRY
  script:
    - |
      if [[ "$CI_COMMIT_BRANCH" == "$CI_DEFAULT_BRANCH" ]]; then
        tag="latest"
      else
        tag=$CI_COMMIT_REF_SLUG
      fi
    - docker build -t $REGISTRY/$IMAGE_NAME:$tag .
    - docker push $REGISTRY/$IMAGE_NAME:$tag
  only:
    - main
    - develop
    - tags

.deploy_template: &deploy_template
  image: alpine/helm:latest
  before_script:
    - apk add --no-cache kubectl
    - mkdir -p ~/.kube
    - echo "$KUBE_CONFIG" | base64 -d > ~/.kube/config

deploy_staging:
  <<: *deploy_template
  stage: deploy
  environment:
    name: staging
    url: https://etlrs-staging.example.com
  variables:
    KUBE_CONFIG: $KUBE_CONFIG_STAGING
  script:
    - |
      helm upgrade --install etlrs-staging ./helm/etlrs \
        --namespace etlrs-staging \
        --create-namespace \
        --set image.tag=$CI_COMMIT_SHA \
        --set ingress.hosts[0].host=etlrs-staging.example.com \
        --values ./helm/etlrs/values-staging.yaml
    - kubectl rollout status deployment/etlrs-staging -n etlrs-staging --timeout=300s
  only:
    - main

deploy_production:
  <<: *deploy_template
  stage: deploy
  environment:
    name: production
    url: https://etlrs.example.com
  variables:
    KUBE_CONFIG: $KUBE_CONFIG_PRODUCTION
  script:
    - |
      helm upgrade --install etlrs ./helm/etlrs \
        --namespace etlrs \
        --create-namespace \
        --set image.tag=$CI_COMMIT_TAG \
        --set ingress.hosts[0].host=etlrs.example.com \
        --values ./helm/etlrs/values-production.yaml
    - kubectl rollout status deployment/etlrs -n etlrs --timeout=600s
  only:
    - tags
  when: manual
```

## Monitoramento de Deployment

### 1. Health Checks

```rust
// src/health.rs
use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    version: String,
    checks: HashMap<String, HealthCheck>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    status: String,
    message: Option<String>,
    duration_ms: u64,
}

pub async fn health_handler(State(health_checker): State<HealthChecker>) -> Result<Json<HealthResponse>, StatusCode> {
    let start = std::time::Instant::now();
    let checks = health_checker.run_all_checks().await;
    
    let overall_status = if checks.values().all(|check| check.status == "healthy") {
        "healthy"
    } else {
        "unhealthy"
    };
    
    Ok(Json(HealthResponse {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
    }))
}

pub async fn readiness_handler(State(health_checker): State<HealthChecker>) -> Result<Json<HealthResponse>, StatusCode> {
    let checks = health_checker.run_readiness_checks().await;
    
    let ready = checks.values().all(|check| check.status == "ready");
    
    if ready {
        Ok(Json(HealthResponse {
            status: "ready".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks,
        }))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

pub struct HealthChecker {
    database_pool: DatabasePool,
    redis_client: RedisClient,
    external_services: Vec<ExternalService>,
}

impl HealthChecker {
    pub async fn run_all_checks(&self) -> HashMap<String, HealthCheck> {
        let mut checks = HashMap::new();
        
        // Database check
        let db_start = std::time::Instant::now();
        let db_check = match self.check_database().await {
            Ok(_) => HealthCheck {
                status: "healthy".to_string(),
                message: None,
                duration_ms: db_start.elapsed().as_millis() as u64,
            },
            Err(e) => HealthCheck {
                status: "unhealthy".to_string(),
                message: Some(e.to_string()),
                duration_ms: db_start.elapsed().as_millis() as u64,
            },
        };
        checks.insert("database".to_string(), db_check);
        
        // Redis check
        let redis_start = std::time::Instant::now();
        let redis_check = match self.check_redis().await {
            Ok(_) => HealthCheck {
                status: "healthy".to_string(),
                message: None,
                duration_ms: redis_start.elapsed().as_millis() as u64,
            },
            Err(e) => HealthCheck {
                status: "unhealthy".to_string(),
                message: Some(e.to_string()),
                duration_ms: redis_start.elapsed().as_millis() as u64,
            },
        };
        checks.insert("redis".to_string(), redis_check);
        
        checks
    }
    
    async fn check_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("SELECT 1").execute(&self.database_pool).await?;
        Ok(())
    }
    
    async fn check_redis(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.redis_client.ping().await?;
        Ok(())
    }
}
```

### 2. Métricas de Deployment

```rust
// src/metrics/deployment.rs
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct DeploymentMetrics {
    pub deployment_count: Counter,
    pub deployment_duration: Histogram,
    pub rollback_count: Counter,
    pub health_check_failures: Counter,
    pub active_connections: Gauge,
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
}

impl DeploymentMetrics {
    pub fn new(registry: &Registry) -> Self {
        let deployment_count = Counter::new("deployments_total", "Total number of deployments").unwrap();
        let deployment_duration = Histogram::new("deployment_duration_seconds", "Deployment duration").unwrap();
        let rollback_count = Counter::new("rollbacks_total", "Total number of rollbacks").unwrap();
        let health_check_failures = Counter::new("health_check_failures_total", "Health check failures").unwrap();
        let active_connections = Gauge::new("active_connections", "Number of active connections").unwrap();
        let memory_usage = Gauge::new("memory_usage_bytes", "Memory usage in bytes").unwrap();
        let cpu_usage = Gauge::new("cpu_usage_percent", "CPU usage percentage").unwrap();
        
        registry.register(Box::new(deployment_count.clone())).unwrap();
        registry.register(Box::new(deployment_duration.clone())).unwrap();
        registry.register(Box::new(rollback_count.clone())).unwrap();
        registry.register(Box::new(health_check_failures.clone())).unwrap();
        registry.register(Box::new(active_connections.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        
        Self {
            deployment_count,
            deployment_duration,
            rollback_count,
            health_check_failures,
            active_connections,
            memory_usage,
            cpu_usage,
        }
    }
    
    pub fn record_deployment(&self, duration: std::time::Duration, success: bool) {
        self.deployment_count.inc();
        self.deployment_duration.observe(duration.as_secs_f64());
        
        if !success {
            self.rollback_count.inc();
        }
    }
}
```

## Scripts de Deployment

### 1. Script de Smoke Tests

```bash
#!/bin/bash
# scripts/smoke-tests.sh

set -e

BASE_URL=$1
if [ -z "$BASE_URL" ]; then
    echo "Usage: $0 <base-url>"
    exit 1
fi

echo "Running smoke tests against $BASE_URL"

# Test health endpoint
echo "Testing health endpoint..."
response=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health")
if [ "$response" != "200" ]; then
    echo "Health check failed with status $response"
    exit 1
fi
echo "✓ Health check passed"

# Test readiness endpoint
echo "Testing readiness endpoint..."
response=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/ready")
if [ "$response" != "200" ]; then
    echo "Readiness check failed with status $response"
    exit 1
fi
echo "✓ Readiness check passed"

# Test metrics endpoint
echo "Testing metrics endpoint..."
response=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/metrics")
if [ "$response" != "200" ]; then
    echo "Metrics check failed with status $response"
    exit 1
fi
echo "✓ Metrics check passed"

# Test API endpoints
echo "Testing API endpoints..."
response=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/pipelines")
if [ "$response" != "200" ] && [ "$response" != "401" ]; then
    echo "API check failed with status $response"
    exit 1
fi
echo "✓ API check passed"

echo "All smoke tests passed!"
```

### 2. Script de Rollback

```bash
#!/bin/bash
# scripts/rollback.sh

set -e

NAMESPACE=${1:-etlrs}
RELEASE_NAME=${2:-etlrs}

echo "Rolling back $RELEASE_NAME in namespace $NAMESPACE"

# Get current revision
current_revision=$(helm status $RELEASE_NAME -n $NAMESPACE -o json | jq -r '.version')
echo "Current revision: $current_revision"

if [ "$current_revision" -le 1 ]; then
    echo "Cannot rollback: no previous revision available"
    exit 1
fi

# Calculate target revision
target_revision=$((current_revision - 1))
echo "Rolling back to revision: $target_revision"

# Perform rollback
helm rollback $RELEASE_NAME $target_revision -n $NAMESPACE

# Wait for rollback to complete
echo "Waiting for rollback to complete..."
kubectl rollout status deployment/$RELEASE_NAME -n $NAMESPACE --timeout=600s

# Run smoke tests
echo "Running smoke tests after rollback..."
./scripts/smoke-tests.sh "https://etlrs.example.com"

echo "Rollback completed successfully!"
```

## Troubleshooting

### 1. Logs Aggregation

```yaml
# monitoring/fluentd-config.yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/containers/etlrs-*.log
      pos_file /var/log/fluentd-etlrs.log.pos
      tag kubernetes.etlrs
      format json
      time_key time
      time_format %Y-%m-%dT%H:%M:%S.%NZ
    </source>
    
    <filter kubernetes.etlrs>
      @type parser
      key_name log
      <parse>
        @type json
        time_key timestamp
        time_format %Y-%m-%dT%H:%M:%S%.%LZ
      </parse>
    </filter>
    
    <match kubernetes.etlrs>
      @type elasticsearch
      host elasticsearch.logging.svc.cluster.local
      port 9200
      index_name etlrs-logs
      type_name _doc
    </match>
```

### 2. Debugging Tools

```bash
#!/bin/bash
# scripts/debug.sh

NAMESPACE=${1:-etlrs}
POD_NAME=${2}

if [ -z "$POD_NAME" ]; then
    echo "Available pods:"
    kubectl get pods -n $NAMESPACE
    echo "Usage: $0 <namespace> <pod-name>"
    exit 1
fi

echo "Debugging pod $POD_NAME in namespace $NAMESPACE"

# Get pod logs
echo "=== Pod Logs ==="
kubectl logs $POD_NAME -n $NAMESPACE --tail=100

# Get pod events
echo "=== Pod Events ==="
kubectl describe pod $POD_NAME -n $NAMESPACE | grep -A 20 "Events:"

# Get resource usage
echo "=== Resource Usage ==="
kubectl top pod $POD_NAME -n $NAMESPACE

# Execute into pod for debugging
echo "=== Entering pod for debugging ==="
kubectl exec -it $POD_NAME -n $NAMESPACE -- /bin/sh
```

Este guia fornece uma base sólida para fazer deployment da biblioteca ETLRS de forma segura e confiável em qualquer ambiente, desde desenvolvimento local até produção enterprise.
