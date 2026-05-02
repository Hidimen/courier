use std::{io::Result, sync::Arc};

use network::{
  Protocol,
  stream::{ReadHalf, SplitStream, WriteHalf},
  transport::{StreamTransport, Transport},
};

pub struct Depot<T, P>
where
  T: Transport,
  P: Protocol<T>,
{
  transport: T,
  protocol: Arc<P>,
}

impl<T, P> Depot<T, P>
where
  T: Transport,
  P: Protocol<T>,
{
  pub fn new(transport: T, protocol: P) -> Self {
    Self { transport, protocol: Arc::new(protocol) }
  }
}

impl<T, P> Depot<T, P>
where
  T: StreamTransport,
  P: Protocol<T>,
  <T as network::transport::Transport>::Data: network::stream::ReadHalf,
{
  pub async fn start(&mut self) -> Result<()> {
    loop {
      let (mut read, mut write) = self.transport.accept().await?.split();
      // let protocol_cloned = self.protocol.clone();
      tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let exact = read.read(&mut buf).await?;

        println!("{:?}", buf);

        let _ = write.write_all(&buf[..exact]).await?;

        write.flush().await?;

        std::io::Result::Ok(())
      });
    }
  }
}
