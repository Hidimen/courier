use event::Dispatcher;
use logger::{
  DefaultFormatter, FormatConfig, PrettyFormatter, flows::ConsoleFlow,
};
use network::transport::{DatagramTransport, StreamTransport};
use protocol::{DatagramProtocol, StreamProtocol};

use crate::{
  Builder, DatagramProvider, RunningHandle, ShutdownHandle, StreamProvider,
  config::Config,
};

pub async fn run_stream<T, P, Provider>(
  config: &Config, provider: Provider,
) -> anyhow::Result<(ShutdownHandle, RunningHandle)>
where
  T: StreamTransport,
  P: StreamProtocol<T>,
  Provider: StreamProvider<Transport = T, Protocol = P>,
{
  let transport = T::bind(&config.address)?;
  let protocol = provider.protocol();
  let pipeline = provider.pipeline();
  let logger = match config.logger.format {
    FormatConfig::Default => logger::Builder::new()
      .capacity(config.logger.capacity)
      .format(DefaultFormatter)
      .add_flow(ConsoleFlow::new(config.logger.level))
      .build()
      .expect("Unable to create a logger"),
    FormatConfig::Pretty => logger::Builder::new()
      .capacity(config.logger.capacity)
      .format(PrettyFormatter)
      .add_flow(ConsoleFlow::new(config.logger.level))
      .build()
      .expect("Unable to create a logger"),
  };
  let dispatcher = Dispatcher::new(config.event.capacity).install();

  logger::info!("Listening on {}", config.address);

  let mut depot = Builder::new()
    .transport(transport)
    .protocol(protocol)
    .pipeline(pipeline)
    .logger(logger)
    .dispatcher(dispatcher)
    .build()?;

  let shutdown_handle = depot.shutdown_handle();

  let handle = tokio::spawn(async move {
    depot.run_stream().await;
  });

  Ok((shutdown_handle, RunningHandle(handle)))
}

pub async fn run_datagram<T, P, Provider>(
  config: &Config, provider: Provider,
) -> anyhow::Result<(ShutdownHandle, RunningHandle)>
where
  T: DatagramTransport,
  P: DatagramProtocol<T>,
  Provider: DatagramProvider<Transport = T, Protocol = P>,
{
  let transport = T::bind(&config.address)?;
  let protocol = provider.protocol();
  let pipeline = provider.pipeline();
  let logger = match config.logger.format {
    FormatConfig::Default => logger::Builder::new()
      .capacity(config.logger.capacity)
      .format(DefaultFormatter)
      .add_flow(ConsoleFlow::new(config.logger.level))
      .build()
      .expect("Unable to create a logger"),
    FormatConfig::Pretty => logger::Builder::new()
      .capacity(config.logger.capacity)
      .format(PrettyFormatter)
      .add_flow(ConsoleFlow::new(config.logger.level))
      .build()
      .expect("Unable to create a logger"),
  };
  let dispatcher = Dispatcher::new(config.event.capacity).install();

  logger::info!("Listening on {}", config.address);

  let mut depot = Builder::new()
    .transport(transport)
    .protocol(protocol)
    .pipeline(pipeline)
    .logger(logger)
    .dispatcher(dispatcher)
    .build()?;

  let shutdown_handle = depot.shutdown_handle();

  let handle = tokio::spawn(async move {
    depot.run_datagram().await;
  });

  Ok((shutdown_handle, RunningHandle(handle)))
}
