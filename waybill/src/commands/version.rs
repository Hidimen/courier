use clap::Args;

#[derive(Args)]
pub struct VersionArgs {
  #[arg(long)]
  show_modules: bool
}

pub async fn handle(args: VersionArgs) -> anyhow::Result<()> {
  println!(
    "courier {} (build {}/{})",
    env!("CARGO_PKG_VERSION"),
    constants::COMMIT_ID,
    constants::BRANCH
  );

  // if args.show_modules {
  //   println!();
  //   println!("All installed builtin modules:");
  //   for module in builtin::get_all_builtins() {
  //     println!("{}", module);
  //   }
  // }

  Ok(())
}
