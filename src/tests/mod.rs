use crate::RXQLiteClient;
use crate::RXQLiteClientBuilder;
//use crate::typ;
use crate::NodeId;
use futures::future::join_all;

use rxqlite_lite_common::{LogId,RaftMetrics};


use rxqlite_tests_common::*;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[cfg(not(feature = "test-dependency"))]
use tokio::runtime::Runtime;

#[cfg(not(feature = "test-dependency"))]
pub mod consts;

#[cfg(not(feature = "test-dependency"))]
pub mod notifications;

#[cfg(not(feature = "test-dependency"))]
pub mod table_notifications;

use std::env::consts::EXE_SUFFIX;

pub fn get_cluster_manager(
    test_name: &str,
    instance_count: usize,
    tls_config: Option<TestTlsConfig>,
) -> anyhow::Result<TestClusterManager> {
    let executable_path = if let Ok(rxqlited_dir) = std::env::var("RXQLITED_DIR") {
        let executable_path = PathBuf::from(rxqlited_dir).join(format!("rxqlited{}", EXE_SUFFIX));
        println!("using rxqlited: {}", executable_path.display());
        executable_path
    } else {
        let cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let executable_path = cargo_path
            .join("target")
            .join("release")
            .join(format!("rxqlited{}", EXE_SUFFIX));
        println!("using rxqlited: {}", executable_path.display());
        executable_path
    };
    assert!(executable_path.is_file());
    let temp_dir = env::temp_dir();
    let working_directory = temp_dir.join(test_name);
    TestClusterManager::new(
        instance_count,
        &working_directory,
        &executable_path,
        "127.0.0.1",
        tls_config,
        None,
    )
}

pub struct TestManager {
    pub tcm: TestClusterManager,
    pub clients: HashMap<NodeId, RXQLiteClient>,
}

impl std::ops::Deref for TestManager {
    type Target = TestClusterManager;
    fn deref(&self) -> &Self::Target {
        &self.tcm
    }
}

impl std::ops::DerefMut for TestManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tcm
    }
}

impl TestManager {
    pub fn new(test_name: &str, instance_count: usize, tls_config: Option<TestTlsConfig>) -> Self {
        let tcm = get_cluster_manager(test_name, instance_count, tls_config.clone()).unwrap();
        let clients: HashMap<NodeId, RXQLiteClient> = tcm
            .instances
            .iter()
            .map(|(node_id, instance)| {
                (
                    *node_id,
                    RXQLiteClientBuilder::new(instance.node_id, instance.http_addr.clone())
                        .use_tls(tls_config.is_some())
                        .accept_invalid_certificates(
                            if let Some(tls_config) = tls_config.as_ref() {
                                tls_config.accept_invalid_certificates
                            } else {
                                false
                            },
                        )
                        .build(),
                )
            })
            .collect();
        Self { tcm, clients }
    }

    pub async fn get_metrics(&self, node_id: NodeId) -> anyhow::Result<RaftMetrics> {
        let client = self.clients.get(&node_id).unwrap();
        let metrics = client.metrics().await?;
        Ok(metrics)
    }
    pub fn node_count(&self) -> usize {
        self.tcm.instances.len()
    }

    pub async fn wait_for_cluster_established(
        &self,
        node_id: NodeId,
        reattempts: usize,
    ) -> anyhow::Result<()> {
        let mut reattempts = reattempts + 1; // wait max for cluster to establish

        loop {
            if let Ok(metrics) = self.get_metrics(node_id).await {
              if metrics.current_leader.is_some() {
                let voter_ids = metrics.membership_config.voter_ids();
                if voter_ids.count() == self.node_count() {
                  return Ok(());
                }
              }
            }
            reattempts -= 1;
            if reattempts == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(anyhow::anyhow!("wait_for_cluster_established timeout"))
    }
    #[allow(dead_code)]
    pub async fn wait_for_last_applied_log(
        &self,
        log_id: LogId,
        reattempts: usize,
    ) -> anyhow::Result<HashMap<NodeId, RaftMetrics>> {
        let mut reattempts = reattempts + 1;
        let mut node_metrics: HashMap<NodeId, RaftMetrics> = Default::default();

        loop {
            let mut futs = vec![];
            for (node_id, client) in self.clients.iter() {
                if node_metrics.contains_key(node_id) {
                    continue;
                }
                futs.push(client.node_metrics());
            }
            if futs.len() == 0 {
                return Ok(node_metrics);
            }
            let metrics = join_all(futs).await;
            for metrics in metrics {
                if let Ok(metrics) = metrics {
                    if let Some(last_applied) = metrics.last_applied {
                        if last_applied >= log_id {
                            node_metrics.insert(metrics.id, metrics);
                        }
                    }
                }
            }
            if node_metrics.len() == self.clients.len() {
                return Ok(node_metrics);
            }
            reattempts -= 1;
            if reattempts == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(anyhow::anyhow!("wait_for_last_applied_log timeout"))
    }
}
