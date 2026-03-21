use std::sync::Arc;

use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

/// Shared metrics registry with typed counters and gauges for Prometheus export.
#[derive(Clone)]
pub struct NodeMetrics {
    inner: Arc<Inner>,
}

struct Inner {
    registry: Registry,

    // Consensus counters
    pub vertices_proposed: Counter,
    pub vertices_received: Counter,
    pub commits: Counter,
    pub rounds_advanced: Counter,
    pub equivocations: Counter,

    // Consensus gauges
    pub last_commit_latency_us: Gauge,

    // Execution counters
    pub txs_processed: Counter,

    // Execution gauges
    pub block_height: Gauge,
    pub base_fee: Gauge,
    pub mempool_size: Gauge,
    pub pending_tasks: Gauge,
    pub peer_count: Gauge,

    // Staking gauges
    pub active_validators: Gauge,
    pub total_staked: Gauge,
    pub slashes_applied: Counter,

    // Sentinel / AI gauges
    pub sentinel_health_score: Gauge,
    pub sentinel_health_level: Gauge,
    pub sentinel_actions_total: Counter,
    pub anomalous_txs_total: Counter,
    pub epoch_number: Gauge,
}

impl NodeMetrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let vertices_proposed = Counter::default();
        let vertices_received = Counter::default();
        let commits = Counter::default();
        let rounds_advanced = Counter::default();
        let equivocations = Counter::default();
        let last_commit_latency_us = Gauge::<i64, _>::default();
        let txs_processed = Counter::default();
        let block_height = Gauge::<i64, _>::default();
        let base_fee = Gauge::<i64, _>::default();
        let mempool_size = Gauge::<i64, _>::default();
        let pending_tasks = Gauge::<i64, _>::default();
        let peer_count = Gauge::<i64, _>::default();
        let active_validators = Gauge::<i64, _>::default();
        let total_staked = Gauge::<i64, _>::default();
        let slashes_applied = Counter::default();
        let sentinel_health_score = Gauge::<i64, _>::default();
        let sentinel_health_level = Gauge::<i64, _>::default();
        let sentinel_actions_total = Counter::default();
        let anomalous_txs_total = Counter::default();
        let epoch_number = Gauge::<i64, _>::default();

        registry.register(
            "aztibase_consensus_vertices_proposed",
            "Total DAG vertices proposed by this validator",
            vertices_proposed.clone(),
        );
        registry.register(
            "aztibase_consensus_vertices_received",
            "Total DAG vertices received from peers",
            vertices_received.clone(),
        );
        registry.register(
            "aztibase_consensus_commits",
            "Total committed batches",
            commits.clone(),
        );
        registry.register(
            "aztibase_consensus_rounds_advanced",
            "Total consensus rounds advanced",
            rounds_advanced.clone(),
        );
        registry.register(
            "aztibase_consensus_equivocations",
            "Equivocation events detected",
            equivocations.clone(),
        );
        registry.register(
            "aztibase_consensus_last_commit_latency_us",
            "Latency of last committed batch in microseconds",
            last_commit_latency_us.clone(),
        );
        registry.register(
            "aztibase_execution_txs_processed",
            "Total transactions processed",
            txs_processed.clone(),
        );
        registry.register(
            "aztibase_execution_block_height",
            "Current block height (batch count)",
            block_height.clone(),
        );
        registry.register(
            "aztibase_execution_base_fee",
            "Current base fee",
            base_fee.clone(),
        );
        registry.register(
            "aztibase_network_mempool_size",
            "Current mempool transaction count",
            mempool_size.clone(),
        );
        registry.register(
            "aztibase_ai_pending_tasks",
            "Pending AI inference tasks",
            pending_tasks.clone(),
        );
        registry.register(
            "aztibase_network_peer_count",
            "Connected P2P peers",
            peer_count.clone(),
        );
        registry.register(
            "aztibase_staking_active_validators",
            "Number of active validators in the current set",
            active_validators.clone(),
        );
        registry.register(
            "aztibase_staking_total_staked",
            "Total amount staked across all validators",
            total_staked.clone(),
        );
        registry.register(
            "aztibase_staking_slashes_applied",
            "Total slashing events applied",
            slashes_applied.clone(),
        );
        registry.register(
            "aztibase_sentinel_health_score",
            "Latest sentinel health score (0-1000, divide by 1000 for float)",
            sentinel_health_score.clone(),
        );
        registry.register(
            "aztibase_sentinel_health_level",
            "Sentinel health level (0=normal, 1=warning, 2=critical)",
            sentinel_health_level.clone(),
        );
        registry.register(
            "aztibase_sentinel_actions_total",
            "Total sentinel actions triggered",
            sentinel_actions_total.clone(),
        );
        registry.register(
            "aztibase_execution_anomalous_txs_total",
            "Total transactions flagged as anomalous",
            anomalous_txs_total.clone(),
        );
        registry.register(
            "aztibase_consensus_epoch_number",
            "Current consensus epoch number",
            epoch_number.clone(),
        );

        Self {
            inner: Arc::new(Inner {
                registry,
                vertices_proposed,
                vertices_received,
                commits,
                rounds_advanced,
                equivocations,
                last_commit_latency_us,
                txs_processed,
                block_height,
                base_fee,
                mempool_size,
                pending_tasks,
                peer_count,
                active_validators,
                total_staked,
                slashes_applied,
                sentinel_health_score,
                sentinel_health_level,
                sentinel_actions_total,
                anomalous_txs_total,
                epoch_number,
            }),
        }
    }

    /// Update consensus counters from a snapshot delta.
    /// Pass the absolute values from `ConsensusMetrics::snapshot()`.
    /// Counters are monotonic, so we set them to the absolute value by computing the increment.
    pub fn update_consensus(
        &self,
        vertices_proposed: u64,
        vertices_received: u64,
        commits_total: u64,
        rounds_advanced: u64,
        equivocations: u64,
        last_commit_latency_us: u64,
    ) {
        set_counter(&self.inner.vertices_proposed, vertices_proposed);
        set_counter(&self.inner.vertices_received, vertices_received);
        set_counter(&self.inner.commits, commits_total);
        set_counter(&self.inner.rounds_advanced, rounds_advanced);
        set_counter(&self.inner.equivocations, equivocations);
        self.inner
            .last_commit_latency_us
            .set(last_commit_latency_us as i64);
    }

    pub fn update_execution(&self, block_height: u64, base_fee: u64) {
        self.inner.block_height.set(block_height as i64);
        self.inner.base_fee.set(base_fee as i64);
    }

    pub fn inc_txs_processed(&self, count: u64) {
        self.inner.txs_processed.inc_by(count);
    }

    pub fn set_mempool_size(&self, size: u64) {
        self.inner.mempool_size.set(size as i64);
    }

    pub fn set_pending_tasks(&self, count: u64) {
        self.inner.pending_tasks.set(count as i64);
    }

    pub fn set_peer_count(&self, count: u64) {
        self.inner.peer_count.set(count as i64);
    }

    pub fn update_staking(&self, active_validators: u64, total_staked: u128) {
        self.inner.active_validators.set(active_validators as i64);
        self.inner
            .total_staked
            .set(total_staked.min(i64::MAX as u128) as i64);
    }

    pub fn inc_slashes(&self, count: u64) {
        self.inner.slashes_applied.inc_by(count);
    }

    pub fn set_sentinel_health(&self, score_millionths: i64, level: i64) {
        self.inner.sentinel_health_score.set(score_millionths);
        self.inner.sentinel_health_level.set(level);
    }

    pub fn inc_sentinel_actions(&self) {
        self.inner.sentinel_actions_total.inc();
    }

    pub fn inc_anomalous_txs(&self, count: u64) {
        self.inner.anomalous_txs_total.inc_by(count);
    }

    pub fn set_epoch(&self, epoch: u64) {
        self.inner.epoch_number.set(epoch as i64);
    }

    /// Encode all metrics in Prometheus text exposition format.
    pub fn encode_prometheus(&self) -> String {
        let mut buf = String::new();
        encode(&mut buf, &self.inner.registry).expect("prometheus encoding should not fail");
        buf
    }

    /// Encode metrics as a JSON object (backward compatibility with ADR-008).
    pub fn encode_json(&self) -> serde_json::Value {
        serde_json::json!({
            "consensus": {
                "vertices_proposed": counter_value(&self.inner.vertices_proposed),
                "vertices_received": counter_value(&self.inner.vertices_received),
                "commits": counter_value(&self.inner.commits),
                "rounds_advanced": counter_value(&self.inner.rounds_advanced),
                "equivocations": counter_value(&self.inner.equivocations),
                "last_commit_latency_us": self.inner.last_commit_latency_us.get(),
            },
            "execution": {
                "block_height": self.inner.block_height.get(),
                "base_fee": self.inner.base_fee.get(),
                "txs_processed": counter_value(&self.inner.txs_processed),
            },
            "network": {
                "mempool_size": self.inner.mempool_size.get(),
                "peer_count": self.inner.peer_count.get(),
            },
            "ai": {
                "pending_tasks": self.inner.pending_tasks.get(),
                "anomalous_txs_total": counter_value(&self.inner.anomalous_txs_total),
            },
            "sentinel": {
                "health_score": self.inner.sentinel_health_score.get(),
                "health_level": self.inner.sentinel_health_level.get(),
                "actions_total": counter_value(&self.inner.sentinel_actions_total),
            },
            "staking": {
                "active_validators": self.inner.active_validators.get(),
                "total_staked": self.inner.total_staked.get(),
                "slashes_applied": counter_value(&self.inner.slashes_applied),
                "epoch_number": self.inner.epoch_number.get(),
            }
        })
    }
}

impl Default for NodeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Set a monotonic counter to an absolute value by incrementing the delta.
fn set_counter(counter: &Counter, absolute: u64) {
    let current = counter.get();
    if absolute > current {
        counter.inc_by(absolute - current);
    }
}

fn counter_value(counter: &Counter) -> u64 {
    counter.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_encoding_contains_metrics() {
        let m = NodeMetrics::new();
        m.update_consensus(10, 20, 5, 3, 0, 1500);
        m.update_execution(42, 100);
        m.set_peer_count(7);

        let text = m.encode_prometheus();
        assert!(text.contains("aztibase_consensus_vertices_proposed"));
        assert!(text.contains("aztibase_consensus_commits"));
        assert!(text.contains("aztibase_execution_block_height"));
        assert!(text.contains("aztibase_network_peer_count"));
    }

    #[test]
    fn json_encoding_matches_values() {
        let m = NodeMetrics::new();
        m.update_consensus(10, 20, 5, 3, 1, 2000);
        m.update_execution(99, 50);
        m.set_mempool_size(42);
        m.set_pending_tasks(3);
        m.set_peer_count(8);

        let json = m.encode_json();
        assert_eq!(json["consensus"]["vertices_proposed"], 10);
        assert_eq!(json["consensus"]["equivocations"], 1);
        assert_eq!(json["execution"]["block_height"], 99);
        assert_eq!(json["execution"]["base_fee"], 50);
        assert_eq!(json["network"]["mempool_size"], 42);
        assert_eq!(json["network"]["peer_count"], 8);
        assert_eq!(json["ai"]["pending_tasks"], 3);
    }

    #[test]
    fn set_counter_increments_correctly() {
        let m = NodeMetrics::new();
        m.update_consensus(5, 10, 2, 1, 0, 100);
        m.update_consensus(8, 15, 4, 3, 0, 200);

        let json = m.encode_json();
        assert_eq!(json["consensus"]["vertices_proposed"], 8);
        assert_eq!(json["consensus"]["commits"], 4);
    }

    #[test]
    fn staking_metrics_tracked() {
        let m = NodeMetrics::new();
        m.update_staking(5, 1_000_000);
        m.inc_slashes(2);

        let json = m.encode_json();
        assert_eq!(json["staking"]["active_validators"], 5);
        assert_eq!(json["staking"]["total_staked"], 1_000_000);
        assert_eq!(json["staking"]["slashes_applied"], 2);

        let text = m.encode_prometheus();
        assert!(text.contains("aztibase_staking_active_validators"));
        assert!(text.contains("aztibase_staking_total_staked"));
    }

    #[test]
    fn inc_txs_processed_accumulates() {
        let m = NodeMetrics::new();
        m.inc_txs_processed(10);
        m.inc_txs_processed(5);
        let json = m.encode_json();
        assert_eq!(json["execution"]["txs_processed"], 15);
    }
}
