use std::{
  io::Result,
  net::{SocketAddr, UdpSocket},
};

use crate::transport::{DatagramTransport, Transport};

pub struct UdpTransport(tokio::net::UdpSocket);

impl Transport for UdpTransport {
  fn bind(addr: SocketAddr) -> Result<Self> {
    /*
      To avoid Tokio panic, set_nonblocking is necessary.
    */
    let sock = UdpSocket::bind(addr)?;
    sock.set_nonblocking(true)?;
    Ok(Self(tokio::net::UdpSocket::from_std(sock)?))
  }

  #[cfg(not(windows))]
  fn bind_reuse(addr: SocketAddr) -> Result<Self> {
    use socket2::{Domain, Protocol, Socket, Type};

    let socket =
      Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;

    let std_socket: std::net::UdpSocket = socket.into();

    Ok(Self(tokio::net::UdpSocket::from_std(std_socket)?))
  }

  fn set_ttl(&self, ttl: u32) -> Result<()> {
    self.0.set_ttl(ttl)
  }

  fn ttl(&self) -> Result<u32> {
    self.0.ttl()
  }

  fn local_addr(&self) -> Result<SocketAddr> {
    self.0.local_addr()
  }
}

impl DatagramTransport for UdpTransport {
  async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
    self.0.recv_from(buf).await
  }

  async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
    self.0.send_to(buf, addr).await
  }
}
