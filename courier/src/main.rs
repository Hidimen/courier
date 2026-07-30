fn main() {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .name("courier:runtime")
    .thread_name("courier:worker")
    .build()
    .unwrap();

  runtime.block_on(async move {
    let cli = waybill::Cli::new();

    cli.execute().await;
  });
}
