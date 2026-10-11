use std::path::PathBuf;

use anyhow::Context;
use chrono::Local;
use clap::{Args, ValueEnum};
use depot::config::Config;
use event::events::ServerStop;
use logger::{DefaultFormatter, Level, PrettyFormatter, flows::ConsoleFlow};

/// Arguments for the `start` subcommand.
#[derive(Args)]
pub struct StartArgs {
  /// Path to the config file.
  #[arg(long, short)]
  config_file: Option<PathBuf>,
  /// Name of the provider to use.
  /// Overrides the `provider` value in the config file and is the default
  /// for every pod that does not name its own.
  #[arg(long, short)]
  provider: Option<String>,
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

/// Starts a server instance.
///
/// On Windows the process first tries to run as a Windows service; when it
/// was launched directly from a terminal (not by the Service Control Manager)
/// the service dispatch fails and the server falls back to a foreground run.
/// Other platforms always run in the foreground.
pub fn handle(args: StartArgs) -> anyhow::Result<()> {
  let mut config = match &args.config_file {
    Some(path) => {
      let content =
        std::fs::read(path).context("Unable to read config file")?;
      marquage::from_slice(&content).context("Unable to parse config file")?
    },
    None => Config::default(),
  };

  if config.pods.is_empty() {
    eprintln!("No pods specified");
    return Ok(());
  }

  if let Some(log_level) = args.log_level {
    config.logger.level = log_level.into();
  }

  if let Some(shutdown_timeout) = args.shutdown_timeout {
    config.shutdown_timeout = shutdown_timeout;
  }

  // `--provider` overrides the config file's top-level `provider`, and the
  // resulting value is the fallback for every pod that names none of its
  // own. A provider is not needed to run the instance itself — the pods
  // resolve theirs — so it is only required for pods that would end up
  // without one.
  let provider = args.provider.or_else(|| config.provider.clone());
  for pod in &mut config.pods {
    if pod.provider.is_none() {
      pod.provider = provider.clone();
    }
  }

  if let Some(pod) = config.pods.iter().find(|pod| pod.provider.is_none()) {
    anyhow::bail!(
      "No provider has been specified for the listener `{}`: pass \
       `--provider`, set `provider` in the pod, or set the top-level \
       `provider` in the config file",
      pod.name.as_deref().unwrap_or("Unnamed")
    );
  }

  run(config)
}

/// Runs the server until the depot's root
/// [`ShutdownHandle`](depot::ShutdownHandle) is cancelled.
///
/// The global singletons come first: the logger is built from
/// `config.logger` and the event bus from `config.event`, because the pods
/// log through the plain logging macros and announce their shutdown
/// through the bus, so both must exist before the first pod is spawned.
/// `builtin::run` then turns every configured listener into a pod, and
/// [`Depot::start`](depot::Depot::start) waits for the root token
/// before joining them all, so the instance is drained before the
/// singletons are flushed below.
///
/// When `signals` is `true`, an OS termination signal (`Ctrl+C` /
/// `SIGTERM`) cancels the root token. It is disabled in service mode,
/// where shutdown is driven by the Service Control Manager instead.
///
/// Once the shutdown has started every further signal is ignored: keeping
/// the handlers installed swallows them, so they can neither interrupt
/// the drain nor kill the process through their default behaviour. The
/// only thing that bounds the shutdown is `shutdown_timeout`, after which
/// the remaining tasks are aborted.
///
/// Before returning, the event bus and the logger are flushed — both are
/// global singletons that are never dropped, so without this the events
/// and the log records emitted while shutting down would be lost. They
/// are flushed on every path, including a startup that failed.
fn run_blocking(config: &Config, signals: bool) -> anyhow::Result<()> {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .name("courier:runtime")
    .thread_name("courier:worker")
    .build()
    .context("Unable to build Tokio runtime")?;

  runtime.block_on(async {
    let result = run_instance(config, signals).await;

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

/// Builds the instance described by `config` and drives it until every pod
/// has drained.
async fn run_instance(config: &Config, signals: bool) -> anyhow::Result<()> {
  // Both singletons are installed before the first pod is spawned: the
  // pods log through the plain logging macros and emit through the bus.
  install_logger(&config.logger)?;
  event::Dispatcher::new(config.event.capacity).install();

  // `builtin::run` resolves the provider of every pod, binds its listener
  // and spawns its task; a listener that cannot be bound fails the whole
  // startup, which the caller reports by exiting.
  let depot = builtin::run(config).await?;

  let depot = depot.install();

  // The depot's root token: cancelling it stops every pod at once, which
  // is exactly what every shutdown path below does.
  let shutdown_handle = depot.shutdown_handle();

  for pod in &config.pods {
    for addr in &pod.listen {
      logger::info!("Listening on {}", addr);
    }
  }

  // On Windows the Service Control Manager may send `Stop` before the
  // depot has finished starting. The control handler records that intent
  // in `SERVICE_STOPPING` and reads the handle published here, so act on
  // it as soon as the handle exists.
  #[cfg(windows)]
  {
    let _ = windows::SERVICE_SHUTDOWN.set(shutdown_handle.clone());
    if windows::SERVICE_STOPPING.load(std::sync::atomic::Ordering::SeqCst) {
      shutdown_handle.shutdown();
    }
  }

  if signals {
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
  }

  // Wait for the root token and join every pod, so that nothing is still
  // serving once the caller flushes the global singletons.
  depot.start().await;

  Ok(())
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

#[cfg(not(windows))]
fn run(config: Config) -> anyhow::Result<()> {
  run_blocking(&config, true)
}

#[cfg(windows)]
fn run(config: Config) -> anyhow::Result<()> {
  windows::run(config)
}

#[cfg(windows)]
mod windows {
  use std::{
    ffi::OsString,
    sync::{
      OnceLock,
      atomic::{AtomicBool, Ordering},
    },
    time::Duration,
  };

  use super::{Config, run_blocking};
  use anyhow::Context;
  use chrono::Local;
  use depot::ShutdownHandle;
  use event::events::ServerStop;
  use windows_service::{
    define_windows_service,
    service::{
      ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
      ServiceStatus, ServiceType,
    },
    service_control_handler::{
      self, ServiceControlHandlerResult, ServiceStatusHandle,
    },
    service_dispatcher,
  };

  /// State the service main needs. The service entry point is a plain `fn`,
  /// so it cannot capture locals: stash the value here before dispatching
  /// and read it back inside the service main.
  struct ServiceContext {
    config: Config,
  }

  static SERVICE_CONTEXT: OnceLock<ServiceContext> = OnceLock::new();
  /// The depot's `ShutdownHandle`, published by `run_blocking` once the
  /// depot has been built. The control handler reads it back to drive the
  /// graceful shutdown, since the handle only exists after `builtin::run`.
  pub(crate) static SERVICE_SHUTDOWN: OnceLock<ShutdownHandle> =
    OnceLock::new();
  /// Handle used to report the service status from the control handler,
  /// which is registered before the handle exists.
  static SERVICE_STATUS: OnceLock<ServiceStatusHandle> = OnceLock::new();
  /// Set once the Service Control Manager has asked the service to stop:
  /// any further request is ignored while the shutdown is in progress.
  pub(crate) static SERVICE_STOPPING: AtomicBool = AtomicBool::new(false);

  /// Error returned by `StartServiceCtrlDispatcherW` when the process was
  /// not started by the Service Control Manager.
  const ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: i32 = 1063;

  define_windows_service!(ffi_service_main, service_main);

  fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
      eprintln!("courier service failed: {error}");
    }
  }

  fn run_service() -> anyhow::Result<()> {
    let context =
      SERVICE_CONTEXT.get().context("service context is not set")?;

    // The Service Control Manager may send `Stop` before the depot has been
    // built, so the intent is recorded in `SERVICE_STOPPING` and the handle
    // to actually trigger the shutdown is read from `SERVICE_SHUTDOWN` once
    // `run_blocking` publishes it.
    // How long the shutdown may take, plus a margin for flushing the
    // event bus and the logger.
    // let wait_hint =
    //   Duration::from_millis(context.config.server.shutdown_timeout)
    //     + Duration::from_secs(1);
    // Fields shared by every status the service reports.
    let running_status = ServiceStatus {
      service_type: ServiceType::OWN_PROCESS,
      current_state: ServiceState::Running,
      controls_accepted: ServiceControlAccept::STOP,
      exit_code: ServiceExitCode::Win32(0),
      checkpoint: 0,
      wait_hint: Duration::from_millis(0),
      process_id: None,
    };

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
      match control_event {
        ServiceControl::Stop => {
          // The shutdown is already underway: only the grace period
          // bounds it, so further requests are ignored.
          if SERVICE_STOPPING.swap(true, Ordering::SeqCst) {
            return ServiceControlHandlerResult::NoError;
          }

          if let Some(bus) = event::Dispatcher::try_get_instance() {
            bus.emit(ServerStop {
              time: {
                let now = Local::now();
                now.timestamp_millis()
              },
            });
          }
          if let Some(shutdown_handle) = SERVICE_SHUTDOWN.get() {
            shutdown_handle.shutdown();
          }

          // Without this the SCM assumes the service is not responding
          // and reports a failure to stop it.
          if let Some(status_handle) = SERVICE_STATUS.get() {
            let _ = status_handle.set_service_status(ServiceStatus {
              current_state: ServiceState::StopPending,
              controls_accepted: ServiceControlAccept::empty(),
              wait_hint: Duration::from_secs(0),
              checkpoint: 0,
              ..running_status
            });
          }

          ServiceControlHandlerResult::NoError
        },
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
      }
    };

    let status_handle =
      service_control_handler::register("courier", event_handler)?;
    let _ = SERVICE_STATUS.set(status_handle);

    status_handle.set_service_status(running_status.clone())?;

    let result = run_blocking(&context.config, false);

    // Tell the Service Control Manager the service stopped, reporting a
    // non-zero exit code when the run failed.
    let stopped = ServiceStatus {
      current_state: ServiceState::Stopped,
      controls_accepted: ServiceControlAccept::empty(),
      exit_code: match &result {
        Ok(()) => ServiceExitCode::Win32(0),
        Err(_) => ServiceExitCode::Win32(1),
      },
      checkpoint: 0,
      wait_hint: Duration::from_millis(0),
      ..running_status
    };
    let _ = status_handle.set_service_status(stopped);

    result
  }

  /// Runs on Windows: dispatches as a service when launched by the SCM,
  /// otherwise falls back to a foreground run.
  pub fn run(config: Config) -> anyhow::Result<()> {
    let _ = SERVICE_CONTEXT.set(ServiceContext { config });

    match service_dispatcher::start("courier", ffi_service_main) {
      Ok(()) => Ok(()),
      Err(windows_service::Error::Winapi(error))
        if error.raw_os_error()
          == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT) =>
      {
        let context =
          SERVICE_CONTEXT.get().context("service context is not set")?;
        run_blocking(&context.config, true)
      },
      Err(error) => Err(error.into()),
    }
  }
}
