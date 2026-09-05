use clap::Args;

#[derive(Args)]
pub struct VersionArgs {
  #[arg(long)]
  show_builtins: bool,
}

pub fn handle(args: VersionArgs) -> anyhow::Result<()> {
  println!(
    "courier {} (build {}/{})",
    env!("CARGO_PKG_VERSION"),
    constants::COMMIT_ID,
    constants::BRANCH
  );

  if args.show_builtins {
    println!();
    println!("All installed builtin modules:");
    for module in builtin::get_all_builtins() {
      println!("{}", module);
    }
  }

  Ok(())
}
