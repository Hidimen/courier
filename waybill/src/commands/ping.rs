use std::io::ErrorKind;

use clap::Args;
use depot::control::standard::{PING_CODE, VERSION};
use ipc::{Client, Response};
use tokio_util::bytes::Bytes;

use crate::control::CommandResponse;

#[derive(Args)]
pub struct PingArgs {}

pub fn handle(_args: PingArgs) -> anyhow::Result<()> {
  let mut client = match Client::new() {
    Ok(client) => client,
    Err(e)
      if e.kind() == ErrorKind::NotFound
        || e.kind() == ErrorKind::ConnectionRefused =>
    {
      eprintln!("Courier instance not starting");
      return Ok(());
    },
    Err(e) => anyhow::bail!(e),
  };
  let mut response =
    client.send::<CommandResponse>(VERSION, PING_CODE, Bytes::new())?;
  response.handle()?;

  Ok(())
}
