# 🔒 Guia de Segurança

## Visão Geral

A segurança é um aspecto crítico em pipelines ETL, especialmente quando lidamos com dados sensíveis ou sistemas em produção. Este guia aborda as melhores práticas de segurança implementadas na biblioteca ETLRS, desde criptografia até controle de acesso e compliance.

## Princípios de Segurança

### 1. Defesa em Profundidade
- **Múltiplas camadas**: Implementar segurança em todas as camadas da aplicação
- **Princípio do menor privilégio**: Conceder apenas as permissões mínimas necessárias
- **Fail secure**: Em caso de falha, o sistema deve falhar de forma segura

### 2. Zero Trust Architecture
- **Never trust, always verify**: Verificar e validar todas as requisições
- **Micro-segmentação**: Isolar componentes e limitir blast radius
- **Continuous monitoring**: Monitorar continuamente atividades suspeitas

### 3. Security by Design
- **Secure defaults**: Configurações seguras por padrão
- **Privacy by design**: Proteger dados pessoais desde o início
- **Threat modeling**: Identificar e mitigar ameaças desde o design

## Criptografia

### 1. Criptografia de Dados em Trânsito

```rust
// src/security/encryption.rs
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

pub struct EncryptionManager {
    key: LessSafeKey,
    rng: SystemRandom,
}

impl EncryptionManager {
    pub fn new(key_bytes: &[u8]) -> Result<Self, SecurityError> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, key_bytes)
            .map_err(|_| SecurityError::InvalidKey)?;
        
        Ok(Self {
            key: LessSafeKey::new(unbound_key),
            rng: SystemRandom::new(),
        })
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptedData, SecurityError> {
        let mut nonce_bytes = vec![0u8; 12];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| SecurityError::RandomGenerationFailed)?;
        
        let nonce = Nonce::assume_unique_for_key(nonce_bytes.clone().try_into().unwrap());
        
        let mut encrypted_data = data.to_vec();
        self.key.seal_in_place_append_tag(nonce, Aad::empty(), &mut encrypted_data)
            .map_err(|_| SecurityError::EncryptionFailed)?;
        
        Ok(EncryptedData {
            nonce: nonce_bytes,
            ciphertext: encrypted_data,
        })
    }
    
    pub fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, SecurityError> {
        let nonce = Nonce::assume_unique_for_key(
            encrypted.nonce.clone().try_into()
                .map_err(|_| SecurityError::InvalidNonce)?
        );
        
        let mut data = encrypted.ciphertext.clone();
        let plaintext = self.key.open_in_place(nonce, Aad::empty(), &mut data)
            .map_err(|_| SecurityError::DecryptionFailed)?;
        
        Ok(plaintext.to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}
```

### 2. Criptografia de Dados em Repouso

```rust
// Criptografia de campo específico
pub struct FieldEncryption {
    encryption_manager: EncryptionManager,
    encrypted_fields: HashSet<String>,
}

impl FieldEncryption {
    pub fn encrypt_record(&self, mut record: Record) -> Result<Record, SecurityError> {
        for field_name in &self.encrypted_fields {
            if let Some(value) = record.fields.get(field_name) {
                let serialized = serde_json::to_vec(value)?;
                let encrypted = self.encryption_manager.encrypt(&serialized)?;
                let encrypted_value = Value::String(base64::encode(&encrypted.ciphertext));
                record.fields.insert(field_name.clone(), encrypted_value);
            }
        }
        Ok(record)
    }
    
    pub fn decrypt_record(&self, mut record: Record) -> Result<Record, SecurityError> {
        for field_name in &self.encrypted_fields {
            if let Some(Value::String(encrypted_str)) = record.fields.get(field_name) {
                let encrypted_data = base64::decode(encrypted_str)?;
                let decrypted = self.encryption_manager.decrypt(&EncryptedData {
                    nonce: encrypted_data[..12].to_vec(),
                    ciphertext: encrypted_data[12..].to_vec(),
                })?;
                let value: Value = serde_json::from_slice(&decrypted)?;
                record.fields.insert(field_name.clone(), value);
            }
        }
        Ok(record)
    }
}
```

### 3. Gestão de Chaves

```rust
// src/security/key_management.rs
use aws_sdk_kms as kms;

pub trait KeyStore {
    async fn get_key(&self, key_id: &str) -> Result<Vec<u8>, SecurityError>;
    async fn rotate_key(&self, key_id: &str) -> Result<String, SecurityError>;
    async fn create_key(&self, purpose: KeyPurpose) -> Result<String, SecurityError>;
}

pub struct KmsKeyStore {
    client: kms::Client,
}

impl KeyStore for KmsKeyStore {
    async fn get_key(&self, key_id: &str) -> Result<Vec<u8>, SecurityError> {
        let request = self.client
            .generate_data_key()
            .key_id(key_id)
            .key_spec(kms::types::DataKeySpec::Aes256);
        
        let response = request.send().await
            .map_err(SecurityError::KmsError)?;
        
        Ok(response.plaintext().unwrap().as_ref().to_vec())
    }
    
    async fn rotate_key(&self, key_id: &str) -> Result<String, SecurityError> {
        let request = self.client.enable_key_rotation().key_id(key_id);
        request.send().await.map_err(SecurityError::KmsError)?;
        Ok(key_id.to_string())
    }
    
    async fn create_key(&self, purpose: KeyPurpose) -> Result<String, SecurityError> {
        let description = match purpose {
            KeyPurpose::DataEncryption => "ETL Data Encryption Key",
            KeyPurpose::ConfigEncryption => "ETL Config Encryption Key",
            KeyPurpose::BackupEncryption => "ETL Backup Encryption Key",
        };
        
        let request = self.client
            .create_key()
            .description(description)
            .key_usage(kms::types::KeyUsageType::EncryptDecrypt);
        
        let response = request.send().await
            .map_err(SecurityError::KmsError)?;
        
        Ok(response.key_metadata().unwrap().key_id().to_string())
    }
}

#[derive(Debug, Clone)]
pub enum KeyPurpose {
    DataEncryption,
    ConfigEncryption,
    BackupEncryption,
}
```

## Controle de Acesso

### 1. Autenticação

```rust
// src/security/authentication.rs
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Clone)]
pub struct AuthenticationManager {
    jwt_secret: Vec<u8>,
    token_expiration: Duration,
    providers: Vec<Box<dyn AuthProvider>>,
}

pub trait AuthProvider {
    async fn authenticate(&self, credentials: &Credentials) -> Result<Identity, SecurityError>;
    fn provider_type(&self) -> AuthProviderType;
}

#[derive(Debug, Clone)]
pub enum AuthProviderType {
    Local,
    Ldap,
    OAuth2,
    Saml,
    ApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // Subject (user ID)
    pub roles: Vec<String>,    // User roles
    pub permissions: Vec<String>, // Specific permissions
    pub exp: i64,             // Expiration time
    pub iat: i64,             // Issued at
    pub iss: String,          // Issuer
}

impl AuthenticationManager {
    pub async fn authenticate(&self, credentials: &Credentials) -> Result<AuthToken, SecurityError> {
        for provider in &self.providers {
            match provider.authenticate(credentials).await {
                Ok(identity) => {
                    return self.create_token(&identity);
                },
                Err(_) => continue,
            }
        }
        
        Err(SecurityError::AuthenticationFailed)
    }
    
    pub fn validate_token(&self, token: &str) -> Result<Claims, SecurityError> {
        let decoding_key = DecodingKey::from_secret(&self.jwt_secret);
        let validation = Validation::default();
        
        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|_| SecurityError::InvalidToken)?;
        
        Ok(token_data.claims)
    }
    
    fn create_token(&self, identity: &Identity) -> Result<AuthToken, SecurityError> {
        let now = chrono::Utc::now();
        let expiration = now + chrono::Duration::from_std(self.token_expiration)
            .map_err(|_| SecurityError::TokenCreationFailed)?;
        
        let claims = Claims {
            sub: identity.user_id.clone(),
            roles: identity.roles.clone(),
            permissions: identity.permissions.clone(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            iss: "etlrs".to_string(),
        };
        
        let encoding_key = EncodingKey::from_secret(&self.jwt_secret);
        let token = encode(&Header::default(), &claims, &encoding_key)
            .map_err(|_| SecurityError::TokenCreationFailed)?;
        
        Ok(AuthToken {
            token,
            expires_at: expiration,
            token_type: "Bearer".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub token_type: String,
}
```

### 2. Autorização

```rust
// src/security/authorization.rs
pub struct AuthorizationManager {
    policies: Vec<SecurityPolicy>,
    role_hierarchy: RoleHierarchy,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub rules: Vec<AccessRule>,
    pub effect: Effect,
}

#[derive(Debug, Clone)]
pub struct AccessRule {
    pub resource: ResourcePattern,
    pub actions: Vec<Action>,
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone)]
pub enum Effect {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub enum Action {
    Read,
    Write,
    Execute,
    Delete,
    Admin,
}

#[derive(Debug, Clone)]
pub enum ResourcePattern {
    Exact(String),
    Wildcard(String),
    Regex(regex::Regex),
}

impl AuthorizationManager {
    pub fn authorize(&self, identity: &Identity, resource: &str, action: &Action) -> bool {
        let request = AuthorizationRequest {
            user_id: &identity.user_id,
            roles: &identity.roles,
            resource,
            action,
            context: &identity.metadata,
        };
        
        // Avaliar políticas em ordem de prioridade
        for policy in &self.policies {
            if self.evaluate_policy(policy, &request) {
                match policy.effect {
                    Effect::Allow => return true,
                    Effect::Deny => return false,
                }
            }
        }
        
        // Default deny
        false
    }
    
    fn evaluate_policy(&self, policy: &SecurityPolicy, request: &AuthorizationRequest) -> bool {
        for rule in &policy.rules {
            if self.matches_resource(&rule.resource, request.resource) &&
               rule.actions.contains(request.action) &&
               self.evaluate_conditions(&rule.conditions, request) {
                return true;
            }
        }
        false
    }
    
    fn matches_resource(&self, pattern: &ResourcePattern, resource: &str) -> bool {
        match pattern {
            ResourcePattern::Exact(exact) => exact == resource,
            ResourcePattern::Wildcard(wildcard) => {
                glob::Pattern::new(wildcard)
                    .map(|p| p.matches(resource))
                    .unwrap_or(false)
            },
            ResourcePattern::Regex(regex) => regex.is_match(resource),
        }
    }
    
    fn evaluate_conditions(&self, conditions: &[Condition], request: &AuthorizationRequest) -> bool {
        conditions.iter().all(|condition| {
            self.evaluate_condition(condition, request)
        })
    }
}

struct AuthorizationRequest<'a> {
    user_id: &'a str,
    roles: &'a [String],
    resource: &'a str,
    action: &'a Action,
    context: &'a HashMap<String, String>,
}
```

### 3. Auditoria

```rust
// src/security/audit.rs
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub details: HashMap<String, serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum AuditOutcome {
    Success,
    Failure(String),
    Partial(String),
}

pub struct AuditLogger {
    appenders: Vec<Box<dyn AuditAppender>>,
    filters: Vec<Box<dyn AuditFilter>>,
}

pub trait AuditAppender: Send + Sync {
    async fn log(&self, event: &AuditEvent) -> Result<(), SecurityError>;
}

pub trait AuditFilter: Send + Sync {
    fn should_log(&self, event: &AuditEvent) -> bool;
}

impl AuditLogger {
    pub async fn log_event(&self, event: AuditEvent) {
        // Aplicar filtros
        for filter in &self.filters {
            if !filter.should_log(&event) {
                return;
            }
        }
        
        // Log para todos os appenders
        for appender in &self.appenders {
            if let Err(e) = appender.log(&event).await {
                eprintln!("Failed to write audit log: {}", e);
            }
        }
    }
    
    pub async fn log_authentication(&self, user_id: &str, success: bool, details: HashMap<String, serde_json::Value>) {
        let event = AuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            user_id: Some(user_id.to_string()),
            session_id: None,
            action: "authentication".to_string(),
            resource: "auth_system".to_string(),
            outcome: if success { 
                AuditOutcome::Success 
            } else { 
                AuditOutcome::Failure("Authentication failed".to_string()) 
            },
            details,
            ip_address: None,
            user_agent: None,
        };
        
        self.log_event(event).await;
    }
    
    pub async fn log_data_access(&self, user_id: &str, resource: &str, action: &str, outcome: AuditOutcome) {
        let event = AuditEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            user_id: Some(user_id.to_string()),
            session_id: None,
            action: action.to_string(),
            resource: resource.to_string(),
            outcome,
            details: HashMap::new(),
            ip_address: None,
            user_agent: None,
        };
        
        self.log_event(event).await;
    }
}
```

## Compliance e Governança

### 1. GDPR (General Data Protection Regulation)

```rust
// src/security/gdpr.rs
pub struct GdprCompliance {
    data_processor_registry: DataProcessorRegistry,
    consent_manager: ConsentManager,
    anonymization_engine: AnonymizationEngine,
}

#[derive(Debug, Clone)]
pub struct PersonalDataField {
    pub field_name: String,
    pub data_category: PersonalDataCategory,
    pub processing_purpose: Vec<ProcessingPurpose>,
    pub retention_period: Option<chrono::Duration>,
    pub anonymization_method: Option<AnonymizationMethod>,
}

#[derive(Debug, Clone)]
pub enum PersonalDataCategory {
    BasicPersonalData,
    SensitivePersonalData,
    BiometricData,
    HealthData,
    FinancialData,
    LocationData,
}

#[derive(Debug, Clone)]
pub enum ProcessingPurpose {
    ServiceProvision,
    Analytics,
    Marketing,
    Compliance,
    SecurityMonitoring,
}

#[derive(Debug, Clone)]
pub enum AnonymizationMethod {
    Pseudonymization,
    Generalization,
    Suppression,
    NoiseAddition,
    Aggregation,
}

impl GdprCompliance {
    pub fn process_personal_data(&self, record: &mut Record, purpose: ProcessingPurpose) -> Result<(), SecurityError> {
        // Verificar se há consentimento
        if let Some(user_id) = record.metadata.get("user_id") {
            if !self.consent_manager.has_consent(user_id, &purpose)? {
                return Err(SecurityError::InsufficientConsent);
            }
        }
        
        // Aplicar anonimização se necessário
        for field in &self.data_processor_registry.get_personal_data_fields() {
            if record.fields.contains_key(&field.field_name) {
                if let Some(method) = &field.anonymization_method {
                    self.anonymization_engine.anonymize_field(record, &field.field_name, method)?;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn handle_data_subject_request(&self, request: DataSubjectRequest) -> Result<DataSubjectResponse, SecurityError> {
        match request.request_type {
            DataSubjectRequestType::Access => {
                self.extract_personal_data(&request.subject_id)
            },
            DataSubjectRequestType::Deletion => {
                self.delete_personal_data(&request.subject_id)
            },
            DataSubjectRequestType::Rectification => {
                self.rectify_personal_data(&request.subject_id, &request.corrections)
            },
            DataSubjectRequestType::Portability => {
                self.export_personal_data(&request.subject_id)
            },
        }
    }
}
```

### 2. SOX (Sarbanes-Oxley Act)

```rust
// src/security/sox.rs
pub struct SoxCompliance {
    change_management: ChangeManagementSystem,
    audit_trail: ComprehensiveAuditTrail,
    segregation_duties: DutiesSegregation,
}

impl SoxCompliance {
    pub fn validate_change(&self, change: &ChangeRequest) -> Result<(), ComplianceError> {
        // Verificar aprovações necessárias
        self.change_management.validate_approvals(change)?;
        
        // Verificar segregação de duties
        self.segregation_duties.validate_change_author(change)?;
        
        // Registrar mudança para auditoria
        self.audit_trail.record_change(change)?;
        
        Ok(())
    }
    
    pub fn generate_sox_report(&self, period: DateRange) -> Result<SoxReport, ComplianceError> {
        let changes = self.audit_trail.get_changes_in_period(&period)?;
        let access_reviews = self.audit_trail.get_access_reviews(&period)?;
        let control_tests = self.audit_trail.get_control_tests(&period)?;
        
        Ok(SoxReport {
            period,
            changes,
            access_reviews,
            control_tests,
            compliance_status: self.assess_compliance(&period)?,
        })
    }
}
```

## Configuração de Segurança

### 1. Configuração Centralizada

```rust
// src/security/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption: EncryptionConfig,
    pub authentication: AuthenticationConfig,
    pub authorization: AuthorizationConfig,
    pub audit: AuditConfig,
    pub compliance: ComplianceConfig,
    pub network: NetworkSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enable_at_rest: bool,
    pub enable_in_transit: bool,
    pub key_rotation_interval_days: u64,
    pub algorithm: String,
    pub key_store_type: String,
    pub key_store_config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub providers: Vec<AuthProviderConfig>,
    pub token_expiration_minutes: u64,
    pub require_2fa: bool,
    pub password_policy: PasswordPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    pub allowed_ips: Vec<String>,
    pub enable_rate_limiting: bool,
    pub rate_limit_requests_per_minute: u64,
    pub enable_tls: bool,
    pub tls_min_version: String,
    pub allowed_ciphers: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            encryption: EncryptionConfig {
                enable_at_rest: true,
                enable_in_transit: true,
                key_rotation_interval_days: 90,
                algorithm: "AES-256-GCM".to_string(),
                key_store_type: "kms".to_string(),
                key_store_config: HashMap::new(),
            },
            authentication: AuthenticationConfig {
                providers: vec![],
                token_expiration_minutes: 60,
                require_2fa: false,
                password_policy: PasswordPolicy::default(),
            },
            authorization: AuthorizationConfig {
                default_deny: true,
                policy_files: vec!["policies/default.json".to_string()],
                enable_rbac: true,
                enable_abac: false,
            },
            audit: AuditConfig {
                enable_audit_logging: true,
                audit_level: AuditLevel::Standard,
                retention_days: 365,
                appenders: vec!["file".to_string(), "syslog".to_string()],
            },
            compliance: ComplianceConfig {
                enabled_frameworks: vec![],
                data_classification_required: true,
                retention_policies: HashMap::new(),
            },
            network: NetworkSecurityConfig {
                allowed_ips: vec!["0.0.0.0/0".to_string()],
                enable_rate_limiting: true,
                rate_limit_requests_per_minute: 1000,
                enable_tls: true,
                tls_min_version: "1.2".to_string(),
                allowed_ciphers: vec![
                    "TLS_AES_256_GCM_SHA384".to_string(),
                    "TLS_AES_128_GCM_SHA256".to_string(),
                ],
            },
        }
    }
}
```

### 2. Validação de Configuração

```rust
impl SecurityConfig {
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        // Validar configuração de criptografia
        self.encryption.validate()?;
        
        // Validar configuração de autenticação
        self.authentication.validate()?;
        
        // Validar configuração de rede
        self.network.validate()?;
        
        Ok(())
    }
}

impl EncryptionConfig {
    fn validate(&self) -> Result<(), ConfigurationError> {
        if self.key_rotation_interval_days == 0 {
            return Err(ConfigurationError::invalid_value(
                "key_rotation_interval_days", "must be greater than 0"
            ));
        }
        
        match self.algorithm.as_str() {
            "AES-256-GCM" | "AES-128-GCM" | "ChaCha20-Poly1305" => {},
            _ => return Err(ConfigurationError::invalid_value(
                "algorithm", "unsupported encryption algorithm"
            )),
        }
        
        Ok(())
    }
}
```

## Testing de Segurança

### 1. Testes de Criptografia

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_roundtrip() {
        let key = generate_test_key();
        let manager = EncryptionManager::new(&key).unwrap();
        
        let original_data = b"sensitive data";
        let encrypted = manager.encrypt(original_data).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        
        assert_eq!(original_data, &decrypted[..]);
    }
    
    #[test]
    fn test_field_encryption() {
        let encryption = FieldEncryption::new(
            EncryptionManager::new(&generate_test_key()).unwrap(),
            vec!["ssn".to_string(), "credit_card".to_string()].into_iter().collect(),
        );
        
        let mut record = Record {
            fields: HashMap::from([
                ("name".to_string(), Value::String("John Doe".to_string())),
                ("ssn".to_string(), Value::String("123-45-6789".to_string())),
            ]),
            metadata: RecordMetadata::default(),
        };
        
        let encrypted_record = encryption.encrypt_record(record.clone()).unwrap();
        assert_ne!(record.fields["ssn"], encrypted_record.fields["ssn"]);
        
        let decrypted_record = encryption.decrypt_record(encrypted_record).unwrap();
        assert_eq!(record.fields["ssn"], decrypted_record.fields["ssn"]);
    }
}
```

### 2. Testes de Controle de Acesso

```rust
#[tokio::test]
async fn test_authentication_flow() {
    let auth_manager = create_test_auth_manager();
    
    // Teste de autenticação válida
    let credentials = Credentials::UsernamePassword {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };
    
    let token = auth_manager.authenticate(&credentials).await.unwrap();
    assert!(!token.token.is_empty());
    
    // Teste de validação de token
    let claims = auth_manager.validate_token(&token.token).unwrap();
    assert_eq!(claims.sub, "testuser");
}

#[test]
fn test_authorization_policies() {
    let auth_manager = create_test_authorization_manager();
    
    let admin_identity = Identity {
        user_id: "admin".to_string(),
        roles: vec!["admin".to_string()],
        permissions: vec!["*".to_string()],
        metadata: HashMap::new(),
    };
    
    let user_identity = Identity {
        user_id: "user".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec!["read".to_string()],
        metadata: HashMap::new(),
    };
    
    // Admin pode fazer tudo
    assert!(auth_manager.authorize(&admin_identity, "/admin/users", &Action::Read));
    assert!(auth_manager.authorize(&admin_identity, "/admin/users", &Action::Write));
    
    // Usuário comum só pode ler dados próprios
    assert!(auth_manager.authorize(&user_identity, "/user/profile", &Action::Read));
    assert!(!auth_manager.authorize(&user_identity, "/admin/users", &Action::Read));
}
```

## Melhores Práticas

### 1. Desenvolvimento Seguro
- **Secure coding practices**: Seguir práticas seguras de desenvolvimento
- **Code review**: Revisão de código focada em segurança
- **Static analysis**: Usar ferramentas de análise estática
- **Dependency scanning**: Verificar vulnerabilidades em dependências

### 2. Deployment Seguro
- **Container security**: Usar imagens base seguras e atualizadas
- **Network isolation**: Isolar componentes na rede
- **Secrets management**: Nunca commitar secrets no código
- **Infrastructure as Code**: Usar IaC para configurações consistentes

### 3. Monitoramento de Segurança
- **Security metrics**: Monitorar métricas de segurança
- **Threat detection**: Detectar ameaças em tempo real
- **Incident response**: Ter planos de resposta a incidentes
- **Regular audits**: Realizar auditorias regulares de segurança

### 4. Compliance Contínua
- **Automated compliance checks**: Automatizar verificações de compliance
- **Documentation**: Manter documentação atualizada
- **Training**: Treinar equipe em práticas de segurança
- **Regular reviews**: Revisar e atualizar políticas regularmente

Este guia estabelece uma base sólida para implementar segurança robusta em pipelines ETL, garantindo proteção adequada para dados sensíveis e conformidade com regulamentações aplicáveis.
