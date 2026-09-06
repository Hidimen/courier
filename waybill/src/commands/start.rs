use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use chrono::Local;
use clap::{Args, ValueEnum};
use courier::depot::config::Config;
use event::{event, events::ServerStop};
use logger::Level;
use tokio_util::sync::CancellationToken;

/// Arguments for the `start` subcommand.
#[derive(Args)]
pub struct StartArgs {
  /// Path to the config file.
  #[arg(long, short)]
  config_file: Option<PathBuf>,
  /// Name of the provider to use.
  /// Overrides the `server.provider` value in the config file.
  #[arg(long, short)]
  provider: Option<String>,
  /// Lowest level of logger.
  /// Prints logs whose level is above `log_level`.
  #[arg(long)]
  log_level: Option<LogLevel>,
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

  let provider =
    args.provider.or_else(|| config.server.provider.clone()).context(
      "No provider has been specified: pass `--provider` or set \
       `server.provider` in the config file",
    )?;

  if let Some(log_level) = args.log_level {
    config.server.logger.level = log_level.into();
  }

  run(provider, config)
}

/// Runs the server until `cancellation_token` fires.
///
/// When `signals` is `true`, an OS termination signal (`Ctrl+C` /
/// `SIGTERM`) also cancels the token. It is disabled in service mode,
/// where shutdown is driven by the Service Control Manager instead.
fn run_blocking(
  provider: &str, config: &Config, cancellation_token: Arc<CancellationToken>,
  signals: bool,
) -> anyhow::Result<()> {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .name("courier:runtime")
    .thread_name("courier:worker")
    .build()
    .context("Unable to build Tokio runtime")?;

  runtime.block_on(async {
    if signals {
      let token = cancellation_token.clone();
      tokio::spawn(async move {
        courier::depot::wait_for_termination().await;
        event!(ServerStop {
          time: {
            let now = Local::now();
            now.timestamp_millis()
          }
        });
        token.cancel();
      });
    }

    builtin::run_instance(provider, config, cancellation_token).await
  })
}

#[cfg(not(windows))]
fn run(provider: String, config: Config) -> anyhow::Result<()> {
  run_blocking(&provider, &config, Arc::new(CancellationToken::new()), true)
}

#[cfg(windows)]
fn run(provider: String, config: Config) -> anyhow::Result<()> {
  windows::run(provider, config)
}

#[cfg(windows)]
mod windows {
  use std::{
    ffi::OsString,
    sync::{Arc, OnceLock},
    time::Duration,
  };

  use super::{Config, run_blocking};
  use anyhow::Context;
  use chrono::Local;
  use event::events::ServerStop;
  use tokio_util::sync::CancellationToken;
  use windows_service::{
    define_windows_service,
    service::{
      ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
      ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
  };

  /// State the service main needs. The service entry point is a plain `fn`,
  /// so it cannot capture locals: stash the values here before dispatching
  /// and read them back inside the service main.
  struct ServiceContext {
    provider: String,
    config: Config,
  }

  static SERVICE_CONTEXT: OnceLock<ServiceContext> = OnceLock::new();
  static SERVICE_TOKEN: OnceLock<Arc<CancellationToken>> = OnceLock::new();

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
    let cancellation_token =
      SERVICE_TOKEN.get().context("service token is not set")?.clone();

    // The Service Control Manager may send `Stop` before the server has
    // started, so shutdown is driven by cancelling the token directly
    // rather than by relying on the (possibly not yet installed) event bus.
    let stop_token = cancellation_token.clone();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
      match control_event {
        ServiceControl::Stop => {
          if let Some(bus) = event::Dispatcher::try_get_instance() {
            bus.emit(ServerStop {
              time: {
                let now = Local::now();
                now.timestamp_millis()
              },
            });
            stop_token.cancel();
          }
          ServiceControlHandlerResult::NoError
        },
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
      }
    };

    let status_handle =
      service_control_handler::register("courier", event_handler)?;

    let running = ServiceStatus {
      service_type: ServiceType::OWN_PROCESS,
      current_state: ServiceState::Running,
      controls_accepted: ServiceControlAccept::STOP,
      exit_code: ServiceExitCode::Win32(0),
      checkpoint: 0,
      wait_hint: Duration::from_millis(0),
      process_id: None,
    };
    status_handle.set_service_status(running.clone())?;

    let result = run_blocking(
      &context.provider,
      &context.config,
      cancellation_token,
      false,
    );

    // Tell the Service Control Manager the service stopped.
    let stopped =
      ServiceStatus { current_state: ServiceState::Stopped, ..running };
    let _ = status_handle.set_service_status(stopped);

    result
  }

  /// Runs on Windows: dispatches as a service when launched by the SCM,
  /// otherwise falls back to a foreground run.
  pub fn run(provider: String, config: Config) -> anyhow::Result<()> {
    let _ = SERVICE_CONTEXT.set(ServiceContext { provider, config });
    let cancellation_token = Arc::new(CancellationToken::new());
    let _ = SERVICE_TOKEN.set(cancellation_token.clone());

    match service_dispatcher::start("courier", ffi_service_main) {
      Ok(()) => Ok(()),
      Err(windows_service::Error::Winapi(error))
        if error.raw_os_error()
          == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT) =>
      {
        let context =
          SERVICE_CONTEXT.get().context("service context is not set")?;
        run_blocking(
          &context.provider,
          &context.config,
          cancellation_token,
          true,
        )
      },
      Err(error) => Err(error.into()),
    }
  }
}
