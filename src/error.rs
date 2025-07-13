use thiserror::Error;

/// Tipo Result principal da biblioteca
pub type Result<T> = std::result::Result<T, ETLError>;

/// Erro principal da biblioteca ETLRS
#[derive(Error, Debug)]
pub enum ETLError {
    #[error("Erro de extração: {0}")]
    Extract(#[from] ExtractError),
    
    #[error("Erro de transformação: {0}")]
    Transform(#[from] TransformError),
    
    #[error("Erro de carga: {0}")]
    Load(#[from] LoadError),
    
    #[error("Erro de configuração: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Erro de pipeline: {0}")]
    Pipeline(String),
    
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Erro de serialização: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Erro genérico: {0}")]
    Generic(#[from] anyhow::Error),
}

/// Erros relacionados à extração de dados
#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("Erro de conexão: {0}")]
    Connection(String),
    
    #[error("Arquivo não encontrado: {0}")]
    FileNotFound(String),
    
    #[error("Formato inválido: {0}")]
    InvalidFormat(String),
    
    #[error("Erro de parsing: {0}")]
    ParseError(String),
    
    #[error("Timeout na extração: {0}")]
    Timeout(String),
    
    #[error("Dados corrompidos: {0}")]
    CorruptedData(String),
}

/// Erros relacionados à transformação de dados
#[derive(Error, Debug)]
pub enum TransformError {
    #[error("Transformação inválida: {0}")]
    InvalidTransformation(String),
    
    #[error("Tipo de dado incompatível: {0}")]
    IncompatibleType(String),
    
    #[error("Validação falhou: {0}")]
    ValidationFailed(String),
    
    #[error("Erro de processamento: {0}")]
    ProcessingError(String),
    
    #[error("Limite de memória excedido: {0}")]
    MemoryLimitExceeded(String),
}

/// Erros relacionados ao carregamento de dados
#[derive(Error, Debug)]
pub enum LoadError {
    #[error("Erro de conexão de destino: {0}")]
    DestinationConnection(String),
    
    #[error("Erro de escrita: {0}")]
    WriteError(String),
    
    #[error("Capacidade excedida: {0}")]
    CapacityExceeded(String),
    
    #[error("Conflito de dados: {0}")]
    DataConflict(String),
    
    #[error("Permissão negada: {0}")]
    PermissionDenied(String),
}

/// Erros relacionados à configuração
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuração inválida: {0}")]
    InvalidConfig(String),
    
    #[error("Parâmetro obrigatório ausente: {0}")]
    MissingRequiredParameter(String),
    
    #[error("Valor inválido para {param}: {value}")]
    InvalidValue { param: String, value: String },
    
    #[error("Erro de parsing de configuração: {0}")]
    ParseError(String),
}

impl ETLError {
    /// Verifica se o erro é recuperável
    pub fn is_recoverable(&self) -> bool {
        match self {
            ETLError::Extract(ExtractError::Timeout(_)) => true,
            ETLError::Extract(ExtractError::Connection(_)) => true,
            ETLError::Load(LoadError::DestinationConnection(_)) => true,
            ETLError::Load(LoadError::CapacityExceeded(_)) => true,
            _ => false,
        }
    }
    
    /// Retorna o código de erro
    pub fn error_code(&self) -> &'static str {
        match self {
            ETLError::Extract(_) => "EXTRACT_ERROR",
            ETLError::Transform(_) => "TRANSFORM_ERROR",
            ETLError::Load(_) => "LOAD_ERROR",
            ETLError::Config(_) => "CONFIG_ERROR",
            ETLError::Pipeline(_) => "PIPELINE_ERROR",
            ETLError::Io(_) => "IO_ERROR",
            ETLError::Serialization(_) => "SERIALIZATION_ERROR",
            ETLError::Generic(_) => "GENERIC_ERROR",
        }
    }
}

impl From<config::ConfigError> for ETLError {
    fn from(err: config::ConfigError) -> Self {
        ETLError::Config(ConfigError::ParseError(err.to_string()))
    }
}

#[cfg(feature = "database")]
impl From<sqlx::Error> for ETLError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                ETLError::Extract(ExtractError::Connection(db_err.to_string()))
            }
            sqlx::Error::Io(io_err) => ETLError::Io(io_err),
            _ => ETLError::Generic(anyhow::anyhow!(err)),
        }
    }
}

#[cfg(feature = "csv")]
impl From<csv::Error> for ETLError {
    fn from(err: csv::Error) -> Self {
        match err.kind() {
            csv::ErrorKind::Io(io_err) => ETLError::Io(std::io::Error::new(io_err.kind(), io_err.to_string())),
            csv::ErrorKind::Utf8 { .. } => {
                ETLError::Extract(ExtractError::InvalidFormat("UTF-8 inválido".to_string()))
            }
            _ => ETLError::Extract(ExtractError::ParseError(err.to_string())),
        }
    }
}
