use depot::DepotBuilder;
use logger::info;
use network::{
  Frame, KeepAlive,
  stream::ReadHalf,
  transport::{Transport, tcp::TcpTransport},
};
use protocol::{Protocol, StreamProtocol};
use relay::PipelineBuilder;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EchoError {
  #[error("IO Error")]
  IOError(#[from] std::io::Error),
}

pub struct EchoProtocol;
pub struct Request(pub String);
pub struct Response(pub String);

impl Protocol for EchoProtocol {
  type Error = EchoError;
  type Request = Request;
  type Response = Response;

  fn name() -> &'static str {
    "protocol:echo"
  }

  fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
  }
}

impl StreamProtocol<TcpTransport> for EchoProtocol {
  async fn decode(
    &self,
    read_half: &mut <TcpTransport as network::transport::StreamTransport>::ReadHalf,
  ) -> Result<Self::Request, Self::Error> {
    let mut buf = [0u8; 1024];
    let _ = read_half.read(&mut buf).await?;

    Ok(Request(String::from_utf8_lossy(&buf).to_string()))
  }

  async fn encode(
    &self, response: Self::Response,
  ) -> Result<(network::Frame, network::KeepAlive), Self::Error> {
    Ok((Frame::from_owner(response.0), KeepAlive::Keep))
  }
}

pub async fn boot() {
  let pipeline = PipelineBuilder::default()
    .then(|request: Request| Response(request.0))
    .build();

  let transport = TcpTransport::bind("0.0.0.0:8080").unwrap();
  let protocol = EchoProtocol;
  let logger = logger::Logger::default().install();

  info!("main", "Server is running on port 8080.");

  let mut server = DepotBuilder::default()
    .logger(logger)
    .pipeline(pipeline)
    .transport(transport)
    .protocol(protocol)
    .build()
    .unwrap();
  let _ = server.run_stream().await;
}
