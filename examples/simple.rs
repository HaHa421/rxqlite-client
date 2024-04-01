
#[tokio::main]
async fn main() {
  let client = rxqlite_client::RXQLiteClient::new(1,"http://127.0.0.1:21001");
}
