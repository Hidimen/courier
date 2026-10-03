use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use logger::{error, info};
use network::{
  KeepAlive,
  stream::{Stream, WriteHalf},
  transport::{DatagramTransport, StreamTransport},
};
use protocol::{DatagramProtocol, Decoded, StreamProtocol};
use sync::activity_tracker::ActivityTracker;
use tokio::{task::JoinHandle, time::sleep};

use crate::{DatagramProvider, ShutdownHandle, StreamProvider};

pub struct Pod {
  pub handle: JoinHandle<()>,
  pub shutdown_handle: ShutdownHandle,
  pub name: String,
  pub id: usize,
}

impl Pod {
  pub fn spawn_stream<T, P, Pvd>(
    provider: Pvd, addr: SocketAddr, shutdown_handle: ShutdownHandle,
    name: String, id: usize,
  ) -> io::Result<Self>
  where
    T: StreamTransport,
    P: StreamProtocol<T>,
    Pvd: StreamProvider<Transport = T, Protocol = P>,
  {
    let transport = T::bind(addr)?;
    let protocol = provider.protocol();
    let pipeline = Arc::new(provider.pipeline());
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
            let protocol_cloned = protocol.clone();
            let pipeline_cloned = pipeline.clone();
            let child = shutdown_handle_child.child();
            let connection_count_cloned = connection_count.clone();

            tokio::spawn(async move {
              let protocol = protocol_cloned;
              let pipeline = pipeline_cloned;
              let (mut read, mut write) = stream.split();
              let mut request_count = 0_usize;
              let mut keep_alive = KeepAlive::Pending;
              let connection_count = connection_count_cloned;

              connection_count.acquire();

              loop {
                let decoded = match keep_alive {
                  KeepAlive::Timeout(d) => {
                    let res = tokio::select! {
                      res = tokio::time::timeout(d, protocol.decode(&mut read)) => res,
                      _ = child.wait() => break
                    };

                    match res {
                      Ok(Ok(decoded)) => decoded,
                      Ok(Err(e)) => {
                        error!("An error occurred when decoding: {}", e);
                        break;
                      },
                      Err(_) => break,
                    }
                  },
                  _ => {
                    let res = tokio::select! {
                      res =  protocol.decode(&mut read) => res,
                      _ = child.wait() => break
                    };

                    match res {
                      Ok(decoded) => decoded,
                      Err(e) => {
                        error!("An error occurred when decoding: {}", e);
                        break;
                      },
                    }
                  },
                };

                match decoded {
                  Decoded::Open(req) => {
                    let res = match pipeline.handle(req).await {
                      Ok(res) => res,
                      Err(e) => {
                        error!(
                          "An error occurred when middleware processed: {}",
                          e
                        );
                        break;
                      },
                    };
                    let (frame, ka) = match protocol.encode(res).await {
                      Ok(data) => data,
                      Err(e) => {
                        error!("An error occurred when encoding: {}", e);
                        break;
                      },
                    };

                    keep_alive = ka;
                    request_count += 1;
                    if let Err(e) = write.write(frame.as_ref()).await {
                      error!("An error occurred when sending frame: {}", e);
                      break;
                    }
                  },
                  Decoded::Closed => break,
                }

                match keep_alive {
                  KeepAlive::Close => break,
                  KeepAlive::UpTo(n) if request_count >= n => break,
                  _ => continue,
                }
              }

              connection_count.release();
            });
          },
          Err(e) => {
            error!("An error occurred when accepting a connection: {}", e)
          },
        }
      }

      // Entering stopping logic
      let timeout =
        sleep(Duration::from_millis(shutdown_handle_child.timeout() as u64));

      tokio::select! {
        _ = connection_count.wait() => {
          info!("Pod stopped");
        },
        _ = timeout => {},
      }
    });

    Ok(Self { handle, shutdown_handle, name, id })
  }

  pub fn spawn_datagram<T, P, Pvd>(
    _provider: Pvd, _addr: SocketAddr, _shutdown_handle: ShutdownHandle,
    _name: String, _id: usize,
  ) -> io::Result<Self>
  where
    T: DatagramTransport,
    P: DatagramProtocol<T>,
    Pvd: DatagramProvider<Transport = T, Protocol = P>,
  {
    // let transport = T::bind(addr)?;
    // let protocol = provider.protocol();
    // let pipeline = provider.pipeline();
    // let shutdown_handle_child = shutdown_handle.child();

    // let handle = tokio::spawn(async move {
    //   let mut buf: [u8; 65535] = [0; 65535];
    //   loop {
    //     let res = tokio::select! {
    //       result = transport.recv_from(&mut buf) => result,
    //       _ = shutdown_handle_child.wait() => break
    //     };

    //     match res {
    //       Ok()
    //     }
    //   }
    // });

    todo!()
  }

  pub async fn shutdown(self) {
    self.shutdown_handle.shutdown();
  }
}

impl Pod {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn id(&self) -> usize {
    self.id
  }
}
