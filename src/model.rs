//! Compact observations. Raw DSNs and credentials never appear here.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Display name plus canonical root.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepoId {
    /// Markdown identity.
    pub name: String,
    /// Canonical repository root.
    pub root: PathBuf,
}

/// Everything one repository proved about its external boundaries.
#[derive(Debug, Clone, Default)]
pub struct RepoInventory {
    /// Repository identity.
    pub repo: RepoId,
    /// Service/package aliases used for host matching.
    pub aliases: BTreeSet<String>,
    /// HTTP/gRPC/GraphQL observations.
    pub api: Vec<ApiObservation>,
    /// Kafka producer/consumer observations.
    pub kafka: Vec<KafkaObservation>,
    /// Sanitized database identities.
    pub databases: Vec<DatabaseObservation>,
}

/// Whether a repository exposes or calls an API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApiDirection {
    /// Server/provider.
    Exposes,
    /// Client/caller.
    Calls,
}

/// API protocol family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApiProtocol {
    /// HTTP/HTTPS.
    Http,
    /// gRPC / Protobuf service.
    Grpc,
    /// GraphQL operation.
    Graphql,
}

/// One API observation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiObservation {
    /// Provider or consumer.
    pub direction: ApiDirection,
    /// Protocol.
    pub protocol: ApiProtocol,
    /// HTTP method when known, uppercased.
    pub method: Option<String>,
    /// Route, gRPC service, or GraphQL field.
    pub resource: String,
    /// Hostname or service alias when statically present.
    pub host_hint: Option<String>,
}

/// Kafka producer or consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KafkaRole {
    /// Publishes to a topic.
    Producer,
    /// Reads from a topic.
    Consumer,
}

/// One Kafka observation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KafkaObservation {
    /// Producer or consumer.
    pub role: KafkaRole,
    /// Topic name.
    pub topic: String,
    /// Optional cluster/broker identity, never a secret.
    pub cluster: Option<String>,
}

/// Database engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatabaseEngine {
    /// PostgreSQL.
    Postgres,
    /// MongoDB.
    Mongo,
    /// MySQL or MariaDB.
    Mysql,
    /// ClickHouse.
    Clickhouse,
    /// Microsoft SQL Server.
    SqlServer,
    /// SQLite (never a cross-repo edge by default).
    Sqlite,
}

impl DatabaseEngine {
    /// Human label used in Markdown.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mongo => "MongoDB",
            Self::Mysql => "MySQL",
            Self::Clickhouse => "ClickHouse",
            Self::SqlServer => "SQL Server",
            Self::Sqlite => "SQLite",
        }
    }
}

/// Sanitized database identity. Host may be a service name, never a DSN.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatabaseObservation {
    /// Engine.
    pub engine: DatabaseEngine,
    /// Hostname or deployment service name.
    pub host: Option<String>,
    /// Logical database name.
    pub database: Option<String>,
    /// True when host is loopback and needs shared deployment evidence.
    pub localhost: bool,
}

/// Proven database neighborhood for one resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRelation {
    /// Engine label.
    pub engine: String,
    /// Logical database name.
    pub database: String,
    /// Other repositories sharing the identity.
    pub peers: Vec<String>,
}

/// Proven Kafka neighborhood for one topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaRelation {
    /// Topic.
    pub topic: String,
    /// Peers that consume what this repository produces.
    pub produces: Vec<String>,
    /// Peers that produce what this repository consumes.
    pub consumes: Vec<String>,
}

/// Proven API neighborhood, collapsed to peer names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiRelation {
    /// Repositories this one calls.
    pub outgoing: Vec<String>,
    /// Repositories that call this one.
    pub incoming: Vec<String>,
}

/// Render-ready map for one repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMap {
    /// Display name.
    pub repository: String,
    /// Database sections.
    pub databases: Vec<DatabaseRelation>,
    /// Kafka sections.
    pub kafka: Vec<KafkaRelation>,
    /// API section.
    pub api: ApiRelation,
}

impl RepositoryMap {
    /// Number of proven peer lines across all sections.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        let databases = self
            .databases
            .iter()
            .map(|item| item.peers.len())
            .sum::<usize>();
        let kafka = self
            .kafka
            .iter()
            .map(|item| item.produces.len() + item.consumes.len())
            .sum::<usize>();
        databases + kafka + self.api.outgoing.len() + self.api.incoming.len()
    }
}
