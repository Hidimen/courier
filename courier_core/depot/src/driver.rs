use std::{
  collections::VecDeque,
  io::{self, ErrorKind},
  sync::Arc,
};

use bytes::BytesMut;
use logger::error;
use network::stream::{ReadHalf, Stream, WriteHalf};
use protocol::{Payload, stream::StreamProtocol};
use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};

pub async fn drive_stream<P: StreamProtocol, S: Stream>(
  protocol: P, shutdown_handle: ShutdownHandle,
  connection_count: Arc<ActivityTracker>, stream: S,
) {
  use protocol::stream::*;

  fn step<P: StreamProtocol>(
    protocol: &mut P, peer: std::net::SocketAddr, output: &mut Output,
    input: Input<P::Command>,
  ) -> Result<(), P::Error> {
    let mut context = Context::new(peer, output);
    protocol.step(&mut context, input)
  }

  match stream.peer_addr() {
    Ok(peer) => {
      let mut protocol = protocol;
      let (mut read, mut write) = stream.split();

      connection_count.acquire();

      let mut read_pause = false;
      let mut read_done = false;
      let mut closed_delivered = false;
      let mut flush_requested: bool;
      let mut output = Output::new();
      let mut pending =
        (VecDeque::<Payload>::new(), 0_usize, 32 * 1024, 64 * 1024);

      loop {
        if !read_pause && !read_done {
          let mut buf = BytesMut::zeroed(64 * 1024);
          let res = tokio::select! {
            result = read.read(&mut buf) => result,
            _ = shutdown_handle.wait() => break,
          };

          match res {
            Ok(0) => {
              // EOF: `Data` must never carry an empty payload, so report the
              // peer close through `Closed` instead of an empty `Data`.
              read_done = true;
              closed_delivered = true;
              if let Err(e) = step(
                &mut protocol,
                peer,
                &mut output,
                Input::Closed(close::CloseReason::PeerClose),
              ) {
                error!(
                  "An unrecoverable error occurred when protocol stepping: {}",
                  e
                );
                break;
              }
            },
            Ok(bytes_read) => {
              if let Err(e) = step(
                &mut protocol,
                peer,
                &mut output,
                Input::Data(buf.split_to(bytes_read).freeze().into()),
              ) {
                error!(
                  "An unrecoverable error occurred when protocol stepping: {}",
                  e
                );
                break;
              }
            },
            Err(e) => {
              let reason = match e.kind() {
                ErrorKind::WouldBlock
                | ErrorKind::TimedOut
                | ErrorKind::Interrupted => continue,
                ErrorKind::ConnectionReset => close::CloseReason::PeerReset,
                ErrorKind::ConnectionAborted => close::CloseReason::PeerClose,
                _ => close::CloseReason::TransportError,
              };

              read_done = true;
              closed_delivered = true;
              if let Err(e) =
                step(&mut protocol, peer, &mut output, Input::Closed(reason))
              {
                error!(
                  "An unrecoverable error occurred when protocol stepping: {}",
                  e
                );
                break;
              }
            },
          }

          let (data, len) = output.take_bytes();
          pending.0.extend(data);
          pending.1 += len;
        }

        /*
          Effect Processing Order:
          - Set `flush` flag
          - Set `read_pause` flag
          - Set timers
          - Process close logic
        */
        flush_requested = output.take_flush_effect();
        if let Some(state) = output.take_reader_effect() {
          read_pause = state;
        }
        // TODO: Timer setting
        match output.take_close_effect() {
          None => {
            if let Err(e) = try_drain(&mut pending, &mut write) {
              match e.kind() {
                ErrorKind::WriteZero => {
                  error!("It seems that a stream cannot accept any bytes");
                  break;
                },
                _ => {
                  error!(
                    "An unrecoverable error occurred when writing bytes into stream: {}",
                    e
                  );
                  break;
                },
              }
            }

            if pending.1 >= pending.3 {
              read_pause = true;
            } else if pending.1 <= pending.2 {
              read_pause = false;
            }

            // The socket buffer is full: wait for writability instead of
            // spinning on `try_write`.
            if pending.1 > 0 {
              let res = tokio::select! {
                result = write.writable() => result,
                _ = shutdown_handle.wait() => break,
              };
              if let Err(e) = res {
                error!(
                  "An unrecoverable error occurred when waiting for writable: {}",
                  e
                );
                break;
              }
            }
          },
          Some(close::CloseMode::Graceful) => {
            if !closed_delivered {
              closed_delivered = true;
              if let Err(e) = step(
                &mut protocol,
                peer,
                &mut output,
                Input::Closed(close::CloseReason::LocalClose),
              ) {
                error!(
                  "An unrecoverable error occurred when protocol stepping: {}",
                  e
                );
                break;
              }
            }

            if let Err(e) = drain(&mut pending, &mut write).await {
              error!(
                "An unrecoverable error occurred when writing bytes into stream: {}",
                e
              );
              break;
            }
          },
          Some(close::CloseMode::Abort) => {
            if !closed_delivered
              && let Err(e) = step(
                &mut protocol,
                peer,
                &mut output,
                Input::Closed(close::CloseReason::LocalClose),
              )
            {
              error!(
                "An unrecoverable error occurred when protocol stepping: {}",
                e
              );
            }
            break;
          },
        }

        if flush_requested && let Err(e) = write.flush().await {
          error!("An unrecoverable error occurred when flushing: {}", e);
          if !closed_delivered {
            closed_delivered = true;
            if let Err(e) = step(
              &mut protocol,
              peer,
              &mut output,
              Input::Closed(close::CloseReason::LocalClose),
            ) {
              error!(
                "An unrecoverable error occurred when protocol stepping: {}",
                e
              );
            }
          }
        }

        // The read half is gone: flush whatever the protocol produced for
        // the final event, close the write half and stop the loop.
        if read_done {
          if let Err(e) = drain(&mut pending, &mut write).await {
            error!(
              "An unrecoverable error occurred when writing bytes into stream: {}",
              e
            );
          }
          if let Err(e) = write.flush().await {
            error!("An unrecoverable error occurred when flushing: {}", e);
          }
          if let Err(e) = write.shutdown().await {
            error!("An unrecoverable error occurred when shutting down: {}", e);
          }
          break;
        }
      }

      connection_count.release();
    },
    Err(e) => {
      error!("Unable to get peer address: {}", e);
    },
  }
}

fn try_drain<W: WriteHalf>(
  src: &mut (VecDeque<Payload>, usize, usize, usize), dst: &mut W,
) -> io::Result<()> {
  while let Some(bytes) = src.0.front() {
    if bytes.is_empty() {
      src.0.pop_front();
      continue;
    }

    match dst.try_write(bytes) {
      Ok(0) => {
        return Err(io::Error::new(
          ErrorKind::WriteZero,
          "try_write returned 0",
        ));
      },
      Ok(n) => {
        src.1 -= n;
        if n == bytes.len() {
          src.0.pop_front();
        } else {
          let front = src.0.front_mut().unwrap();
          *front = front.slice(n..);
        }
      },
      Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
      Err(e) => return Err(e),
    }
  }

  Ok(())
}

async fn drain<W: WriteHalf>(
  src: &mut (VecDeque<Payload>, usize, usize, usize), dst: &mut W,
) -> io::Result<()> {
  while let Some(bytes) = src.0.pop_front() {
    if bytes.is_empty() {
      continue;
    }

    dst.write_all(&bytes).await?;
    src.1 -= bytes.len();
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use std::{io, sync::Arc};

  use network::transport::{StreamTransport, Transport, tcp::TcpTransport};
  use protocol::{
    Protocol,
    stream::{Context, Input, StreamProtocol},
  };
  use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};
  use tokio::io::{AsyncReadExt, AsyncWriteExt};

  use super::drive_stream;

  #[derive(Clone)]
  struct Echo;

  impl Protocol for Echo {
    type Command = ();
    type Error = io::Error;

    const NAME: &'static str = "test:echo";
  }

  impl StreamProtocol for Echo {
    fn step(
      &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
    ) -> Result<(), Self::Error> {
      if let Input::Data(bytes) = input {
        let _ = ctx.write_bytes(bytes);
      }

      Ok(())
    }
  }

  #[tokio::test]
  async fn echoes_and_stops_at_peer_eof() {
    let listener = TcpTransport::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
      let stream = listener.accept().await.unwrap();
      drive_stream(
        Echo,
        ShutdownHandle::new(1_000),
        Arc::new(ActivityTracker::new()),
        stream,
      )
      .await;
    });

    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();

    for expected in [b"hello".as_slice(), b"world".as_slice()] {
      client.write_all(expected).await.unwrap();

      let mut buf = vec![0_u8; expected.len()];
      client.read_exact(&mut buf).await.unwrap();
      assert_eq!(buf, expected);
    }

    drop(client);
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
      .await
      .unwrap()
      .unwrap();
  }
}
