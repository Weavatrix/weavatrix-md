//! Inverted indexes. Missing edges are omitted; ambiguous edges are omitted.

use crate::model::{
    ApiDirection, ApiProtocol, ApiRelation, DatabaseEngine, DatabaseRelation, KafkaRelation,
    KafkaRole, RepoInventory, RepositoryMap,
};
use crate::normalize;
use std::collections::{BTreeMap, BTreeSet};

type RepoName = String;

#[derive(Default)]
struct TopicPeers {
    producers: BTreeSet<RepoName>,
    consumers: BTreeSet<RepoName>,
    clusters: BTreeSet<String>,
    missing_cluster: bool,
}

impl TopicPeers {
    fn merge(&mut self, other: Self) {
        self.producers.extend(other.producers);
        self.consumers.extend(other.consumers);
        self.clusters.extend(other.clusters);
        self.missing_cluster |= other.missing_cluster;
    }
}

/// Builds one map per target from compact inventories.
#[must_use]
pub fn resolve(inventories: &[RepoInventory], targets: &[RepoName]) -> Vec<RepositoryMap> {
    let alias_index = alias_index(inventories);
    let kafka = kafka_edges(inventories);
    let databases = database_edges(inventories);
    let api = api_edges(inventories, &alias_index);
    targets
        .iter()
        .map(|name| RepositoryMap {
            repository: name.clone(),
            databases: databases.get(name).cloned().unwrap_or_default(),
            kafka: kafka.get(name).cloned().unwrap_or_default(),
            api: api.get(name).cloned().unwrap_or_default(),
        })
        .collect()
}

fn alias_index(inventories: &[RepoInventory]) -> BTreeMap<String, BTreeSet<RepoName>> {
    let mut index = BTreeMap::<String, BTreeSet<RepoName>>::new();
    for inventory in inventories {
        for alias in &inventory.aliases {
            for candidate in normalize::alias_candidates(alias) {
                index
                    .entry(candidate)
                    .or_default()
                    .insert(inventory.repo.name.clone());
            }
        }
        for candidate in normalize::alias_candidates(&inventory.repo.name) {
            index
                .entry(candidate)
                .or_default()
                .insert(inventory.repo.name.clone());
        }
    }
    index
}

fn kafka_edges(inventories: &[RepoInventory]) -> BTreeMap<RepoName, Vec<KafkaRelation>> {
    let mut topics = BTreeMap::<(Option<String>, String), TopicPeers>::new();
    for inventory in inventories {
        for item in &inventory.kafka {
            let key = (item.cluster.clone(), item.topic.clone());
            let entry = topics.entry(key).or_default();
            match item.cluster {
                Some(ref cluster) => {
                    entry.clusters.insert(cluster.clone());
                }
                None => entry.missing_cluster = true,
            }
            match item.role {
                KafkaRole::Producer => {
                    entry.producers.insert(inventory.repo.name.clone());
                }
                KafkaRole::Consumer => {
                    entry.consumers.insert(inventory.repo.name.clone());
                }
            }
        }
    }
    let mut by_topic = BTreeMap::<String, TopicPeers>::new();
    for ((_cluster, topic), peers) in topics {
        by_topic.entry(topic).or_default().merge(peers);
    }
    let mut maps = BTreeMap::<RepoName, BTreeMap<String, KafkaRelation>>::new();
    for (topic, peers) in by_topic {
        if peers.producers.is_empty() || peers.consumers.is_empty() {
            continue;
        }
        if peers.clusters.len() > 1 {
            continue;
        }
        if normalize::is_generic_topic(&topic)
            && (peers.clusters.is_empty() || peers.missing_cluster)
        {
            continue;
        }
        for producer in &peers.producers {
            let consumers: Vec<String> = peers
                .consumers
                .iter()
                .filter(|consumer| *consumer != producer)
                .cloned()
                .collect();
            if consumers.is_empty() {
                continue;
            }
            let relation = maps
                .entry(producer.clone())
                .or_default()
                .entry(topic.clone())
                .or_insert_with(|| KafkaRelation {
                    topic: topic.clone(),
                    produces: Vec::new(),
                    consumes: Vec::new(),
                });
            relation.produces.extend(consumers);
            relation.produces.sort();
            relation.produces.dedup();
        }
        for consumer in &peers.consumers {
            let producers: Vec<String> = peers
                .producers
                .iter()
                .filter(|producer| *producer != consumer)
                .cloned()
                .collect();
            if producers.is_empty() {
                continue;
            }
            let relation = maps
                .entry(consumer.clone())
                .or_default()
                .entry(topic.clone())
                .or_insert_with(|| KafkaRelation {
                    topic: topic.clone(),
                    produces: Vec::new(),
                    consumes: Vec::new(),
                });
            relation.consumes.extend(producers);
            relation.consumes.sort();
            relation.consumes.dedup();
        }
    }
    maps.into_iter()
        .map(|(repo, topics)| {
            let mut relations: Vec<KafkaRelation> = topics.into_values().collect();
            relations.sort_by(|left, right| left.topic.cmp(&right.topic));
            (repo, relations)
        })
        .collect()
}

fn database_edges(inventories: &[RepoInventory]) -> BTreeMap<RepoName, Vec<DatabaseRelation>> {
    #[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
    struct Key {
        engine: DatabaseEngine,
        host: String,
        database: Option<String>,
    }
    let mut index = BTreeMap::<Key, BTreeSet<RepoName>>::new();
    for inventory in inventories {
        for item in &inventory.databases {
            if item.engine == DatabaseEngine::Sqlite {
                continue;
            }
            let Some(host) = item.host.clone() else {
                continue;
            };
            if item.localhost {
                continue;
            }
            index
                .entry(Key {
                    engine: item.engine,
                    host,
                    database: item.database.clone(),
                })
                .or_default()
                .insert(inventory.repo.name.clone());
        }
    }
    let host_counts = {
        let mut counts = BTreeMap::<(DatabaseEngine, String), usize>::new();
        for key in index.keys() {
            *counts.entry((key.engine, key.host.clone())).or_insert(0) += 1;
        }
        counts
    };
    let mut maps = BTreeMap::<RepoName, Vec<DatabaseRelation>>::new();
    for (key, repos) in index {
        let repos: Vec<String> = if key.database.is_none() {
            if host_counts.get(&(key.engine, key.host.clone())).copied() != Some(1) {
                continue;
            }
            repos.into_iter().collect()
        } else {
            repos.into_iter().collect()
        };
        if repos.len() < 2 {
            continue;
        }
        let database = key.database.clone().unwrap_or_else(|| key.host.clone());
        for repo in &repos {
            let peers: Vec<String> = repos.iter().filter(|peer| *peer != repo).cloned().collect();
            maps.entry(repo.clone())
                .or_default()
                .push(DatabaseRelation {
                    engine: key.engine.label().to_owned(),
                    database: database.clone(),
                    peers,
                });
        }
    }
    for relations in maps.values_mut() {
        relations.sort_by(|left, right| {
            left.engine
                .cmp(&right.engine)
                .then(left.database.cmp(&right.database))
        });
    }
    maps
}

fn api_edges(
    inventories: &[RepoInventory],
    alias_index: &BTreeMap<String, BTreeSet<RepoName>>,
) -> BTreeMap<RepoName, ApiRelation> {
    let mut providers: BTreeMap<(Option<String>, String), BTreeSet<RepoName>> = BTreeMap::new();
    let mut grpc_providers: BTreeMap<String, BTreeSet<RepoName>> = BTreeMap::new();
    let mut exposed_by_repo: BTreeMap<RepoName, Vec<(Option<String>, String)>> = BTreeMap::new();
    for inventory in inventories {
        for item in &inventory.api {
            if item.direction != ApiDirection::Exposes {
                continue;
            }
            if item.protocol == ApiProtocol::Grpc {
                grpc_providers
                    .entry(item.resource.clone())
                    .or_default()
                    .insert(inventory.repo.name.clone());
                continue;
            }
            if item.protocol != ApiProtocol::Http {
                continue;
            }
            let key = (item.method.clone(), item.resource.clone());
            providers
                .entry(key.clone())
                .or_default()
                .insert(inventory.repo.name.clone());
            exposed_by_repo
                .entry(inventory.repo.name.clone())
                .or_default()
                .push(key);
        }
    }
    let mut outgoing = BTreeMap::<RepoName, BTreeSet<RepoName>>::new();
    let mut incoming = BTreeMap::<RepoName, BTreeSet<RepoName>>::new();
    for inventory in inventories {
        for item in &inventory.api {
            if item.direction != ApiDirection::Calls {
                continue;
            }
            if item.protocol == ApiProtocol::Grpc {
                if let Some(providers) = grpc_providers.get(&item.resource)
                    && providers.len() == 1
                    && let Some(provider) = providers.iter().next()
                    && provider != &inventory.repo.name
                {
                    remember(&mut outgoing, &mut incoming, &inventory.repo.name, provider);
                }
                continue;
            }
            if item.protocol != ApiProtocol::Http {
                continue;
            }
            if let Some(host) = &item.host_hint
                && let Some(provider) = unique_host(alias_index, host)
                && provider != inventory.repo.name
                && route_compatible(
                    exposed_by_repo.get(&provider),
                    item.method.as_deref(),
                    &item.resource,
                )
            {
                remember(
                    &mut outgoing,
                    &mut incoming,
                    &inventory.repo.name,
                    &provider,
                );
                continue;
            }
            if item.host_hint.is_none()
                && !normalize::is_generic_route(&item.resource)
                && let Some(providers) =
                    providers.get(&(item.method.clone(), item.resource.clone()))
                && providers.len() == 1
                && let Some(provider) = providers.iter().next()
                && provider != &inventory.repo.name
            {
                remember(&mut outgoing, &mut incoming, &inventory.repo.name, provider);
            }
        }
    }
    let mut maps = BTreeMap::new();
    let names: BTreeSet<RepoName> = inventories
        .iter()
        .map(|item| item.repo.name.clone())
        .collect();
    for name in names {
        maps.insert(
            name.clone(),
            ApiRelation {
                outgoing: outgoing
                    .get(&name)
                    .map(|set| set.iter().cloned().collect())
                    .unwrap_or_default(),
                incoming: incoming
                    .get(&name)
                    .map(|set| set.iter().cloned().collect())
                    .unwrap_or_default(),
            },
        );
    }
    maps
}

fn unique_host(alias_index: &BTreeMap<String, BTreeSet<RepoName>>, host: &str) -> Option<RepoName> {
    let mut found = BTreeSet::new();
    for alias in normalize::host_aliases(host) {
        if let Some(repos) = alias_index.get(&alias) {
            found.extend(repos.iter().cloned());
        }
    }
    (found.len() == 1).then(|| found.into_iter().next().unwrap_or_default())
}

fn route_compatible(
    exposed: Option<&Vec<(Option<String>, String)>>,
    method: Option<&str>,
    route: &str,
) -> bool {
    let Some(exposed) = exposed else {
        return false;
    };
    let client = normalize::normalize_route(route);
    exposed.iter().any(|(exposed_method, exposed_route)| {
        let method_ok = match (exposed_method.as_deref(), method) {
            (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
            _ => true,
        };
        if !method_ok {
            return false;
        }
        let provider = normalize::normalize_route(exposed_route);
        if provider == client {
            return true;
        }
        if provider == "/" || client == "/" {
            return false;
        }
        client.starts_with(&format!("{provider}/")) || provider.starts_with(&format!("{client}/"))
    })
}

fn remember(
    outgoing: &mut BTreeMap<RepoName, BTreeSet<RepoName>>,
    incoming: &mut BTreeMap<RepoName, BTreeSet<RepoName>>,
    from: &str,
    to: &str,
) {
    outgoing
        .entry(from.to_owned())
        .or_default()
        .insert(to.to_owned());
    incoming
        .entry(to.to_owned())
        .or_default()
        .insert(from.to_owned());
}
