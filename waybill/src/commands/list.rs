use std::io::ErrorKind;

use anyhow::Context;
use clap::Args;
use depot::control::standard::{QUERY_POD_CODE, VERSION};
use ipc::{Client, Response};
use tokio_util::bytes::Bytes;

use crate::control::CommandResponse;

#[derive(Args)]
pub struct ListArgs {}

pub fn handle(_args: ListArgs) -> anyhow::Result<()> {
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
  let mut response = client
    .send::<CommandResponse>(VERSION, QUERY_POD_CODE, Bytes::new())
    .context("Unable to send command")?;
  response.handle().context("Unable to handle response")?;

  Ok(())
}
