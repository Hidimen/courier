use std::{
  io::Result,
  net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

use crate::transport::{DatagramTransport, Transport};

pub struct UdpTransport(tokio::net::UdpSocket);

impl Transport for UdpTransport {
  fn bind<A: ToSocketAddrs + Send>(addr: A) -> Result<Self> {
    Ok(Self(tokio::net::UdpSocket::from_std(UdpSocket::bind(addr)?)?))
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
  async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
    self.0.recv(buf).await
  }

  async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
    self.0.recv_from(buf).await
  }

  async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<usize> {
    self.0.send_to(buf, addr).await
  }
}
