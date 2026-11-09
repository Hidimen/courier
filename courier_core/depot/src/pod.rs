use std::{
  io::{self, ErrorKind},
  net::SocketAddr,
  sync::Arc,
  time::Duration,
};

use bytes::{Bytes, BytesMut};
use logger::{error, warn};
use network::{
  stream::{ReadHalf, Stream},
  transport::{StreamTransport, Transport},
};
use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};
use tokio::{
  task::{JoinHandle, JoinSet},
  time::sleep,
};
use tokio_mpmc::{ChannelError, channel};

use crate::{DatagramProvider, StreamProvider, driver::*};

#[derive(Debug)]
pub struct Pod {
  pub handle: JoinHandle<()>,
  pub shutdown_handle: ShutdownHandle,
  pub name: String,
  pub id: u32,
}

impl Pod {
  pub fn spawn_stream<Pvd>(
    addr: SocketAddr, shutdown_handle: ShutdownHandle, name: String, id: u32,
  ) -> io::Result<Self>
  where
    Pvd: StreamProvider,
  {
    let transport = <Pvd::Transport as Transport>::bind(addr)?;
    let shutdown_handle_child = shutdown_handle.child();

    let handle = tokio::spawn(async move {
      let connection_count = Arc::new(ActivityTracker::new());

      loop {
        let res = tokio::select! {
          result = transport.accept() => result,
          _ = shutdown_handle_child.wait() => break
        };

        match res {
          Ok(stream) => {
            let protocol = Pvd::protocol();
            let child = shutdown_handle_child.child();
            let connection_count_cloned = connection_count.clone();

            tokio::spawn(async move {
              drive_stream(protocol, child, connection_count_cloned, stream)
                .await;
            });
          },
          Err(e) => match e.kind() {
            ErrorKind::ConnectionAborted => {
              error!("A connection aborted, retrying");
            },
            ErrorKind::WouldBlock => {
              continue;
            },
            ErrorKind::NetworkDown
            | ErrorKind::NetworkUnreachable
            | ErrorKind::HostUnreachable => {
              error!("A momentary mistake occurred, retrying");
              continue;
            },
            _ => {
              error!("An unrecoverable error occurred, graceful exiting");
              break;
            },
          },
        }
      }

      // Entering stopping logic
      let timeout =
        sleep(Duration::from_millis(shutdown_handle_child.timeout() as u64));

      tokio::select! {
        _ = connection_count.wait() => {},
        _ = timeout => {},
      }
    });

    Ok(Self { handle, shutdown_handle, name, id })
  }

  pub fn spawn_datagram<Pvd>(
    addr: SocketAddr, shutdown_handle: ShutdownHandle, name: String, id: u32,
  ) -> io::Result<Self>
  where
    Pvd: DatagramProvider,
  {
    // let transport = Arc::new(T::bind(addr)?);
    // let shutdown_handle_child = shutdown_handle.child();
    // let shutdown_handle_loop = shutdown_handle.child();

    // let handle = tokio::spawn(async move {
    //   let transport = transport;
    //   let provider = provider;

    //   let shutdown_handle_child = shutdown_handle_child;

    //   let (sender, receiver) = channel(1024);

    //   let mut recv_tasks = JoinSet::new();
    //   let mut worker_tasks = JoinSet::new();

    //   #[cfg(not(windows))]
    //   {
    //     for _ in 0..num_cpus::get() {
    //       let transport_cloned = transport.clone();
    //       let transport_worker_cloned = transport.clone();
    //       let protocol_cloned = protocol.clone();
    //       let shutdown_handle_cloned = shutdown_handle_child.child();
    //       let shutdown_handle_worker = shutdown_handle_child.child();

    //       let sender = sender.clone();
    //       let receiver = receiver.clone();

    //       // recv tasks
    //       recv_tasks.spawn(async move {
    //         let mut buf: [u8; 65535] = [0; 65535];

    //         loop {
    //           let res = tokio::select! {
    //             result = transport_cloned.recv_from(&mut buf) => result,
    //             _ = shutdown_handle_cloned.wait() => break
    //           };

    //           match res {
    //             Ok((bytes, peer_addr)) => {
    //               match sender.send((peer_addr, Frame::copy_from_slice(&buf[..bytes]))).await {
    //                 Err(ChannelError::ChannelClosed) => {
    //                   warn!("A `datagram recv task` is stopping for worker channel has been closed");
    //                   break;
    //                 },
    //                 _ => continue
    //               }
    //             },
    //             Err(e) => error!("An error occurred when fetching datagram: {}", e),
    //           }
    //         }
    //       });

    //       worker_tasks.spawn(async move {
    //         let transport = transport_worker_cloned;
    //         let protocol = protocol_cloned;
    //         let pipeline = pipeline_cloned;
    //         let receiver = receiver;

    //         loop {
    //           let res = tokio::select! {
    //             result = receiver.recv() => result,
    //             _ = shutdown_handle_worker.wait() => break,
    //           };

    //           match res {
    //             Ok(Some((peer_addr, frame))) => {
    //               let decoded = protocol.decode((frame, peer_addr)).await;
    //               match decoded {
    //                 Ok(request) => {
    //                   let response = pipeline.handle(request).await;
    //                   let frame = match response {
    //                     Ok(response) => match protocol.encode(response).await {
    //                       Ok(frame) => frame,
    //                       Err(e) => {
    //                         error!("An error occurred when encoding: {}", e);
    //                         continue;
    //                       },
    //                     },
    //                     Err(e) => {
    //                       error!(
    //                         "An error occurred when middleware processing: {}",
    //                         e
    //                       );
    //                       continue;
    //                     },
    //                   };

    //                   if let Err(e) =
    //                     transport.send_to(frame.as_ref(), peer_addr).await
    //                   {
    //                     error!("An error occurred when sending frame: {}", e);
    //                   }
    //                 },
    //                 Err(e) => error!("An error occurred when decoding: {}", e),
    //               }
    //             },
    //             Ok(None) => {
    //               warn!(
    //               "A `datagram worker` is stopping for worker channel has been closed"
    //               );
    //               break;
    //             },
    //             Err(_) => {warn!(
    //               "A `datagram worker` is stopping for worker channel has been closed"
    //             );
    //             break;
    //             },
    //           }
    //         }
    //       });
    //     }
    //   }

    //   #[cfg(windows)]
    //   {
    //     let transport_cloned = transport.clone();
    //     let shutdown_handle_cloned = shutdown_handle_child.child();
    //     let sender = sender.clone();

    //     // recv tasks
    //     recv_tasks.spawn(async move {
    //       let mut buf: [u8; 65535] = [0; 65535];

    //       loop {
    //         let res = tokio::select! {
    //           result = transport_cloned.recv_from(&mut buf) => result,
    //           _ = shutdown_handle_cloned.wait() => break
    //         };

    //         match res {
    //           Ok((bytes, peer_addr)) => {
    //             match sender
    //               .send((peer_addr, Frame::copy_from_slice(&buf[..bytes])))
    //               .await
    //             {
    //               Err(ChannelError::ChannelClosed) => {
    //                 warn!(
    //                   "A `datagram recv task` is stopping for worker channel has been closed"
    //                 );
    //                 break;
    //               },
    //               _ => continue,
    //             }
    //           },
    //           Err(e) => {
    //             error!("An error occurred when fetching datagram: {}", e)
    //           },
    //         }
    //       }
    //     });

    //     for _ in 0..num_cpus::get() {
    //       let transport_worker_cloned = transport.clone();
    //       let protocol_cloned = protocol.clone();
    //       let pipeline_cloned = pipeline.clone();
    //       let shutdown_handle_worker = shutdown_handle_child.child();
    //       let receiver = receiver.clone();

    //       worker_tasks.spawn(async move {
    //         let transport = transport_worker_cloned;
    //         let protocol = protocol_cloned;
    //         let pipeline = pipeline_cloned;
    //         let receiver = receiver;

    //         loop {
    //           let res = tokio::select! {
    //             result = receiver.recv() => result,
    //             _ = shutdown_handle_worker.wait() => break,
    //           };

    //           match res {
    //             Ok(Some((peer_addr, frame))) => {
    //               let decoded = protocol.decode((frame, peer_addr)).await;
    //               match decoded {
    //                 Ok(request) => {
    //                   let response = pipeline.handle(request).await;
    //                   let frame = match response {
    //                     Ok(response) => match protocol.encode(response).await {
    //                       Ok(frame) => frame,
    //                       Err(e) => {
    //                         error!("An error occurred when encoding: {}", e);
    //                         continue;
    //                       },
    //                     },
    //                     Err(e) => {
    //                       error!(
    //                         "An error occurred when middleware processing: {}",
    //                         e
    //                       );
    //                       continue;
    //                     },
    //                   };

    //                   if let Err(e) =
    //                     transport.send_to(frame.as_ref(), peer_addr).await
    //                   {
    //                     error!("An error occurred when sending frame: {}", e);
    //                   }
    //                 },
    //                 Err(e) => error!("An error occurred when decoding: {}", e),
    //               }
    //             },
    //             Ok(None) => {
    //               warn!(
    //               "A `datagram worker` is stopping for worker channel has been closed"
    //               );
    //               break;
    //             },
    //             Err(_) => {warn!(
    //               "A `datagram worker` is stopping for worker channel has been closed"
    //             );
    //             break;
    //             },
    //           }
    //         }
    //       });
    //     }
    //   }

    //   shutdown_handle_loop.wait().await;

    //   // Entering stopping logic
    //   let timeout =
    //     sleep(Duration::from_millis(shutdown_handle_child.timeout() as u64));

    //   tokio::select! {
    //     _ = worker_tasks.join_all() => {},
    //     _ = recv_tasks.join_all() => {},
    //     _ = timeout => {},
    //   }
    // });

    // Ok(Self { handle, shutdown_handle, name, id })

    todo!()
  }

  pub fn shutdown(&self) {
    self.shutdown_handle.shutdown();
  }
}

impl Pod {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn id(&self) -> u32 {
    self.id
  }

  pub fn abort(&self) {
    self.handle.abort();
  }
}
