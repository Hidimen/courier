use clap::Args;
use depot::control::standard::{PING_CODE, VERSION};
use ipc::{Client, Response};
use tokio_util::bytes::Bytes;

use crate::control::CommandResponse;

#[derive(Args)]
pub struct PingArgs {}

pub fn handle(_args: PingArgs) -> anyhow::Result<()> {
  let mut client = Client::new()?;
  let mut response =
    client.send::<CommandResponse>(VERSION, PING_CODE, Bytes::new())?;
  response.handle()?;

  Ok(())
}
