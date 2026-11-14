use std::{
  collections::VecDeque,
  io::{self, ErrorKind},
  net::SocketAddr,
  pin::Pin,
  sync::Arc,
  task::{Poll, Waker},
  time::{Duration, Instant},
};

use bytes::BytesMut;
use logger::error;
use network::stream::{ReadHalf, Stream, WriteHalf};
use protocol::{
  Payload, TimerId,
  stream::{
    Context, Input, Output, StreamProtocol,
    close::{self, CloseMode},
    effect::TimerOp,
  },
};
use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};
use tokio::time::{Instant as TokioInstant, Sleep, sleep_until};

use crate::timer::TimerSet;

pub async fn drive_stream<P: StreamProtocol, S: Stream>(
  protocol: P, shutdown_handle: ShutdownHandle,
  connection_count: Arc<ActivityTracker>, stream: S,
) {
  let peer = match stream.peer_addr() {
    Ok(peer) => peer,
    Err(e) => {
      error!("Unable to get peer address: {}", e);
      return;
    },
  };

  connection_count.acquire();
  StreamDriver::new(protocol, peer, stream).run(&shutdown_handle).await;
  connection_count.release();
}

/// Whether the driver should keep running or leave the loop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
  Continue,
  Stop,
}

/// The one thing that woke the driver for this iteration.
enum Wake<E> {
  Data(usize),
  Eof,
  Failed(io::Error),
  Timer(TimerId),
  Writable,
  /// The protocol finished its outstanding work; these are the bytes (if any)
  /// it produced while being polled.
  Protocol(Result<(), E>),
  Shutdown,
}

fn is_retryable(e: &io::Error) -> bool {
  matches!(
    e.kind(),
    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
  )
}

/// The read and write halves of one connection.
struct Io<S: Stream> {
  read: S::ReadHalf,
  write: S::WriteHalf,
}

/// The protocol instance together with the output buffer it writes into.
struct Proto<P: StreamProtocol> {
  peer: SocketAddr,
  state: P,
  output: Output,
}

impl<P: StreamProtocol> Proto<P> {
  fn new(state: P, peer: SocketAddr) -> Self {
    Self { peer, state, output: Output::new() }
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

/// The buffers queued for the write half and their watermarks.
struct Pending {
  queue: VecDeque<Payload>,
  bytes: usize,
  low_watermark: usize,
  high_watermark: usize,
}

impl Pending {
  fn new() -> Self {
    Self {
      queue: VecDeque::new(),
      bytes: 0,
      low_watermark: 32 * 1024,
      high_watermark: 64 * 1024,
    }
  }

  fn push(&mut self, parts: Vec<Payload>, len: usize) {
    self.queue.extend(parts);
    self.bytes += len;
  }

  fn is_empty(&self) -> bool {
    self.bytes == 0
  }

  fn try_write<W: WriteHalf>(&mut self, dst: &mut W) -> io::Result<()> {
    while let Some(bytes) = self.queue.front() {
      if bytes.is_empty() {
        self.queue.pop_front();
        continue;
      }

      match dst.try_write(bytes) {
        Ok(0) => {
          return Err(io::Error::new(
            ErrorKind::WriteZero,
            "try_write returned 0",
          ));
        },
        Ok(n) => {
          self.bytes -= n;
          if n == bytes.len() {
            self.queue.pop_front();
          } else {
            let front = self.queue.front_mut().unwrap();
            *front = front.slice(n..);
          }
        },
        Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
        Err(e) => return Err(e),
      }
    }

    Ok(())
  }

  async fn drain<W: WriteHalf>(&mut self, dst: &mut W) -> io::Result<()> {
    while let Some(bytes) = self.queue.pop_front() {
      if bytes.is_empty() {
        continue;
      }

      dst.write_all(&bytes).await?;
      self.bytes -= bytes.len();
    }

    Ok(())
  }
}

/// Drives one stream connection: waits for whichever source becomes ready,
/// feeds the event to the protocol and applies the effects it produced.
///
/// The wait is a race between five independent sources, each gated on its own
/// condition. Backpressure never blocks the whole wait, so a paused read does
/// not stop writes, timers or the protocol's own work.
struct StreamDriver<P: StreamProtocol, S: Stream> {
  io: Io<S>,
  proto: Proto<P>,
  buf: BytesMut,
  pending: Pending,
  timers: TimerSet,
  sleep: Pin<Box<Sleep>>,
  sleep_for: Option<(Instant, TimerId)>,
  /// The protocol asked to stop reading (flow control, upgrade, ...).
  input_paused: bool,
  /// Queued bytes crossed the high watermark; stop reading and producing.
  output_blocked: bool,
  /// The read half reached a terminal state.
  read_done: bool,
  /// `Input::Closed` has been delivered; it is always the last event.
  closed_delivered: bool,
  /// The protocol reported having no outstanding work, so polling it again
  /// would only burn CPU until it receives new input.
  protocol_ready: bool,
  /// `Input::Opened` is delivered exactly once, before the first wait.
  opened: bool,
}

impl<P: StreamProtocol, S: Stream> StreamDriver<P, S> {
  fn new(protocol: P, peer: SocketAddr, stream: S) -> Self {
    let (read, write) = stream.split();

    Self {
      io: Io { read, write },
      proto: Proto::new(protocol, peer),
      buf: BytesMut::zeroed(64 * 1024),
      pending: Pending::new(),
      timers: TimerSet::new(),
      // One `Sleep` is reused for the whole connection. It is re-armed only
      // when the earliest deadline changes, so a connection with steady
      // traffic does not register a new timer on every loop iteration.
      sleep: Box::pin(sleep_until(TokioInstant::from(
        Instant::now() + Duration::from_secs(3600),
      ))),
      sleep_for: None,
      input_paused: false,
      output_blocked: false,
      read_done: false,
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
  /// protocol, then apply the effects and advance the write side.
  async fn tick(&mut self, shutdown: &ShutdownHandle) -> Flow {
    if !self.opened {
      self.opened = true;
      if self.feed(Input::Opened) == Flow::Stop {
        return Flow::Stop;
      }

      let (flush, close_mode) = self.apply_effects();
      return self.settle(flush, close_mode).await;
    }

    let flow = match self.next_event(shutdown).await {
      Wake::Shutdown => Flow::Stop,
      Wake::Writable => Flow::Continue,
      Wake::Protocol(Ok(())) => {
        self.on_protocol_ready();
        Flow::Continue
      },
      Wake::Protocol(Err(e)) => {
        error!("An unrecoverable error occurred when polling: {}", e);
        Flow::Stop
      },
      Wake::Data(bytes_read) => {
        // `buf` is reused across iterations, so the bytes are copied out
        // instead of split off (splitting would shrink the buffer until it
        // is empty and reads look like EOF).
        let data = Payload::from(&self.buf[..bytes_read]);
        self.feed(Input::Data(data))
      },
      Wake::Eof => {
        // EOF: `Data` must never carry an empty payload, so report the peer
        // close through `Closed` instead of an empty `Data`.
        self.mark_read_done();
        self.feed(Input::Closed(close::CloseReason::PeerClose))
      },
      Wake::Failed(e) => {
        let reason = match e.kind() {
          ErrorKind::ConnectionReset => close::CloseReason::PeerReset,
          ErrorKind::ConnectionAborted => close::CloseReason::PeerClose,
          _ => close::CloseReason::TransportError,
        };

        self.mark_read_done();
        self.feed(Input::Closed(reason))
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
      - Set `flush` flag
      - Set `read_pause` flag
      - Set timers
      - Process close logic
    */
    let (flush, close_mode) = self.apply_effects();
    self.settle(flush, close_mode).await
  }

  /// Races every event source. Each branch carries its own precondition, so
  /// backpressure on one direction cannot stall the others.
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
        io,
        proto,
        buf,
        pending,
        sleep,
        input_paused,
        output_blocked,
        read_done,
        closed_delivered,
        protocol_ready,
        ..
      } = self;

      let can_read = !*input_paused && !*output_blocked && !*read_done;
      let can_poll = !*output_blocked && !*closed_delivered && !*protocol_ready;
      let can_write = !pending.is_empty();

      let wake = tokio::select! {
        result = io.read.read(buf), if can_read => match result {
          Ok(0) => Wake::Eof,
          Ok(bytes_read) => Wake::Data(bytes_read),
          Err(e) => Wake::Failed(e),
        },
        result = std::future::poll_fn(|cx| proto.poll(cx.waker())), if can_poll => {
          Wake::Protocol(result)
        },
        _ = sleep.as_mut(), if due.is_some() && !*closed_delivered => {
          Wake::Timer(due.expect("the timer branch requires a due timer"))
        },
        _ = io.write.writable(), if can_write => Wake::Writable,
        _ = shutdown.wait() => Wake::Shutdown,
      };

      match wake {
        Wake::Failed(e) if is_retryable(&e) => continue,
        wake => return wake,
      }
    }
  }

  /// Applies the state changes that follow a protocol step and advances the
  /// write side.
  async fn settle(
    &mut self, flush: bool, close_mode: Option<CloseMode>,
  ) -> Flow {
    if self.handle_close(close_mode).await == Flow::Stop {
      return Flow::Stop;
    }

    if self.try_write() == Flow::Stop {
      return Flow::Stop;
    }

    if flush {
      self.flush().await;
    }

    // The read half is gone: flush whatever the protocol produced for the
    // final event, close the write half and stop the loop.
    if self.read_done {
      self.finish().await;
      return Flow::Stop;
    }

    Flow::Continue
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
    let (data, len) = self.proto.output.take_bytes();
    self.pending.push(data, len);
    len
  }

  /// Latches the protocol as quiescent unless it filled the whole write
  /// budget, in which case it still has more to produce.
  fn on_protocol_ready(&mut self) {
    let produced = self.collect_output();
    self.protocol_ready = produced < Output::DEFAULT_BUDGET;
  }

  fn mark_read_done(&mut self) {
    self.read_done = true;
    self.closed_delivered = true;
    self.timers.clear();
  }

  /// Applies the effects buffered by the protocol and returns the flush flag
  /// together with the close request (if any).
  fn apply_effects(&mut self) -> (bool, Option<CloseMode>) {
    let flush_requested = self.proto.output.take_flush_effect();
    if let Some(state) = self.proto.output.take_reader_effect() {
      self.input_paused = state;
    }
    for (id, op) in self.proto.output.take_timer_effect() {
      match op {
        TimerOp::Create(at) => self.timers.arm(id, at),
        TimerOp::Remove => self.timers.cancel(id),
      }
    }

    (flush_requested, self.proto.output.take_close_effect())
  }

  /// Writes as much pending data as the socket accepts and refreshes the
  /// watermarks.
  fn try_write(&mut self) -> Flow {
    if let Err(e) = self.pending.try_write(&mut self.io.write) {
      match e.kind() {
        ErrorKind::WriteZero => {
          error!("It seems that a stream cannot accept any bytes");
        },
        _ => {
          error!(
            "An unrecoverable error occurred when writing bytes into stream: {}",
            e
          );
        },
      }

      return Flow::Stop;
    }

    self.output_blocked = if self.pending.bytes >= self.pending.high_watermark {
      true
    } else if self.pending.bytes <= self.pending.low_watermark {
      false
    } else {
      self.output_blocked
    };

    Flow::Continue
  }

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

        if let Err(e) = self.pending.drain(&mut self.io.write).await {
          error!(
            "An unrecoverable error occurred when writing bytes into stream: {}",
            e
          );
          return Flow::Stop;
        }
        self.output_blocked = false;

        Flow::Continue
      },
      Some(CloseMode::Abort) => {
        self.timers.clear();
        if !self.closed_delivered {
          let _ = self.feed(Input::Closed(close::CloseReason::LocalClose));
        }

        Flow::Stop
      },
    }
  }

  async fn flush(&mut self) {
    if let Err(e) = self.io.write.flush().await {
      error!("An unrecoverable error occurred when flushing: {}", e);
      if !self.closed_delivered {
        self.closed_delivered = true;
        self.timers.clear();
        let _ = self.feed(Input::Closed(close::CloseReason::LocalClose));
      }
    }
  }

  async fn finish(&mut self) {
    self.timers.clear();
    if let Err(e) = self.pending.drain(&mut self.io.write).await {
      error!(
        "An unrecoverable error occurred when writing bytes into stream: {}",
        e
      );
    }
    if let Err(e) = self.io.write.flush().await {
      error!("An unrecoverable error occurred when flushing: {}", e);
    }
    if let Err(e) = self.io.write.shutdown().await {
      error!("An unrecoverable error occurred when shutting down: {}", e);
    }
  }
}
