#![deny(warnings)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;

use rxqlite_lite_common::*;
use reqwest::{Client, ClientBuilder};
use serde::de::DeserializeOwned;
//use serde::Deserialize;
use serde::Serialize;

use tokio::time::{Duration};
//use tokio::io::{AsyncReadExt,AsyncWriteExt};
pub type Request = rxqlite_common::Message;

#[cfg(any(test,feature = "test-dependency"))]
pub mod tests;

//use crate::TypeConfig;

use rxqlite_common::{Message, MessageResponse, RSQliteClientTlsConfig, Rows, Value};


pub struct RXQLiteClientBuilder {
    node_id: NodeId,
    node_addr: String,
    //tls_config: Option<RSQliteClientTlsConfig>,
    use_tls: bool,
    accept_invalid_certificates: bool,
}

impl RXQLiteClientBuilder {
    pub fn new(node_id: NodeId, node_addr: String) -> Self {
        Self {
            node_id,
            node_addr,
            //tls_config: None,
            use_tls: false,
            accept_invalid_certificates: false,
        }
    }
    pub fn tls_config(mut self, tls_config: Option<RSQliteClientTlsConfig>) -> Self {
        if let Some(tls_config) = tls_config {
            self.use_tls = true;
            self.accept_invalid_certificates = tls_config.accept_invalid_certificates;
        } else {
            self.use_tls = false;
            self.accept_invalid_certificates = false;
        }
        self
    }
    pub fn use_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }
    pub fn accept_invalid_certificates(mut self, accept_invalid_certificates: bool) -> Self {
        self.accept_invalid_certificates = accept_invalid_certificates;
        self
    }
    pub fn build(self) -> RXQLiteClient {
        let mut inner = ClientBuilder::new();
        let use_tls = if self.use_tls {
            if self.accept_invalid_certificates {
                inner = inner.danger_accept_invalid_certs(true);
            }
            true
        } else {
            false
        };
        let inner = inner.build().unwrap();
        RXQLiteClient {
            node: Arc::new(Mutex::new((self.node_id, self.node_addr.clone()))),
            leader: Arc::new(Mutex::new((self.node_id, self.node_addr))),
            inner,
            use_tls,
            //accept_invalid_certificates: self.accept_invalid_certificates,
            notification_stream_manager: NetStreamManager::new(use_tls,self.accept_invalid_certificates),
            
        }
    }
}

//#[derive(Default)]
pub struct NetStreamManager {
  pub listening_for_any_notifications: bool,
  pub listened_tables: BTreeSet<String>,
  pub notification_stream: Option<NetStream>,
  pub use_tls: bool,
  pub accept_invalid_certificates: bool,
}

impl NetStreamManager {
  fn new(use_tls: bool,
    accept_invalid_certificates: bool)-> Self {
    Self {
      listening_for_any_notifications: false,
      listened_tables: Default::default(),
      notification_stream : None,
      use_tls,
      accept_invalid_certificates,
    }
  }
}

impl NetStreamManager {
    pub fn stop_listening_for_all(&mut self) {
      //server will do what needs to be done
      self.notification_stream.take();
      self.listened_tables.clear();
      self.listening_for_any_notifications=false;
    }
    fn is_listening(&self)->bool {
      self.listening_for_any_notifications || self.listened_tables.len() > 0
    }
    pub async fn stop_listening_for_notifications(&mut self) -> anyhow::Result<()> {
        if self.notification_stream.is_none() {
            return Ok(());
        }
        if let Err(err) = self.notification_stream
            .as_mut()
            .unwrap()
            .write(NotificationRequest::Unregister)
            .await {
          self.stop_listening_for_all();
          return Err(err.into());
        }
        self.listening_for_any_notifications=false;
        if !self.is_listening() {
          self.notification_stream.take();
          
        }
        Ok(())
    }
    pub async fn start_listening_for_notifications(
        &mut self,
        notifications_addr: &str,
    ) -> anyhow::Result<()> {
        if self.listening_for_any_notifications {
          return Ok(());
        }
        if self.notification_stream.is_none() {
          let notification_stream = NetStream::new(notifications_addr,
            if self.use_tls {Some(self.accept_invalid_certificates)} else {None}).await?;
          self.notification_stream = Some(notification_stream);
        }
        if let Err(err) = self.notification_stream.as_mut().unwrap()
          .write(NotificationRequest::Register)
          .await {
            self.stop_listening_for_all();
            return Err(err.into());
        }
        self.listening_for_any_notifications=true;
        Ok(())
    }
    
    pub async fn stop_listening_for_table_notifications(&mut self,table: &String) -> anyhow::Result<()> {
        if !self.listened_tables.contains(table) {
            return Ok(());
        }
        if let Err(err) = self.notification_stream
            .as_mut()
            .unwrap()
            .write(NotificationRequest::UnregisterForTable(table.clone()))
            .await {
          self.stop_listening_for_all();
          return Err(err.into());
        }
        self.listened_tables.remove(table);
        if !self.is_listening() {
          self.notification_stream.take();
          
        }
        Ok(())
    }
    pub async fn start_listening_for_table_notifications(
        &mut self,
        notifications_addr: &str,
        table:& String,
    ) -> anyhow::Result<()> {
        if self.listened_tables.contains(table) {
            return Ok(());
        }
        if self.notification_stream.is_none() {
          let notification_stream = NetStream::new(notifications_addr,
            if self.use_tls {Some(self.accept_invalid_certificates)} else {None}).await?;
          self.notification_stream = Some(notification_stream);
        }
        if let Err(err) = self.notification_stream.as_mut().unwrap()
          .write(NotificationRequest::RegisterForTable(table.clone()))
          .await {
            self.stop_listening_for_all();
            return Err(err.into());
        }
        self.listened_tables.insert(table.clone());
        Ok(())
    }
    
}

pub struct RXQLiteClient {
    /// The leader node to send request to.
    ///
    /// All traffic should be sent to the leader in a cluster.
    pub leader: Arc<Mutex<(NodeId, String)>>,

    /// The original node to send request to.
    ///
    /// Mainly used to get node metrics.
    pub node: Arc<Mutex<(NodeId, String)>>,

    pub inner: Client,

    use_tls: bool,

    //accept_invalid_certificates: bool,

    pub notification_stream_manager: NetStreamManager,
}

impl RXQLiteClient {
    /*
    pub fn with_options(options: &ConnectOptions) -> Self {
        let mut inner = ClientBuilder::new();
        let accept_invalid_certificates = if let Some(tls_config) = options.tls_config.as_ref() {
            inner = inner.use_rustls_tls();
            if tls_config.accept_invalid_certificates {
                inner = inner.danger_accept_invalid_certs(true);
                true
            } else {
                false
            }
        } else {
            false
        };
        let inner = inner.build().unwrap();
        let node = Arc::new(Mutex::new((
            options.leader_id,
            format!("{}:{}", options.leader_host, options.leader_port),
        )));
        let leader = Arc::new(Mutex::new((
            options.leader_id,
            format!("{}:{}", options.leader_host, options.leader_port),
        )));
        Self {
            node,
            leader,
            inner,
            use_tls: options.tls_config.is_some(),
            notification_stream: None,
            accept_invalid_certificates,
        }
    }
    */
    /// Create a client with a leader node id and a node manager to get node address by node id.
    pub fn new(node_id: NodeId, node_addr: &str) -> Self {
        Self {
            node: Arc::new(Mutex::new((node_id, node_addr.into()))),
            leader: Arc::new(Mutex::new((node_id, node_addr.into()))),
            inner: Client::new(),
            use_tls: false,
            //accept_invalid_certificates: false,
            notification_stream_manager: NetStreamManager::new(false,false),
        }
    }

    // --- Application API

    /// Submit a write request to the raft cluster.
    ///
    /// The request will be processed by raft protocol: it will be replicated to a quorum and then
    /// will be applied to state machine.
    ///
    /// The result of applying the request will be returned.
    pub async fn sql(
        &self,
        req: &Request,
    ) -> Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> {
        self.send_rpc_to_leader("lite-api/sql", Some(req)).await
    }
    
    pub async fn sql_with_retries_and_delay(
        &self,
        req: &Request,
        mut retries: usize,
        delay_between_retries: Duration,
    ) -> Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> {
        retries += 1;
        loop {
          let res: Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> =
            self.send_rpc_to_leader("lite-api/sql", Some(req)).await;
          match res {
            Ok(res)=>return Ok(res),
            Err(rpc_err)=> {
              if let RPCError::RemoteError(remote_err) = &rpc_err {
                let raft_err/*: &RaftError<_>*/ = remote_err.source.try_as_ref();

                if let Some(ForwardToLeader {
                    leader_id,   //: Some(leader_id),
                    leader_node : _, //: Some(leader_node),
                    ..
                }) = raft_err {
                  if leader_id.is_some() {
                    return Err(rpc_err);
                  } else {
                    retries-=1;
                    if retries == 0 {
                      return Err(rpc_err);
                    }
                    tokio::time::sleep(delay_between_retries).await;
                  }
                } else {
                  return Err(rpc_err);
                }
              } else {
                return Err(rpc_err);
              }
            }
          }
        }
    }
    
    pub async fn consistent_sql(
        &self,
        req: &Request,
    ) -> Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> {
        self.send_rpc_to_leader("lite-api/sql-consistent", Some(req))
            .await
    }
    pub async fn execute(
        &self,
        query: &str,
        arguments: Vec<Value>,
    ) -> Result<Rows, RXQLiteError> {
        let req = Message::Execute(query.into(), arguments);
        let res: Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> = self
            .send_rpc_to_leader("lite-api/sql-consistent", Some(&req))
            .await;
        match res {
            Ok(res) => match res.data {
                Some(res) => match res {
                    MessageResponse::Rows(rows) => Ok(rows),
                    MessageResponse::Error(error) => Err(anyhow::anyhow!(error)),
                },
                _ => Ok(Rows::default()),
            },
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
    pub async fn fetch_all(
        &self,
        query: &str,
        arguments: Vec<Value>,
    ) -> Result<Rows, RXQLiteError> {
        let req = Message::Fetch(query.into(), arguments);
        let res: Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> = self
            .send_rpc_to_leader("lite-api/sql-consistent", Some(&req))
            .await;
        match res {
            Ok(res) => match res.data {
                Some(res) => match res {
                    MessageResponse::Rows(rows) => Ok(rows),
                    MessageResponse::Error(error) => Err(anyhow::anyhow!(error)),
                },
                _ => Ok(Rows::default()),
            },
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
    pub async fn fetch_one(
        &self,
        query: &str,
        arguments: Vec<Value>,
    ) -> Result<rxqlite_common::Row, RXQLiteError> {
        let req = Message::FetchOne(query.into(), arguments);
        let res: Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> = self
            .send_rpc_to_leader("lite-api/sql-consistent", Some(&req))
            .await;
        match res {
            Ok(res) => match res.data {
                Some(res) => match res {
                    MessageResponse::Rows(mut rows) => {
                        if rows.len() >= 1 {
                            Ok(rows.remove(0))
                        } else {
                            Err(anyhow::anyhow!("no row matching query"))
                        }
                    }
                    MessageResponse::Error(error) => Err(anyhow::anyhow!(error)),
                },
                _ => Err(anyhow::anyhow!("no row matching query")),
            },
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
    pub async fn fetch_optional(
        &self,
        query: &str,
        arguments: Vec<Value>,
    ) -> Result<Option<rxqlite_common::Row>, RXQLiteError> {
        let req = Message::FetchOptional(query.into(), arguments);
        let res: Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> = self
            .send_rpc_to_leader("lite-api/sql-consistent", Some(&req))
            .await;
        match res {
            Ok(res) => match res.data {
                Some(res) => match res {
                    MessageResponse::Rows(mut rows) => {
                        if rows.len() >= 1 {
                            Ok(Some(rows.remove(0)))
                        } else {
                            Ok(None)
                        }
                    }
                    MessageResponse::Error(error) => Err(anyhow::anyhow!(error)),
                },
                _ => Ok(None),
            },
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    // --- Cluster management API

    /// Add a node as learner.
    ///
    /// The node to add has to exist, i.e., being added with `write(ExampleRequest::AddNode{})`
    pub async fn add_learner(
        &self,
        req: (NodeId, String, String),
    ) -> Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> {
        self.send_rpc_to_leader("lite-cluster/add-learner", Some(&req))
            .await
    }

    /// Change membership to the specified set of nodes.
    ///
    /// All nodes in `req` have to be already added as learner with [`add_learner`],
    /// or an error [`LearnerNotFound`] will be returned.
    pub async fn change_membership(
        &self,
        req: &BTreeSet<NodeId>,
    ) -> Result<ClientWriteResponse, RPCError<RaftError<ClientWriteError>>> {
        self.send_rpc_to_leader("lite-cluster/change-membership", Some(req))
            .await
    }

    /// Get the metrics about the cluster.
    ///
    /// Metrics contains various information about the cluster, such as current leader,
    /// membership config, replication status etc.
    /// See [`RaftMetrics`].
    pub async fn metrics(&self) -> Result<RaftMetrics, RPCError<Infallible>> {
        self.do_send_rpc_to_leader("lite-cluster/metrics", None::<&()>)
            .await
    }

    /// Get the metrics about the cluster from the original node.
    ///
    /// Metrics contains various information about the cluster, such as current leader,
    /// membership config, replication status etc.
    /// See [`RaftMetrics`].
    pub async fn node_metrics(&self) -> Result<RaftMetrics, RPCError<Infallible>> {
        self.do_send_rpc_to_node(&self.node, "lite-cluster/metrics", None::<&()>)
            .await
    }

    // --- Internal methods

    /// Send RPC to specified node.
    ///
    /// It sends out a POST request if `req` is Some. Otherwise a GET request.
    /// The remote endpoint must respond a reply in form of `Result<T, E>`.
    /// An `Err` happened on remote will be wrapped in an [`RPCError::RemoteError`].
    async fn do_send_rpc_to_node<Req, Resp, Err>(
        &self,
        dest_node: &Arc<Mutex<(NodeId, String)>>,
        uri: &str,
        req: Option<&Req>,
    ) -> Result<Resp, RPCError<Err>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error + Serialize + DeserializeOwned,
    {
        let (node_id, url) = {
            let t = dest_node.lock().unwrap();
            let target_addr = &t.1;
            (
                t.0,
                format!(
                    "{}://{}/{}",
                    if self.use_tls { "https" } else { "http" },
                    target_addr,
                    uri
                ),
            )
        };

        let resp = if let Some(r) = req {
            tracing::debug!(
                ">>> client send request to {}: {}",
                url,
                serde_json::to_string_pretty(&r).unwrap()
            );
            self.inner.post(url.clone()).json(r)
        } else {
            tracing::debug!(">>> client send request to {}", url,);
            self.inner.get(url.clone())
        }
        .send()
        .await
        .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let res: Result<Resp, Err> = resp
            .json()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        tracing::debug!(
            "<<< client recv reply from {}: {}",
            url,
            serde_json::to_string_pretty(&res).unwrap()
        );

        res.map_err(|e| RPCError::RemoteError(RemoteError::new(node_id, e)))
    }
    /// Send RPC to specified node.
    ///
    /// It sends out a POST request if `req` is Some. Otherwise a GET request.
    /// The remote endpoint must respond a reply in form of `Result<T, E>`.
    /// An `Err` happened on remote will be wrapped in an [`RPCError::RemoteError`].

    async fn do_send_rpc_to_leader<Req, Resp, Err>(
        &self,
        uri: &str,
        req: Option<&Req>,
    ) -> Result<Resp, RPCError<Err>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error + Serialize + DeserializeOwned,
    {
        self.do_send_rpc_to_node(&self.leader, uri, req).await
    }

    /// Try the best to send a request to the leader.
    ///
    /// If the target node is not a leader, a `ForwardToLeader` error will be
    /// returned and this client will retry at most 3 times to contact the updated leader.
    async fn send_rpc_to_leader<Req, Resp, Err>(
        &self,
        uri: &str,
        req: Option<&Req>,
    ) -> Result<Resp, RPCError<Err>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error
            + Serialize
            + DeserializeOwned
            + TryAsRef<ForwardToLeader>
            + Clone,
    {
        // Retry at most 3 times to find a valid leader.
        let mut n_retry = 3;

        loop {
            let res: Result<Resp, RPCError<Err>> = self.do_send_rpc_to_leader(uri, req).await;

            let rpc_err = match res {
                Ok(x) => return Ok(x),
                Err(rpc_err) => rpc_err,
            };

            if let RPCError::RemoteError(remote_err) = &rpc_err {
                let raft_err/*: &RaftError<_>*/ = remote_err.source.try_as_ref();

                if let Some(ForwardToLeader {
                    leader_id,   //: Some(leader_id),
                    leader_node, //: Some(leader_node),
                    ..
                }) = raft_err
                {
                    // Update target to the new leader.
                    if let (Some(leader_id), Some(leader_node)) = (leader_id, leader_node) {
                        let mut t = self.leader.lock().unwrap();
                        let api_addr = leader_node.api_addr.clone();
                        *t = (*leader_id, api_addr);
                    } else {
                        break Err(rpc_err);
                    }

                    n_retry -= 1;
                    if n_retry > 0 {
                        continue;
                    }
                }
            }

            return Err(rpc_err);
        }
    }
}

impl RXQLiteClient {
    
}
