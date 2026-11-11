//! Micro-benchmarks for the per-connection timer set.
//!
//! Run with `cargo bench -p depot --bench timer_set`.
//!
//! Timers are per connection and every operation is local to one task, so the
//! per-operation numbers below are the whole story: there is no global lock
//! or cross-connection contention to measure separately.

use std::{
  hint::black_box,
  time::{Duration, Instant},
};

use depot::timer::TimerSet;
use protocol::TimerId;

const SIZES: [usize; 5] = [1, 10, 100, 1_000, 10_000];
const OPS: u64 = 1_000_000;

fn fill(n: usize, deadline: impl Fn(usize) -> Instant) -> TimerSet {
  let mut set = TimerSet::new();
  for i in 0..n {
    set.arm(TimerId(i as u32), deadline(i));
  }

  set
}

fn per_op(total: Duration, ops: u64) -> f64 {
  total.as_nanos() as f64 / ops as f64
}

/// Arms a fresh timer id and cancels it, i.e. the per-request churn pattern.
fn bench_arm_cancel(n: usize) -> f64 {
  let far = Instant::now() + Duration::from_secs(3600);
  let mut set = fill(n, |i| far + Duration::from_nanos(i as u64));
  let mut next = n as u32;

  let start = Instant::now();
  for _ in 0..OPS {
    let id = TimerId(black_box(next));
    set.arm(id, far);
    set.cancel(id);
    next = next.wrapping_add(1);
  }

  let elapsed = start.elapsed();
  black_box(&set);

  per_op(elapsed, OPS)
}

/// Re-arms the same id with a new deadline, i.e. a timeout being pushed back.
fn bench_arm_replace(n: usize) -> f64 {
  let far = Instant::now() + Duration::from_secs(3600);
  let mut set = fill(n, |i| far + Duration::from_nanos(i as u64 + 1));
  let id = TimerId(0);

  let start = Instant::now();
  for i in 0..OPS {
    set.arm(id, far + Duration::from_nanos(i % 1_000));
  }

  let elapsed = start.elapsed();
  black_box(&set);

  per_op(elapsed, OPS)
}

/// Looks up the earliest deadline when nothing is due (the idle loop path).
fn bench_next_due(n: usize) -> f64 {
  let far = Instant::now() + Duration::from_secs(3600);
  let mut set = fill(n, |i| far + Duration::from_nanos(i as u64));

  let start = Instant::now();
  for _ in 0..OPS {
    black_box(set.next_due());
  }

  let elapsed = start.elapsed();
  black_box(&set);

  per_op(elapsed, OPS)
}

/// Drains already expired timers through `next_due` + `take`.
///
/// Sets are filled before the timed section and drained in a batch, so the
/// `Instant::now` overhead of a sample does not dominate small sizes.
fn bench_fire(n: usize) -> f64 {
  let past = Instant::now() - Duration::from_secs(1);
  let batch = (50_000 / n).max(1);
  let rounds = (200_000 / (n * batch)).max(1);
  let mut total = Duration::ZERO;

  for _ in 0..rounds {
    let mut sets: Vec<TimerSet> = (0..batch)
      .map(|_| fill(n, |i| past + Duration::from_nanos(i as u64)))
      .collect();

    let start = Instant::now();
    for set in &mut sets {
      while let Some((_, id)) = set.next_due() {
        set.take(id);
      }
    }
    total += start.elapsed();
    black_box(&sets);
  }

  per_op(total, (rounds * batch * n) as u64)
}

fn main() {
  println!("{:<22} {:>8}  {:>14}", "scenario", "n", "per op");

  for n in SIZES {
    println!(
      "{:<22} {:>8}  {:>10.1} ns",
      "arm + cancel",
      n,
      bench_arm_cancel(n)
    );
  }

  for n in SIZES {
    println!(
      "{:<22} {:>8}  {:>10.1} ns",
      "arm (replace)",
      n,
      bench_arm_replace(n)
    );
  }

  for n in SIZES {
    println!("{:<22} {:>8}  {:>10.1} ns", "next_due", n, bench_next_due(n));
  }

  for n in SIZES {
    println!("{:<22} {:>8}  {:>10.1} ns", "fire", n, bench_fire(n));
  }
}
