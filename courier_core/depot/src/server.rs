use std::sync::Arc;

use network::{
  stream::{SplitStream, WriteHalf},
  transport::{DatagramTransport, StreamTransport, Transport},
};
use protocol::{DatagramProtocol, Protocol, StreamProtocol};

use crate::error::{DepotError, ProtocolErrorWrapper};

pub struct Depot<T, P>
where
  T: Transport,
  P: Protocol,
{
  transport: T,
  protocol: Arc<P>,
}

impl<T, P> Depot<T, P>
where
  T: Transport,
  P: Protocol,
{
  pub fn new(transport: T, protocol: P) -> Self {
    Self { transport, protocol: Arc::new(protocol) }
  }
}

impl<T, P> Depot<T, P>
where
  T: StreamTransport,
  P: StreamProtocol<T>,
{
  pub async fn run_stream(&mut self) -> Result<(), DepotError<P::Error>> {
    loop {
      let (read, mut write) = self.transport.accept().await?.split();
      let protocol_cloned = self.protocol.clone();
      let data: Result<network::Frame, DepotError<P::Error>> = match tokio::spawn(async move {
        let c = protocol_cloned.decode(read).await.map_err(ProtocolErrorWrapper)?;
        let d = protocol_cloned.encode(c).await;
        Ok(d)
      })
      .await
      {
        Ok(d) => d,
        Err(e) => return Err(DepotError::TaskTerminated(e)),
      };
      write.write_all(data?.as_ref()).await?;
      write.flush().await?
    }
  }
}

impl<T, P> Depot<T, P>
where
  T: DatagramTransport,
  P: DatagramProtocol<T>,
{
  pub async fn run_datagram(&mut self) -> Result<(), DepotError<P::Error>> {
    loop {
      let mut buf = [0u8; 1024];
      let (_, peer) = self.transport.recv_from(&mut buf).await?;

      let protocol_cloned = self.protocol.clone();
      let data: Result<network::Frame, DepotError<P::Error>> = match tokio::spawn(async move {
        let c = protocol_cloned.decode(&buf).await.map_err(ProtocolErrorWrapper)?;
        let d = protocol_cloned.encode(c).await;
        Ok(d)
      })
      .await
      {
        Ok(d) => d,
        Err(e) => return Err(DepotError::TaskTerminated(e)),
      };

      self.transport.send_to(data?.as_ref(), peer).await?;
    }
  }
}
