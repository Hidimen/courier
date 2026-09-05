fn main() -> anyhow::Result<()> {
  let cli = waybill::Cli::new();
  cli.execute()
}
