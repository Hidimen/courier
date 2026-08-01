use clap::Args;

#[derive(Args)]
pub struct VersionArgs;

impl VersionArgs {
  pub async fn execute(&self) {
    println!(
      "courier {} (build {}/{})",
      env!("CARGO_PKG_VERSION"),
      constants::COMMIT_ID,
      constants::BRANCH
    )
  }
}
