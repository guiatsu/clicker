use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::JoinHandle,
};

use crate::settings::Point;

#[derive(Debug, Clone)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum HotkeyEvent {
    KeyChanged {
        key_code: u16,
        is_down: bool,
        down_keys: Vec<u16>,
        previous_keys: Vec<u16>,
        cursor: Option<Point>,
    },
}

pub struct HotkeyService {
    pub events: Receiver<HotkeyEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    available: bool,
}

impl HotkeyService {
    pub fn new() -> Self {
        let (sender, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        let thread = {
            let thread_stop = Arc::clone(&stop);
            Some(std::thread::spawn(move || {
                windows::watch(thread_stop, sender)
            }))
        };
        #[cfg(not(windows))]
        let thread: Option<JoinHandle<()>> = {
            drop(sender);
            None
        };

        Self {
            events,
            stop,
            thread,
            available: cfg!(windows),
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::{
        collections::BTreeSet,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::Sender,
        },
        thread,
        time::Duration,
    };

    use windows_sys::Win32::{
        Foundation::POINT,
        UI::{Input::KeyboardAndMouse::GetAsyncKeyState, WindowsAndMessaging::GetCursorPos},
    };

    use crate::settings::{Point, normalize_key};

    use super::HotkeyEvent;

    pub fn watch(stop: Arc<AtomicBool>, sender: Sender<HotkeyEvent>) {
        let mut previous_keys = BTreeSet::new();

        while !stop.load(Ordering::Acquire) {
            let mut physical = [false; 256];
            for (key, state) in physical.iter_mut().enumerate().skip(1) {
                if is_keyboard_key(key as u16) {
                    *state = unsafe { GetAsyncKeyState(key as i32) } < 0;
                }
            }

            let current_keys = physical
                .iter()
                .enumerate()
                .filter_map(|(key, is_down)| is_down.then_some(normalize_key(key as u16)))
                .collect::<BTreeSet<_>>();

            let mut working = previous_keys.clone();
            for key in current_keys
                .difference(&previous_keys)
                .copied()
                .collect::<Vec<_>>()
            {
                let before = working.iter().copied().collect::<Vec<_>>();
                working.insert(key);
                let cursor = cursor_position();
                if sender
                    .send(HotkeyEvent::KeyChanged {
                        key_code: key,
                        is_down: true,
                        down_keys: working.iter().copied().collect(),
                        previous_keys: before,
                        cursor,
                    })
                    .is_err()
                {
                    return;
                }
            }

            for key in previous_keys
                .difference(&current_keys)
                .copied()
                .collect::<Vec<_>>()
            {
                let before = working.iter().copied().collect::<Vec<_>>();
                working.remove(&key);
                if sender
                    .send(HotkeyEvent::KeyChanged {
                        key_code: key,
                        is_down: false,
                        down_keys: working.iter().copied().collect(),
                        previous_keys: before,
                        cursor: None,
                    })
                    .is_err()
                {
                    return;
                }
            }

            previous_keys = current_keys;
            thread::sleep(Duration::from_millis(4));
        }
    }

    fn is_keyboard_key(key: u16) -> bool {
        // VK_LBUTTON..VK_XBUTTON2 belong to the mouse and are not shortcuts.
        !matches!(key, 0x01..=0x06 | 0x07)
    }

    fn cursor_position() -> Option<Point> {
        let mut cursor = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut cursor) } != 0).then_some(Point {
            x: cursor.x,
            y: cursor.y,
        })
    }
}
