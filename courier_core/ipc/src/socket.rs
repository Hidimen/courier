use std::io::Result;

use interprocess::local_socket::{
  GenericNamespaced, ListenerOptions, ToNsName, tokio::prelude::*,
};
use logger::error;
use sync::handle::ShutdownHandle;
use tokio::{io::AsyncWriteExt, task::JoinHandle};

use crate::{Command, Framed};

pub struct LocalSocket {
  handle: JoinHandle<()>,
}

impl LocalSocket {
  pub fn new<C: Command>(shutdown_handle: ShutdownHandle) -> Result<Self> {
    let name = constants::IPC_NAME.to_ns_name::<GenericNamespaced>()?;

    let opts = ListenerOptions::new().name(name);
    let listener: LocalSocketListener = opts.create_tokio()?;

    let handle = tokio::spawn(async move {
      loop {
        let res = tokio::select! {
          res = listener.accept() => res,
          _ =shutdown_handle.wait() => break,
        };

        let mut conn = match res {
          Ok(conn) => conn,
          Err(e) => {
            error!("An error occurred when IPC communicates: {}", e);
            continue;
          },
        };

        tokio::spawn(async move {
          match Framed::parse(&mut conn).await {
            Ok(framed) => {
              let version = framed.version;
              match C::from_framed(framed) {
                Ok(cmd) => match cmd.handle().await {
                  Ok((kind, payload)) => {
                    if let Err(e) = conn
                      .write_all(
                        &Framed {
                          version,
                          kind,
                          len: payload.len() as u32,
                          payload,
                        }
                        .into_bytes(),
                      )
                      .await
                    {
                      error!("An error occurred when IPC communicates: {}", e);
                    }
                  },
                  Err(e) => {
                    error!("An error occurred when IPC communicates: {}", e)
                  },
                },
                Err(e) => {
                  error!("An error occurred when IPC communicates: {}", e)
                },
              }
            },
            Err(e) => error!("An error occurred when IPC communicates: {}", e),
          }
        });
      }
    });

    Ok(Self { handle })
  }

  pub fn abort(&self) {
    self.handle.abort();
  }
}
