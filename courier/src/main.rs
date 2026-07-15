#[tokio::main]
async fn main() {
  let cli = waybill::Cli::new();

  cli.execute().await;
}
