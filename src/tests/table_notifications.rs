use super::*;

use rxqlite_lite_common::NotificationEvent;
use rxqlite_common::{Message, MessageResponse};
use chrono::{/*DateTime,*/ Utc};
use rxqlite_notification::{Action, Notification};
use rxqlite_tests_common::TestTlsConfig;

use super::consts::NOTIFICATIONS_READ_TIMEOUT;

use super::consts::{LEADER_VACATION_RETRIES,DELAY_BETWEEN_LEADER_VACATION_RETRIES};

fn do_notifications(test_name: &str,
                  tls_config: Option<TestTlsConfig>) {
    let rt = Runtime::new().unwrap();

    let _ = rt.block_on(async {
        //const QUERY: &str ="SELECT name,birth_date from _test_user_ where name = ?";
        let mut tm = TestManager::new(test_name, 3, tls_config);
        //tm.keep_temp_directories=keep_temp_directories;
        tm.wait_for_cluster_established(1, 60).await.unwrap();
        let notifications_addr = tm.instances.get(&1).unwrap().notifications_addr.clone();
        let client = tm.clients.get_mut(&1).unwrap();

        client.notification_stream_manager
            .start_listening_for_table_notifications(&notifications_addr,&"_test_user2_".into())
            .await
            .unwrap();

        let message = Message::Execute(
            "CREATE TABLE IF NOT EXISTS _test_user_ (
      id INTEGER PRIMARY KEY,
      name TEXT NOT NULL UNIQUE,
      birth_date DATETIME NOT NULL
      )"
            .into(),
            vec![],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        
        

        let message = response.data.unwrap();
        match message {
            MessageResponse::QueryResult(_) => {}
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        }
        
        let message = Message::Execute(
            "CREATE TABLE IF NOT EXISTS _test_user2_ (
      id INTEGER PRIMARY KEY,
      name TEXT NOT NULL UNIQUE,
      birth_date DATETIME NOT NULL
      )"
            .into(),
            vec![],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        
        

        let message = response.data.unwrap();
        match message {
            MessageResponse::QueryResult(_) => {}
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        }
        
        let name = "Ha";
        let birth_date = Utc::now();
        let message = Message::Execute(
            "INSERT INTO _test_user_ (name,birth_date) VALUES (?,?)".into(),
            vec![name.into(), birth_date.into()],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        let message = response.data.unwrap();
        let last_insert_rowid=match message {
            MessageResponse::QueryResult(res) => {
              assert_eq!(res.changes , 1);
              res.last_insert_rowid
            }
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        };
        
        let message = Message::Execute(
              "INSERT INTO _test_user2_ (name,birth_date) VALUES (?,?)".into(),
              vec![name.into(), birth_date.into()],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        let message = response.data.unwrap();
        let last_insert_rowid2 = match message {
            MessageResponse::QueryResult(res) => {
              assert_eq!(res.changes , 1);
              res.last_insert_rowid
            }
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        };
          
        //tm.wait_for_last_applied_log(response.log_id,60).await.unwrap();

        // now we check for notification, will do with this:

        let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
        let message = notification_stream
            .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
            .await
            .unwrap();
        assert!(message.is_some());
        let insert_row_id2 = match message.unwrap() {
            NotificationEvent::Notification(Notification::Update {
                action,
                database,
                table,
                row_id,
            }) => {
                assert_eq!(action, Action::SQLITE_INSERT);
                assert_eq!(&database, "main");
                assert_eq!(&table, "_test_user2_");
                row_id
            }
        };
        assert_eq!(last_insert_rowid2,insert_row_id2);
        let message = Message::Execute(
            "DELETE FROM _test_user_ WHERE name = ?".into(),
            vec![name.into()],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        let message = response.data.unwrap();
        match message {
            MessageResponse::QueryResult(res) => {
              assert_eq!(res.changes , 1);
              assert_eq!(last_insert_rowid,res.last_insert_rowid);
            }
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        }
        
        let message = Message::Execute(
            "DELETE FROM _test_user2_ WHERE name = ?".into(),
            vec![name.into()],
        );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        let message = response.data.unwrap();
        match message {
            MessageResponse::QueryResult(res) => {
              assert_eq!(res.changes , 1);
              assert_eq!(res.last_insert_rowid,insert_row_id2);
            }
            MessageResponse::Rows(_) => panic!("unexpected database response"),
            MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
            MessageResponse::Error(err) => panic!("{}", err),
        }
        let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
        let message = notification_stream
            .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
            .await
            .unwrap();
        assert!(message.is_some());
        match message.unwrap() {
            NotificationEvent::Notification(Notification::Update {
                action,
                database,
                table,
                row_id,
            }) => {
                assert_eq!(action, Action::SQLITE_DELETE);
                assert_eq!(&database, "main");
                assert_eq!(&table, "_test_user2_");
                assert_eq!(insert_row_id2,row_id);
            }
        }
        
    });
}


fn do_notifications2(test_name: &str,
                  tls_config: Option<TestTlsConfig>) {
    let rt = Runtime::new().unwrap();

    let _ = rt.block_on(async {
        //const QUERY: &str ="SELECT name,birth_date from _test_user_ where name = ?";
        let mut tm = TestManager::new(test_name, 3, tls_config);
        //tm.keep_temp_directories=keep_temp_directories;
        //#[cfg(not(target_os = "linux"))]
        const MAX_ITER:usize = 5;
        /*// on linux we wait DELAY_BETWEEN_KILL_AND_START
        // for the OS to let us reuse node sockets (https://github.com/HaHa421/rxqlite/issues/8)
        #[cfg(target_os = "linux")]
        const MAX_ITER:usize = 2;
        */
        for i in 0..MAX_ITER{
          tm.wait_for_cluster_established(1, 60).await.unwrap();
          let notifications_addr = tm.instances.get(&1).unwrap().notifications_addr.clone();
          let client = tm.clients.get_mut(&1).unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          client.notification_stream_manager
              .start_listening_for_table_notifications(&notifications_addr,&"_test_user2_".into())
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          let message = Message::Execute(
              "CREATE TABLE IF NOT EXISTS _test_user2_ (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        birth_date DATETIME NOT NULL
        )"
              .into(),
              vec![],
          );
        let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();
        
        let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(_) => {}
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }
          
          let message = Message::Execute(
              "CREATE TABLE IF NOT EXISTS _test_user_ (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        birth_date DATETIME NOT NULL
        )"
              .into(),
              vec![],
          );
          let response = client.sql_with_retries_and_delay(&message,
          LEADER_VACATION_RETRIES,
          DELAY_BETWEEN_LEADER_VACATION_RETRIES,
        ).await.unwrap();

          let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(_) => {}
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }
          let name = "Ha";
          let birth_date = Utc::now();
          let message = Message::Execute(
              "INSERT INTO _test_user_ (name,birth_date) VALUES (?,?)".into(),
              vec![name.into(), birth_date.into()],
          );
          let response = client.sql_with_retries_and_delay(&message,
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();
          let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }
          
          let message = Message::Execute(
              "INSERT INTO _test_user2_ (name,birth_date) VALUES (?,?)".into(),
              vec![name.into(), birth_date.into()],
          );
          let response = client.sql_with_retries_and_delay(&message,
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();
          let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }

          //tm.wait_for_last_applied_log(response.log_id,60).await.unwrap();

          // now we check for notification, will do with this:
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
          let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
              .await
              .unwrap();
          assert!(message.is_some());
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          let insert_row_id = match message.unwrap() {
              NotificationEvent::Notification(Notification::Update {
                  action,
                  database,
                  table,
                  row_id,
              }) => {
                  assert_eq!(action, Action::SQLITE_INSERT);
                  assert_eq!(&database, "main");
                  assert_eq!(&table, "_test_user2_");
                  row_id
              }
          };
          let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT/64)
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          assert!(message.is_none());
          let message = Message::Execute(
              "DELETE FROM _test_user_ WHERE name = ?".into(),
              vec![name.into()],
          );
          let response = client.sql_with_retries_and_delay(&message,
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();
          let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }
          
          let message = Message::Execute(
              "DELETE FROM _test_user2_ WHERE name = ?".into(),
              vec![name.into()],
          );
          let response = client.sql_with_retries_and_delay(&message,
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();
          let message = response.data.unwrap();
          match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
          }
          
          let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
              .await
              .unwrap();
          assert!(message.is_some());
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          match message.unwrap() {
              NotificationEvent::Notification(Notification::Update {
                  action,
                  database,
                  table,
                  row_id,
              }) => {
                  assert_eq!(action, Action::SQLITE_DELETE);
                  assert_eq!(&database, "main");
                  assert_eq!(&table, "_test_user2_");
                  assert_eq!(insert_row_id,row_id);
              }
          
          }
          let message = client.notification_stream_manager.notification_stream
              .as_mut().unwrap()
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT/64)
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          assert!(message.is_none());
          client.notification_stream_manager
            .stop_listening_for_table_notifications(&"_test_user2_".into())
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          if i < MAX_ITER - 1 {
            tm.kill_all().unwrap();
            /*
            #[cfg(target_os = "linux")]
            {
               tokio::time::sleep(DELAY_BETWEEN_KILL_AND_START).await;
            }
            */
            tm.start().unwrap();
          }
        }
    });
}

fn do_notifications3(test_name: &str,
                  tls_config: Option<TestTlsConfig>) {
    let rt = Runtime::new().unwrap();

    let _ = rt.block_on(async {
        //const QUERY: &str ="SELECT name,birth_date from _test_user_ where name = ?";
        let mut tm = TestManager::new(test_name, 3, tls_config);
        //tm.keep_temp_directories=keep_temp_directories;
        //#[cfg(not(target_os = "linux"))]
        const MAX_ITER:usize = 2;
        /*// on linux we wait DELAY_BETWEEN_KILL_AND_START
        // for the OS to let us reuse node sockets (https://github.com/HaHa421/rxqlite/issues/8)
        #[cfg(target_os = "linux")]
        const MAX_ITER:usize = 2;
        */
        tm.wait_for_cluster_established(1, 60).await.unwrap();
        for _ in 0..MAX_ITER{
          let mut notifications_addresses:HashMap<NodeId,String> = Default::default();
          for (node_id,instance) in tm.instances.iter() {
            let notifications_addr = instance.notifications_addr.clone();
            notifications_addresses.insert(*node_id,notifications_addr);
          }
            
          for (node_id,client) in tm.clients.iter_mut() {
            
            client.notification_stream_manager
                .start_listening_for_table_notifications(&notifications_addresses.get(node_id).unwrap(),&"_test_user2_".into())
                .await
                .unwrap();

            let message = Message::Execute(
                "CREATE TABLE IF NOT EXISTS _test_user_ (
          id INTEGER PRIMARY KEY,
          name TEXT NOT NULL UNIQUE,
          birth_date DATETIME NOT NULL
          )"
                .into(),
                vec![],
            );
            let response = client.sql_with_retries_and_delay(&message.into(),
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();

            let message = response.data.unwrap();
            match message {
                MessageResponse::QueryResult(_) => {}
                MessageResponse::Rows(_) => panic!("unexpected database response"),
                MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
                MessageResponse::Error(err) => panic!("{}", err),
            }
            let message = Message::Execute(
                "CREATE TABLE IF NOT EXISTS _test_user2_ (
          id INTEGER PRIMARY KEY,
          name TEXT NOT NULL UNIQUE,
          birth_date DATETIME NOT NULL
          )"
                .into(),
                vec![],
            );
            
            
            let response = client.sql_with_retries_and_delay(&message.into(),
            LEADER_VACATION_RETRIES,
            DELAY_BETWEEN_LEADER_VACATION_RETRIES,
          ).await.unwrap();

            let message = response.data.unwrap();
            match message {
                MessageResponse::QueryResult(_) => {}
                MessageResponse::Rows(_) => panic!("unexpected database response"),
                MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
                MessageResponse::Error(err) => panic!("{}", err),
            }
            let name = "Ha";
            let birth_date = Utc::now();
            let message = Message::Execute(
                "INSERT INTO _test_user_ (name,birth_date) VALUES (?,?)".into(),
                vec![name.into(), birth_date.into()],
            );
            let response = client.sql_with_retries_and_delay(&message.into(),
              LEADER_VACATION_RETRIES,
              DELAY_BETWEEN_LEADER_VACATION_RETRIES,
            ).await.unwrap();
            let message = response.data.unwrap();
            match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
            }

            //tm.wait_for_last_applied_log(response.log_id,60).await.unwrap();

            // now we check for notification, will do with this:

            
            let message = client.notification_stream_manager.notification_stream
              .as_mut().unwrap()
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT/64)
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          assert!(message.is_none());
          
          let name = "Ha";
            let birth_date = Utc::now();
            let message = Message::Execute(
                "INSERT INTO _test_user2_ (name,birth_date) VALUES (?,?)".into(),
                vec![name.into(), birth_date.into()],
            );
            let response = client.sql_with_retries_and_delay(&message.into(),
              LEADER_VACATION_RETRIES,
              DELAY_BETWEEN_LEADER_VACATION_RETRIES,
            ).await.unwrap();
            let message = response.data.unwrap();
            match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
            }
            
            let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
            
            let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
                .await
                .unwrap();
            assert!(message.is_some());
            let insert_row_id = match message.unwrap() {
                  NotificationEvent::Notification(Notification::Update {
                  action,
                  database,
                  table,
                  row_id,
                  }) => {
                    assert_eq!(action, Action::SQLITE_INSERT);
                    assert_eq!(&database, "main");
                    assert_eq!(&table, "_test_user2_");
                    row_id
                  }
              //_other=>panic!("unexpected message"),
            };
            let message = Message::Execute(
                "DELETE FROM _test_user_ WHERE name = ?".into(),
                vec![name.into()],
            );
            let response = client.sql_with_retries_and_delay(&message.into(),
              LEADER_VACATION_RETRIES,
              DELAY_BETWEEN_LEADER_VACATION_RETRIES,
            ).await.unwrap();
            let message = response.data.unwrap();
            match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
            }
            
            let message = client.notification_stream_manager.notification_stream
              .as_mut().unwrap()
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT/64)
              .await
              .unwrap();
          //println!("**********************{}({})[{}]********************",file!(),line!(),i);
          assert!(message.is_none());
          
          
          let message = Message::Execute(
                "DELETE FROM _test_user2_ WHERE name = ?".into(),
                vec![name.into()],
            );
            let response = client.sql_with_retries_and_delay(&message.into(),
              LEADER_VACATION_RETRIES,
              DELAY_BETWEEN_LEADER_VACATION_RETRIES,
            ).await.unwrap();
            let message = response.data.unwrap();
            match message {
              MessageResponse::QueryResult(res) => assert!(res.changes == 1),
              MessageResponse::Rows(_) => panic!("unexpected database response"),
              MessageResponse::QueryResultsAndRows(_) => panic!("unexpected database response"),
              MessageResponse::Error(err) => panic!("{}", err),
            }
          
            let notification_stream = client.notification_stream_manager.notification_stream.as_mut().unwrap();
            
            let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT)
              .await
                .unwrap();
            assert!(message.is_some());
            match message.unwrap() {
                  NotificationEvent::Notification(Notification::Update {
                    action,
                    database,
                    table,
                    row_id,
                  }) => {
                    assert_eq!(action, Action::SQLITE_DELETE);
                    assert_eq!(&database, "main");
                    assert_eq!(&table, "_test_user2_");
                    assert_eq!(insert_row_id,row_id);
                  }
              //_other=>panic!("unexpected message"),
            }
            let message = notification_stream
              .read_timeout(NOTIFICATIONS_READ_TIMEOUT/64)
              .await
                .unwrap();
            assert!(message.is_none());
            client.notification_stream_manager
                .stop_listening_for_table_notifications(&"_test_user2_".into())
                .await
                .unwrap();
          }
        }
    });
}

#[tracing_test::traced_test]
#[test]
fn table_notifications() {
    do_notifications("table_notifications", None);
}

#[tracing_test::traced_test]
#[test]
fn table_notifications_insecure_ssl() {
    do_notifications(
        "table_notifications_insecure_ssl", 
        Some(TestTlsConfig::default().accept_invalid_certificates(true)),
    );
}

#[tracing_test::traced_test]
#[test]
fn table_notifications2_no_ssl() {
    do_notifications2("table_notifications2_no_ssl", None);
    
}

#[tracing_test::traced_test]
#[test]
fn table_notifications2_insecure_ssl() {
    do_notifications2(
        "table_notifications2_insecure_ssl",
        Some(TestTlsConfig::default().accept_invalid_certificates(true)),
    );
    
    
}

#[tracing_test::traced_test]
#[test]
fn table_notifications3_no_ssl() {
    do_notifications3("table_notifications3_no_ssl", None);
    
}

#[tracing_test::traced_test]
#[test]
fn table_notifications3_insecure_ssl() {
    do_notifications3(
        "table_notifications3_insecure_ssl",
        Some(TestTlsConfig::default().accept_invalid_certificates(true)),
    );
    
    
}
