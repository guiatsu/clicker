use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    input::{self, ClickInput},
    settings::{ClickKind, DestinationMode, Settings, SpeedMode},
};

#[derive(Debug, Clone, Default)]
pub struct EngineStats {
    pub clicks_sent: u64,
    pub successful_operations: u64,
    pub failed_events: u64,
    pub elapsed: Duration,
}

#[derive(Debug)]
pub enum EngineEvent {
    Progress(EngineStats),
    Finished {
        stats: EngineStats,
        error: Option<String>,
    },
}

pub struct EngineSession {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineSession {
    pub fn start(settings: Settings, events: Sender<EngineEvent>) -> Result<Self, String> {
        settings.validate()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("autoclick-engine".to_owned())
            .spawn(move || run(settings, events, thread_stop))
            .map_err(|error| format!("Não foi possível iniciar o motor: {error}"))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }

    pub fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for EngineSession {
    fn drop(&mut self) {
        self.stop();
        self.join();
    }
}

fn run(settings: Settings, events: Sender<EngineEvent>, stop: Arc<AtomicBool>) {
    let started = Instant::now();
    let mut stats = EngineStats::default();
    let mut last_report = started;
    let result = input::create_backend().and_then(|mut backend| {
        execute(
            &settings,
            backend.as_mut(),
            &stop,
            &mut stats,
            &events,
            started,
            &mut last_report,
        )
    });
    stats.elapsed = started.elapsed();
    let error = result.err().map(|error| error.to_string());
    let _ = events.send(EngineEvent::Finished { stats, error });
}

fn execute(
    settings: &Settings,
    input: &mut dyn ClickInput,
    stop: &AtomicBool,
    stats: &mut EngineStats,
    events: &Sender<EngineEvent>,
    started: Instant,
    last_report: &mut Instant,
) -> io::Result<()> {
    let sequence = settings.destination_mode == DestinationMode::PositionSequence;
    let clicks_per_group = settings.clicks_per_position.max(1);
    let click_multiplier = if settings.click_kind == ClickKind::Double {
        2
    } else {
        1
    };
    let max_clicks = settings
        .max_clicks
        .or_else(|| (!sequence && !settings.repeat_continuously).then_some(1));
    let interval = if settings.speed_mode == SpeedMode::Unlimited {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(settings.interval_ms / 1000.0)
    };
    let press_duration = Duration::from_secs_f64(settings.press_duration_ms / 1000.0);
    let position_interval =
        Duration::from_secs_f64(settings.interval_between_positions_ms / 1000.0);
    let double_gap = Duration::from_millis(60);
    let mut cycles_done = 0_u64;

    loop {
        if should_stop(stop, stats, max_clicks) {
            break;
        }
        let position_count = match settings.destination_mode {
            DestinationMode::PositionSequence => settings.positions.len(),
            DestinationMode::CurrentCursor | DestinationMode::FixedPosition => 1,
        };

        for position_index in 0..position_count {
            let target = match settings.destination_mode {
                DestinationMode::CurrentCursor => None,
                DestinationMode::FixedPosition => Some(settings.fixed_position),
                DestinationMode::PositionSequence => Some(settings.positions[position_index]),
            };
            for _ in 0..clicks_per_group {
                for click_index in 0..click_multiplier {
                    if should_stop(stop, stats, max_clicks) {
                        break;
                    }
                    if let Some(point) = target {
                        match input.move_cursor(point) {
                            Ok(()) => stats.successful_operations += 1,
                            Err(error) => {
                                stats.failed_events += 1;
                                report_if_due(stats, events, started, last_report);
                                return Err(error);
                            }
                        }
                    }

                    let click_started = Instant::now();
                    if let Err(error) = input.send_button(settings.mouse_button, true) {
                        stats.failed_events += 1;
                        report_if_due(stats, events, started, last_report);
                        return Err(error);
                    }
                    stats.successful_operations += 1;
                    wait_duration(press_duration, settings.speed_mode, stop);
                    if let Err(error) = input.send_button(settings.mouse_button, false) {
                        stats.failed_events += 1;
                        if input.send_button(settings.mouse_button, false).is_err() {
                            stats.failed_events += 1;
                            report_if_due(stats, events, started, last_report);
                            return Err(error);
                        }
                        stats.successful_operations += 1;
                    } else {
                        stats.successful_operations += 1;
                    }
                    stats.clicks_sent += 1;
                    report_if_due(stats, events, started, last_report);

                    if click_index + 1 < click_multiplier {
                        wait_duration(double_gap, settings.speed_mode, stop);
                    } else if interval > Duration::ZERO {
                        wait_until(click_started + interval, settings.speed_mode, stop);
                    }
                }
                if should_stop(stop, stats, max_clicks) {
                    break;
                }
            }
            if should_stop(stop, stats, max_clicks) {
                break;
            }
            if sequence && position_index + 1 < position_count {
                wait_duration(position_interval, settings.speed_mode, stop);
            }
        }

        cycles_done += 1;
        if !settings.repeat_continuously && sequence && cycles_done >= settings.cycles {
            break;
        }
    }

    stats.elapsed = started.elapsed();
    let _ = events.send(EngineEvent::Progress(stats.clone()));
    Ok(())
}

fn should_stop(stop: &AtomicBool, stats: &EngineStats, max_clicks: Option<u64>) -> bool {
    stop.load(Ordering::Acquire) || max_clicks.is_some_and(|max| stats.clicks_sent >= max)
}

fn report_if_due(
    stats: &EngineStats,
    events: &Sender<EngineEvent>,
    started: Instant,
    last_report: &mut Instant,
) {
    if last_report.elapsed() >= Duration::from_millis(250) {
        let mut snapshot = stats.clone();
        snapshot.elapsed = started.elapsed();
        let _ = events.send(EngineEvent::Progress(snapshot));
        *last_report = Instant::now();
    }
}

fn wait_duration(duration: Duration, mode: SpeedMode, stop: &AtomicBool) {
    if duration.is_zero() {
        return;
    }
    wait_until(Instant::now() + duration, mode, stop);
}

fn wait_until(deadline: Instant, mode: SpeedMode, stop: &AtomicBool) {
    while !stop.load(Ordering::Acquire) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        if mode == SpeedMode::HighPrecision && remaining <= Duration::from_micros(250) {
            std::hint::spin_loop();
        } else {
            let sleep = if mode == SpeedMode::HighPrecision {
                remaining.saturating_sub(Duration::from_micros(250))
            } else {
                remaining.min(Duration::from_millis(10))
            };
            if !sleep.is_zero() {
                thread::sleep(sleep);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{atomic::AtomicBool, mpsc},
    };

    use crate::settings::{MouseButton, Point, Settings};

    use super::*;

    #[derive(Default)]
    struct FakeInput {
        events: Vec<(bool, Option<Point>)>,
    }

    impl ClickInput for FakeInput {
        fn move_cursor(&mut self, point: Point) -> io::Result<()> {
            self.events.push((false, Some(point)));
            Ok(())
        }

        fn send_button(&mut self, _button: MouseButton, down: bool) -> io::Result<()> {
            self.events.push((down, None));
            Ok(())
        }
    }

    #[test]
    fn completes_a_finite_run_and_releases_button() {
        let settings = Settings {
            repeat_continuously: false,
            max_clicks: Some(3),
            press_duration_ms: 0.0,
            ..Settings::default()
        };
        let (sender, receiver) = mpsc::channel();
        let started = Instant::now();
        let mut last_report = started;
        let mut stats = EngineStats::default();
        let mut input = FakeInput::default();
        execute(
            &settings,
            &mut input,
            &AtomicBool::new(false),
            &mut stats,
            &sender,
            started,
            &mut last_report,
        )
        .unwrap();
        assert_eq!(stats.clicks_sent, 3);
        assert_eq!(input.events.iter().filter(|event| event.0).count(), 3);
        assert_eq!(input.events.iter().filter(|event| !event.0).count(), 3);
        assert!(matches!(receiver.try_recv(), Ok(EngineEvent::Progress(_))));
    }
}
