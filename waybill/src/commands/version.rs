use chrono::{Local, TimeZone};
use clap::Args;

#[derive(Args)]
pub struct VersionArgs {
  #[arg(long)]
  show_builtins: bool,
}

pub fn handle(args: VersionArgs) -> anyhow::Result<()> {
  println!(
    "courier {} (commit {}, branch {}), built at {}",
    env!("CARGO_PKG_VERSION"),
    constants::COMMIT_ID,
    constants::BRANCH,
    Local
      .timestamp_opt(constants::COMPILE_TIME.parse::<i64>().unwrap(), 0)
      .unwrap()
      .format("%Y-%m-%d %H:%M:%S")
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
