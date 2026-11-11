use std::{
  collections::VecDeque,
  io::{self, ErrorKind},
  net::SocketAddr,
  pin::Pin,
  sync::Arc,
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

/// The single event that woke the driver for this iteration.
enum Wake {
  Data(usize),
  Eof,
  Failed(io::Error),
  Timer(TimerId),
  Shutdown,
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

/// Drives one stream connection: waits for events, feeds them to the protocol
/// and applies the effects the protocol produced.
struct StreamDriver<P: StreamProtocol, S: Stream> {
  protocol: P,
  peer: SocketAddr,
  read: S::ReadHalf,
  write: S::WriteHalf,
  output: Output,
  buf: BytesMut,
  pending: Pending,
  timers: TimerSet,
  sleep: Pin<Box<Sleep>>,
  sleep_for: Option<(Instant, TimerId)>,
  read_paused: bool,
  read_done: bool,
  closed_delivered: bool,
}

impl<P: StreamProtocol, S: Stream> StreamDriver<P, S> {
  fn new(protocol: P, peer: SocketAddr, stream: S) -> Self {
    let (read, write) = stream.split();

    Self {
      protocol,
      peer,
      read,
      write,
      output: Output::new(),
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
      read_paused: false,
      read_done: false,
      closed_delivered: false,
    }
  }

  async fn run(mut self, shutdown: &ShutdownHandle) {
    loop {
      if self.tick(shutdown).await == Flow::Stop {
        break;
      }
    }
  }

  /// Runs one iteration: wait for an event, feed it to the protocol, apply
  /// the produced effects and advance the write side.
  async fn tick(&mut self, shutdown: &ShutdownHandle) -> Flow {
    if !self.read_paused && !self.read_done {
      let wake = self.next_event(shutdown).await;
      if self.on_event(wake) == Flow::Stop {
        return Flow::Stop;
      }

      self.collect_output();
    }

    /*
      Effect Processing Order:
      - Set `flush` flag
      - Set `read_pause` flag
      - Set timers
      - Process close logic
    */
    let (flush, close) = self.apply_effects();
    if self.handle_close(close, shutdown).await == Flow::Stop {
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

  /// Waits for the next read result, timer expiry or shutdown.
  async fn next_event(&mut self, shutdown: &ShutdownHandle) -> Wake {
    loop {
      let next = self.timers.next_due();
      if next != self.sleep_for {
        if let Some((at, _)) = next {
          self.sleep.as_mut().reset(TokioInstant::from(at));
        }
        self.sleep_for = next;
      }

      let due = next.map(|(_, id)| id);
      let res = tokio::select! {
        result = self.read.read(&mut self.buf) => result,
        _ = &mut self.sleep, if due.is_some() && !self.closed_delivered => {
          return Wake::Timer(due.expect("the timer branch requires a due timer"));
        },
        _ = shutdown.wait() => return Wake::Shutdown,
      };

      match res {
        Ok(0) => return Wake::Eof,
        Ok(bytes_read) => return Wake::Data(bytes_read),
        Err(e)
          if matches!(
            e.kind(),
            ErrorKind::WouldBlock
              | ErrorKind::TimedOut
              | ErrorKind::Interrupted
          ) =>
        {
          continue;
        },
        Err(e) => return Wake::Failed(e),
      }
    }
  }

  /// Feeds the event to the protocol.
  fn on_event(&mut self, wake: Wake) -> Flow {
    match wake {
      Wake::Shutdown => Flow::Stop,
      Wake::Timer(id) => {
        self.timers.take(id);
        self.sleep_for = None;
        self.step(Input::Timer(id))
      },
      Wake::Eof => {
        // EOF: `Data` must never carry an empty payload, so report the peer
        // close through `Closed` instead of an empty `Data`.
        self.mark_read_done();
        self.step(Input::Closed(close::CloseReason::PeerClose))
      },
      Wake::Data(bytes_read) => {
        // `buf` is reused across iterations, so the bytes are copied out
        // instead of split off (splitting would shrink the buffer until it
        // is empty and reads look like EOF).
        let data = Payload::from(&self.buf[..bytes_read]);
        self.step(Input::Data(data))
      },
      Wake::Failed(e) => {
        let reason = match e.kind() {
          ErrorKind::ConnectionReset => close::CloseReason::PeerReset,
          ErrorKind::ConnectionAborted => close::CloseReason::PeerClose,
          _ => close::CloseReason::TransportError,
        };

        self.mark_read_done();
        self.step(Input::Closed(reason))
      },
    }
  }

  fn mark_read_done(&mut self) {
    self.read_done = true;
    self.closed_delivered = true;
    self.timers.clear();
  }

  /// Steps the protocol and maps its error to a log plus a control flow.
  fn step(&mut self, input: Input<P::Command>) -> Flow {
    let mut context = Context::new(self.peer, &mut self.output);
    match self.protocol.step(&mut context, input) {
      Ok(()) => Flow::Continue,
      Err(e) => {
        error!("An unrecoverable error occurred when protocol stepping: {}", e);
        Flow::Stop
      },
    }
  }

  fn collect_output(&mut self) {
    let (data, len) = self.output.take_bytes();
    self.pending.push(data, len);
  }

  /// Applies the effects buffered by the protocol and returns the flush flag
  /// together with the close request (if any).
  fn apply_effects(&mut self) -> (bool, Option<CloseMode>) {
    let flush_requested = self.output.take_flush_effect();
    if let Some(state) = self.output.take_reader_effect() {
      self.read_paused = state;
    }
    for (id, op) in self.output.take_timer_effect() {
      match op {
        TimerOp::Create(at) => self.timers.arm(id, at),
        TimerOp::Remove => self.timers.cancel(id),
      }
    }

    (flush_requested, self.output.take_close_effect())
  }

  /// Writes as much pending data as the socket accepts, applies the
  /// watermarks and waits for writability when bytes are still queued.
  async fn pump_writes(&mut self, shutdown: &ShutdownHandle) -> Flow {
    if let Err(e) = self.pending.try_write(&mut self.write) {
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

    if self.pending.bytes >= self.pending.high_watermark {
      self.read_paused = true;
    } else if self.pending.bytes <= self.pending.low_watermark {
      self.read_paused = false;
    }

    // The socket buffer is full: wait for writability instead of spinning on
    // `try_write`.
    if !self.pending.is_empty() {
      let res = tokio::select! {
        result = self.write.writable() => result,
        _ = shutdown.wait() => return Flow::Stop,
      };
      if let Err(e) = res {
        error!(
          "An unrecoverable error occurred when waiting for writable: {}",
          e
        );
        return Flow::Stop;
      }
    }

    Flow::Continue
  }

  async fn handle_close(
    &mut self, mode: Option<CloseMode>, shutdown: &ShutdownHandle,
  ) -> Flow {
    match mode {
      None => self.pump_writes(shutdown).await,
      Some(CloseMode::Graceful) => {
        self.timers.clear();
        if !self.closed_delivered {
          self.closed_delivered = true;
          if self.step(Input::Closed(close::CloseReason::LocalClose))
            == Flow::Stop
          {
            return Flow::Stop;
          }
        }

        if let Err(e) = self.pending.drain(&mut self.write).await {
          error!(
            "An unrecoverable error occurred when writing bytes into stream: {}",
            e
          );
          return Flow::Stop;
        }

        Flow::Continue
      },
      Some(CloseMode::Abort) => {
        self.timers.clear();
        if !self.closed_delivered {
          let _ = self.step(Input::Closed(close::CloseReason::LocalClose));
        }

        Flow::Stop
      },
    }
  }

  async fn flush(&mut self) {
    if let Err(e) = self.write.flush().await {
      error!("An unrecoverable error occurred when flushing: {}", e);
      if !self.closed_delivered {
        self.closed_delivered = true;
        self.timers.clear();
        let _ = self.step(Input::Closed(close::CloseReason::LocalClose));
      }
    }
  }

  async fn finish(&mut self) {
    self.timers.clear();
    if let Err(e) = self.pending.drain(&mut self.write).await {
      error!(
        "An unrecoverable error occurred when writing bytes into stream: {}",
        e
      );
    }
    if let Err(e) = self.write.flush().await {
      error!("An unrecoverable error occurred when flushing: {}", e);
    }
    if let Err(e) = self.write.shutdown().await {
      error!("An unrecoverable error occurred when shutting down: {}", e);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{io, sync::Arc, time::Duration};

  use network::transport::{StreamTransport, Transport, tcp::TcpTransport};
  use protocol::{
    Payload, Protocol, TimerId,
    stream::{Context, Input, StreamProtocol},
  };
  use sync::{activity_tracker::ActivityTracker, handle::ShutdownHandle};
  use tokio::io::{AsyncReadExt, AsyncWriteExt};

  use super::drive_stream;

  #[derive(Clone)]
  struct Echo;

  impl Protocol for Echo {
    type Command = ();
    type Error = io::Error;

    const NAME: &'static str = "test:echo";
  }

  impl StreamProtocol for Echo {
    fn step(
      &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
    ) -> Result<(), Self::Error> {
      if let Input::Data(bytes) = input {
        let _ = ctx.write_bytes(bytes);
      }

      Ok(())
    }
  }

  #[tokio::test]
  async fn echoes_and_stops_at_peer_eof() {
    let listener = TcpTransport::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
      let stream = listener.accept().await.unwrap();
      drive_stream(
        Echo,
        ShutdownHandle::new(1_000),
        Arc::new(ActivityTracker::new()),
        stream,
      )
      .await;
    });

    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();

    for expected in [b"hello".as_slice(), b"world".as_slice()] {
      client.write_all(expected).await.unwrap();

      let mut buf = vec![0_u8; expected.len()];
      client.read_exact(&mut buf).await.unwrap();
      assert_eq!(buf, expected);
    }

    drop(client);
    tokio::time::timeout(std::time::Duration::from_secs(3), server)
      .await
      .unwrap()
      .unwrap();
  }

  #[derive(Clone)]
  struct TimerEcho {
    armed: bool,
  }

  impl Protocol for TimerEcho {
    type Command = ();
    type Error = io::Error;

    const NAME: &'static str = "test:timer-echo";
  }

  impl StreamProtocol for TimerEcho {
    fn step(
      &mut self, ctx: &mut Context<'_>, input: Input<Self::Command>,
    ) -> Result<(), Self::Error> {
      match input {
        Input::Data(_) if !self.armed => {
          self.armed = true;
          let at = ctx.now() + Duration::from_millis(50);
          ctx.arm_timer(TimerId(1), at);
        },
        Input::Timer(TimerId(1)) => {
          let _ = ctx.write_bytes(Payload::from_static(b"tick"));
          ctx.flush();
        },
        _ => {},
      }

      Ok(())
    }
  }

  #[tokio::test]
  async fn delivers_an_armed_timer_exactly_once() {
    let listener = TcpTransport::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
      let stream = listener.accept().await.unwrap();
      drive_stream(
        TimerEcho { armed: false },
        ShutdownHandle::new(1_000),
        Arc::new(ActivityTracker::new()),
        stream,
      )
      .await;
    });

    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
    client.write_all(b"go").await.unwrap();

    let mut buf = [0_u8; 4];
    tokio::time::timeout(Duration::from_secs(3), client.read_exact(&mut buf))
      .await
      .unwrap()
      .unwrap();
    assert_eq!(&buf, b"tick");

    // The timer must not fire twice: one arm yields exactly one event.
    let mut extra = [0_u8; 1];
    let res =
      tokio::time::timeout(Duration::from_millis(300), client.read(&mut extra))
        .await;
    assert!(res.is_err(), "timer should fire exactly once");

    drop(client);
    tokio::time::timeout(Duration::from_secs(3), server)
      .await
      .unwrap()
      .unwrap();
  }
}
