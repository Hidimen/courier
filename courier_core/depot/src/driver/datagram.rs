//! Event loop for one datagram socket.
//!
//! A datagram socket is not tied to a single peer: every datagram carries its
//! own address, so one driver serves all peers of the socket. The wait races
//! three independent sources — the socket, the protocol's own work and the
//! per-socket timers — plus the shutdown signal, and each branch is gated on
//! its own condition so a paused read cannot hide the other branches.

use std::{
  io::{self, ErrorKind},
  net::SocketAddr,
  pin::Pin,
  task::{Poll, Waker},
  time::{Duration, Instant},
};

use bytes::BytesMut;
use logger::error;
use network::transport::DatagramTransport;
use protocol::{
  Payload, TimerId,
  datagram::{
    Context, DatagramProtocol, Input, Output,
    close::{self, CloseMode},
    effect::TimerOp,
  },
};
use sync::handle::ShutdownHandle;
use tokio::time::{Instant as TokioInstant, Sleep, sleep_until};

use crate::timer::TimerSet;

/// Drives one datagram socket until the protocol closes it or a shutdown starts.
///
/// `Context::peer` reports the sender of the datagram being stepped, and falls
/// back to the socket's local address for the events that have no sender
/// (`Opened`, a timer fired before any datagram arrived).
///
/// **Note**: the driver is crate-internal; a builtin reaches it through
/// [`Depot::run_datagram`](crate::Depot::run_datagram).
pub async fn drive_datagram<P: DatagramProtocol, T: DatagramTransport>(
  protocol: P, shutdown_handle: ShutdownHandle, transport: T,
) {
  let local = match transport.local_addr() {
    Ok(local) => local,
    Err(e) => {
      error!("Unable to get local address: {}", e);
      return;
    },
  };

  DatagramDriver::new(protocol, local, transport).run(&shutdown_handle).await;
}

/// Whether the driver should keep running or leave the loop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
  Continue,
  Stop,
}

/// The one thing that woke the driver for this iteration.
enum Wake<E> {
  Message {
    peer: SocketAddr,
    len: usize,
  },
  Failed(io::Error),
  Timer(TimerId),
  /// The protocol finished its outstanding work; these are the datagrams (if
  /// any) it produced while being polled.
  Protocol(Result<(), E>),
  Shutdown,
}

fn is_retryable(e: &io::Error) -> bool {
  matches!(
    e.kind(),
    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
  )
}

/// The protocol instance together with the output buffer it writes into.
struct Proto<P: DatagramProtocol> {
  /// What `Context::peer` reports: the sender of the most recent datagram, or
  /// the local endpoint before the first one arrives.
  peer: SocketAddr,
  state: P,
  output: Output,
}

impl<P: DatagramProtocol> Proto<P> {
  fn new(state: P, local: SocketAddr) -> Self {
    Self { peer: local, state, output: Output::new() }
  }

  fn step(&mut self, input: Input<P::Command>) -> Result<(), P::Error> {
    let mut context = Context::new(self.peer, &mut self.output);
    self.state.step(&mut context, input)
  }

  /// Polls the protocol's own work. `Pending` means a waker has been
  /// registered, `Ready` means the protocol has nothing outstanding.
  fn poll(&mut self, waker: &Waker) -> Poll<Result<(), P::Error>> {
    let mut context = Context::new(self.peer, &mut self.output);
    self.state.poll(&mut context, waker)
  }
}

/// Drives one datagram socket: waits for whichever source becomes ready, feeds
/// the event to the protocol and sends the datagrams it produced.
struct DatagramDriver<P: DatagramProtocol, T: DatagramTransport> {
  transport: T,
  proto: Proto<P>,
  buf: BytesMut,
  pending: Vec<(SocketAddr, Payload)>,
  timers: TimerSet,
  sleep: Pin<Box<Sleep>>,
  sleep_for: Option<(Instant, TimerId)>,
  /// The protocol asked to stop reading (flow control, ...).
  input_paused: bool,
  /// The socket reached a terminal state.
  transport_done: bool,
  /// `Input::Closed` has been delivered; it is always the last event.
  closed_delivered: bool,
  /// The protocol reported having no outstanding work, so polling it again
  /// would only burn CPU until it receives new input.
  protocol_ready: bool,
  /// `Input::Opened` is delivered exactly once, before the first wait.
  opened: bool,
}

impl<P: DatagramProtocol, T: DatagramTransport> DatagramDriver<P, T> {
  fn new(protocol: P, local: SocketAddr, transport: T) -> Self {
    Self {
      transport,
      proto: Proto::new(protocol, local),
      // Sized for the largest UDP payload, so a datagram is never truncated.
      buf: BytesMut::zeroed(64 * 1024),
      pending: Vec::new(),
      timers: TimerSet::new(),
      // One `Sleep` is reused for the whole socket. It is re-armed only when
      // the earliest deadline changes, so a busy socket does not register a
      // new timer on every loop iteration.
      sleep: Box::pin(sleep_until(TokioInstant::from(
        Instant::now() + Duration::from_secs(3600),
      ))),
      sleep_for: None,
      input_paused: false,
      transport_done: false,
      closed_delivered: false,
      protocol_ready: false,
      opened: false,
    }
  }

  async fn run(mut self, shutdown: &ShutdownHandle) {
    loop {
      if self.tick(shutdown).await == Flow::Stop {
        break;
      }
    }
  }

  /// Runs one iteration: deliver `Opened` or wait for an event, feed it to the
  /// protocol, then apply the effects and send what it produced.
  async fn tick(&mut self, shutdown: &ShutdownHandle) -> Flow {
    if !self.opened {
      self.opened = true;
      if self.feed(Input::Opened) == Flow::Stop {
        return Flow::Stop;
      }

      let close_mode = self.apply_effects();
      return self.settle(close_mode).await;
    }

    let flow = match self.next_event(shutdown).await {
      Wake::Shutdown => Flow::Stop,
      Wake::Protocol(Ok(())) => {
        self.on_protocol_ready();
        Flow::Continue
      },
      Wake::Protocol(Err(e)) => {
        error!("An unrecoverable error occurred when polling: {}", e);
        Flow::Stop
      },
      Wake::Message { peer, len } => {
        // `buf` is reused across iterations, so the bytes are copied out. A
        // zero-length datagram is a message of its own, not EOF.
        let payload = Payload::from(&self.buf[..len]);
        self.proto.peer = peer;
        self.feed(Input::Message { peer, payload })
      },
      Wake::Failed(e) => {
        error!(
          "An unrecoverable error occurred when receiving a datagram: {}",
          e
        );

        self.mark_transport_done();
        self.feed(Input::Closed(close::CloseReason::TransportError))
      },
      Wake::Timer(id) => {
        self.timers.take(id);
        self.sleep_for = None;
        self.feed(Input::Timer(id))
      },
    };

    if flow == Flow::Stop {
      return Flow::Stop;
    }

    /*
      Effect Processing Order:
      - Set `read_pause` flag
      - Set timers
      - Process close logic
    */
    let close_mode = self.apply_effects();
    self.settle(close_mode).await
  }

  /// Races every event source. Each branch carries its own precondition, so
  /// a paused read cannot stall the protocol or the timers.
  async fn next_event(&mut self, shutdown: &ShutdownHandle) -> Wake<P::Error> {
    loop {
      let next = self.timers.next_due();
      if next != self.sleep_for {
        if let Some((at, _)) = next {
          self.sleep.as_mut().reset(TokioInstant::from(at));
        }
        self.sleep_for = next;
      }

      let due = next.map(|(_, id)| id);

      let Self {
        transport,
        proto,
        buf,
        sleep,
        input_paused,
        transport_done,
        closed_delivered,
        protocol_ready,
        ..
      } = self;

      let can_read = !*input_paused && !*transport_done;
      let can_poll = !*closed_delivered && !*protocol_ready;

      let wake = tokio::select! {
        result = transport.recv_from(buf), if can_read => match result {
          Ok((len, peer)) => Wake::Message { peer, len },
          Err(e) => Wake::Failed(e),
        },
        result = std::future::poll_fn(|cx| proto.poll(cx.waker())), if can_poll => {
          Wake::Protocol(result)
        },
        _ = sleep.as_mut(), if due.is_some() && !*closed_delivered => {
          Wake::Timer(due.expect("the timer branch requires a due timer"))
        },
        _ = shutdown.wait() => Wake::Shutdown,
      };

      match wake {
        Wake::Failed(e) if is_retryable(&e) => continue,
        wake => return wake,
      }
    }
  }

  /// Delivers the close request (if any) and sends the queued datagrams.
  async fn settle(&mut self, close_mode: Option<CloseMode>) -> Flow {
    if self.handle_close(close_mode).await == Flow::Stop {
      return Flow::Stop;
    }

    self.send().await;

    if self.transport_done {
      return Flow::Stop;
    }

    Flow::Continue
  }

  /// Delivers a local close and decides whether the loop keeps running. A
  /// datagram socket has no per-peer read side to drain, so any local close
  /// ends the loop.
  async fn handle_close(&mut self, mode: Option<CloseMode>) -> Flow {
    match mode {
      None => Flow::Continue,
      Some(CloseMode::Graceful) => {
        self.timers.clear();
        if !self.closed_delivered {
          self.closed_delivered = true;
          if self.feed(Input::Closed(close::CloseReason::LocalClose))
            == Flow::Stop
          {
            return Flow::Stop;
          }
        }

        // A graceful close still sends whatever the protocol produced while
        // handling `Closed`; an abort drops it.
        self.send().await;

        Flow::Stop
      },
      Some(CloseMode::Abort) => {
        self.timers.clear();
        if !self.closed_delivered {
          self.closed_delivered = true;
          let _ = self.feed(Input::Closed(close::CloseReason::LocalClose));
        }

        self.pending.clear();

        Flow::Stop
      },
    }
  }

  /// Steps the protocol and collects whatever it produced.
  fn feed(&mut self, input: Input<P::Command>) -> Flow {
    // Any new input may give the protocol work to do, so re-arm its branch.
    self.protocol_ready = false;

    match self.proto.step(input) {
      Ok(()) => {
        self.collect_output();
        Flow::Continue
      },
      Err(e) => {
        error!("An unrecoverable error occurred when protocol stepping: {}", e);
        Flow::Stop
      },
    }
  }

  fn collect_output(&mut self) -> usize {
    let (parts, len) = self.proto.output.take_bytes();
    self.pending.extend(parts);
    len
  }

  /// Latches the protocol as quiescent unless it filled the whole write
  /// budget, in which case it still has more to produce.
  fn on_protocol_ready(&mut self) {
    let produced = self.collect_output();
    self.protocol_ready = produced < Output::DEFAULT_BUDGET;
  }

  fn mark_transport_done(&mut self) {
    self.transport_done = true;
    self.closed_delivered = true;
    self.timers.clear();
  }

  /// Applies the effects buffered by the protocol and returns the close request
  /// (if any).
  fn apply_effects(&mut self) -> Option<CloseMode> {
    // A datagram socket has no buffered writer, so a flush request has no
    // counterpart here; draining it keeps the effect buffer in sync.
    let _ = self.proto.output.take_flush_effect();

    if let Some(state) = self.proto.output.take_reader_effect() {
      self.input_paused = state;
    }

    for (id, op) in self.proto.output.take_timer_effect() {
      match op {
        TimerOp::Create(at) => self.timers.arm(id, at),
        TimerOp::Remove => self.timers.cancel(id),
      }
    }

    self.proto.output.take_close_effect()
  }

  /// Sends every queued datagram to the peer it was addressed to.
  ///
  /// A send failure only affects the datagram it happened on, so it is logged
  /// and the loop moves on to the next one.
  async fn send(&mut self) {
    for (peer, payload) in std::mem::take(&mut self.pending) {
      if let Err(e) = self.transport.send_to(&payload, peer).await {
        error!("An error occurred when sending a datagram to {}: {}", peer, e);
      }
    }
  }
}
