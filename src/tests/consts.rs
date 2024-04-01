/*
#[cfg(target_os = "linux")]
pub const DELAY_BETWEEN_KILL_AND_START: tokio::time::Duration= tokio::time::Duration::from_secs(0);
*/
pub const DELAY_BETWEEN_LEADER_VACATION_RETRIES: tokio::time::Duration = 
  tokio::time::Duration::from_secs(1);

pub const LEADER_VACATION_RETRIES: usize = 10;

pub const NOTIFICATIONS_READ_TIMEOUT: tokio::time::Duration= tokio::time::Duration::from_secs(120);

#[allow(dead_code)]
pub const WAIT_FOR_LAST_APPLIED_LOG_SECS: usize= 120;