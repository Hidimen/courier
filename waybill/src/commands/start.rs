use std::{net::SocketAddr, path::PathBuf};

use anyhow::Context;
use chrono::Local;
use clap::{Args, ValueEnum};
use depot::{Config, Depot, config::RunningKind};
use event::events::ServerStop;
use extension::builtin::Module;
use logger::{
  DefaultFormatter, Level, PrettyFormatter, flows::ConsoleFlow, info,
};

/// Arguments for the `start` subcommand.
#[derive(Args)]
pub struct StartArgs {
  /// Path to the config file.
  #[arg(long, short)]
  config_file: Option<PathBuf>,
  /// Name of the provider to use.
  #[arg(long, short)]
  provider: Option<String>,
  /// Running kind.
  #[arg(long)]
  kind: Option<Kind>,
  /// Lowest level of logger.
  /// Prints logs whose level is above `log_level`.
  #[arg(long)]
  log_level: Option<LogLevel>,
  /// Maximal processing milliseconds for shutdown.
  #[arg(long)]
  shutdown_timeout: Option<usize>,
}

#[derive(ValueEnum, Clone, Copy)]
enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
  Fatal,
}

impl From<LogLevel> for Level {
  fn from(value: LogLevel) -> Self {
    match value {
      LogLevel::Trace => Self::Trace,
      LogLevel::Debug => Self::Debug,
      LogLevel::Info => Self::Info,
      LogLevel::Warn => Self::Warn,
      LogLevel::Error => Self::Error,
      LogLevel::Fatal => Self::Fatal,
    }
  }
}

#[derive(ValueEnum, Clone, Copy)]
enum Kind {
  Stream,
  Datagram,
}

impl From<Kind> for RunningKind {
  fn from(value: Kind) -> Self {
    match value {
      Kind::Stream => RunningKind::Stream,
      Kind::Datagram => RunningKind::Datagram,
    }
  }
}

/// Starts a server instance and blocks until it shuts down.
pub fn handle(args: StartArgs) -> anyhow::Result<()> {
  let mut config = match &args.config_file {
    Some(path) => {
      let content =
        std::fs::read(path).context("Unable to read config file")?;
      marquage::from_slice(&content).context("Unable to parse config file")?
    },
    None => Config::default(),
  };

  if let Some(log_level) = args.log_level {
    config.logger.level = log_level.into();
  }

  if let Some(shutdown_timeout) = args.shutdown_timeout {
    config.shutdown_timeout = shutdown_timeout;
  }

  if let Some(kind) = args.kind {
    config.kind = kind.into();
  }

  config.provider = args.provider;

  run(config)
}

/// Runs the server until the depot's root
/// [`ShutdownHandle`](depot::ShutdownHandle) is cancelled.
///
/// The global singletons come first: the logger is built from
/// `config.logger` and the event bus from `config.event`, because the
/// capabilities log through the plain logging macros and announce their
/// shutdown through the bus, so both must exist before the first one is
/// spawned.
/// The selected builtin is then looked up in the
/// [`Registry`](crate::builtin::Registry), and every
/// configured listener is bound once per capability it declares, after
/// which [`Depot::wait`](depot::Depot::wait) joins them all, so the
/// instance is drained before the singletons are flushed below.
///
/// A termination signal (`Ctrl+C` / `SIGTERM`) cancels the root token. Once
/// the shutdown has started every further signal is ignored: keeping the
/// handlers installed swallows them, so they can neither interrupt the drain
/// nor kill the process through their default behaviour. The only thing that
/// bounds the shutdown is `shutdown_timeout`, after which the remaining tasks
/// are aborted.
///
/// Before returning, the event bus and the logger are flushed — both are
/// global singletons that are never dropped, so without this the events
/// and the log records emitted while shutting down would be lost. They
/// are flushed on every path, including a startup that failed.
fn run(config: Config) -> anyhow::Result<()> {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .name("courier:runtime")
    .thread_name("courier:worker")
    .build()
    .context("Unable to build Tokio runtime")?;

  runtime.block_on(async {
    let result = run_instance(&config).await;

    // The event bus comes first: its listeners may still log, and those
    // records are only written once the logger is flushed below.
    if let Some(bus) = event::Dispatcher::try_get_instance() {
      bus.shutdown().await;
    }
    if let Some(logger) = logger::Logger::try_get_instance() {
      logger.shutdown();
    }

    result
  })
}

/// Builds the instance described by `config` and drives it until every
/// capability has drained.
async fn run_instance(config: &Config) -> anyhow::Result<()> {
  // Both singletons are installed before the first capability is spawned:
  // the capabilities log through the plain logging macros and emit through
  // the bus.
  install_logger(&config.logger)?;
  event::Dispatcher::new(config.event.capacity).install();

  // The registry resolves the selected builtin, binds one listener per
  // capability and spawns its task; a listener that cannot be bound fails
  // the whole startup, which the caller reports by exiting.
  let registry = crate::builtin::registry();
  let name = config.provider.as_deref().context("No builtin selected")?;
  let module =
    registry.get(name).with_context(|| format!("Unknown builtin `{name}`"))?;

  let depot = build_depot(module, config)?;

  // The depot's root token: cancelling it stops every task at once, which
  // is exactly what every shutdown path below does.
  let shutdown_handle = depot.shutdown_handle();

  let shutdown = shutdown_handle.clone();
  tokio::spawn(async move {
    depot::wait_for_termination().await;
    if let Some(bus) = event::Dispatcher::try_get_instance() {
      bus.emit(ServerStop {
        time: {
          let now = Local::now();
          now.timestamp_millis()
        },
      });
    }
    shutdown.shutdown();

    // The shutdown is underway: consume every subsequent signal
    // without acting on it, so it neither interrupts the drain nor
    // reaches the default OS handler.
    loop {
      depot::wait_for_termination().await;
    }
  });

  let metrics = depot.metrics();
  info!(
    "depot",
    "Server runs on `{}` protocol provided by `{}` builtin module, based on `{}` transport",
    metrics.protocol,
    module.name(),
    metrics.transport
  );

  depot.wait().await;

  Ok(())
}

/// Builds a depot that serves every capability of `module`.
///
/// Each capability is served on every address of `config.listen`: a stream
/// capability gets one transport listener per address and a datagram
/// capability one socket per address.
///
/// # Errors
///
/// Returns an error if a configured address cannot be parsed or if a
/// capability cannot bind it.
fn build_depot(
  module: &Module<Depot>, config: &Config,
) -> anyhow::Result<Depot> {
  let mut depot = Depot::new(config.shutdown_timeout);

  match config.kind {
    RunningKind::Stream => {
      if let Some(pvd) = &module.stream_provider {
        for addr in &config.listen {
          let parsed = addr
            .parse::<SocketAddr>()
            .with_context(|| format!("Invalid socket address {}", addr))?;
          (pvd.install)(&mut depot, parsed)?;
        }
      } else {
        anyhow::bail!(
          "Builtin module `{}` do not register stream provider",
          module.name()
        );
      }
    },
    RunningKind::Datagram => {
      if let Some(pvd) = &module.datagram_provider {
        for addr in &config.listen {
          let parsed = addr
            .parse::<SocketAddr>()
            .with_context(|| format!("Invalid socket address {}", addr))?;
          (pvd.install)(&mut depot, parsed)?;
        }
      } else {
        anyhow::bail!(
          "Builtin module `{}` do not register datagram provider",
          module.name()
        );
      }
    },
  }

  Ok(depot)
}

/// Builds the process logger from `config` and installs it as the global
/// singleton the logging macros use.
fn install_logger(config: &logger::Config) -> anyhow::Result<()> {
  // The formatter is a type parameter of the builder, so the two
  // alternatives are two instantiations of the same chain.
  match config.format {
    logger::FormatConfig::Default => install_with(config, DefaultFormatter),
    logger::FormatConfig::Pretty => install_with(config, PrettyFormatter),
  }
}

/// Installs the global logger, rendering every record with `formatter`.
fn install_with<F: logger::Format>(
  config: &logger::Config, formatter: F,
) -> anyhow::Result<()> {
  logger::Builder::new()
    .capacity(config.capacity)
    .format(formatter)
    .add_flow(ConsoleFlow::new(config.level))
    .build()
    .map_err(|error| anyhow::anyhow!("Unable to create a logger: {error}"))?;

  Ok(())
}
