use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::JoinHandle,
};

use crate::settings::{Hotkey, Point};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum HotkeyEvent {
    KeyDown {
        hotkey: Hotkey,
        cursor: Option<Point>,
    },
    KeyUp {
        hotkey: Hotkey,
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

    use crate::settings::{Hotkey, Modifiers, Point};

    use super::HotkeyEvent;

    pub fn watch(stop: Arc<AtomicBool>, sender: Sender<HotkeyEvent>) {
        let mut previous = [false; 256];
        let mut active_chords = [Hotkey::default(); 256];

        while !stop.load(Ordering::Acquire) {
            let mut down = [false; 256];
            for (key, state) in down.iter_mut().enumerate().skip(1) {
                *state = unsafe { GetAsyncKeyState(key as i32) } < 0;
            }
            let modifiers = Modifiers {
                control: down[0x11] || down[0xa2] || down[0xa3],
                shift: down[0x10] || down[0xa0] || down[0xa1],
                alt: down[0x12] || down[0xa4] || down[0xa5],
                windows: down[0x5b] || down[0x5c],
            };

            for (key, (&is_down, &was_down)) in down.iter().zip(&previous).enumerate().skip(1) {
                if is_down == was_down || is_modifier(key as u16) {
                    continue;
                }
                if is_down {
                    let hotkey = Hotkey {
                        key_code: key as u16,
                        modifiers,
                    };
                    active_chords[key] = hotkey;
                    let mut cursor = POINT { x: 0, y: 0 };
                    let has_cursor = unsafe { GetCursorPos(&mut cursor) } != 0;
                    let cursor = has_cursor.then_some(Point {
                        x: cursor.x,
                        y: cursor.y,
                    });
                    if sender
                        .send(HotkeyEvent::KeyDown { hotkey, cursor })
                        .is_err()
                    {
                        return;
                    }
                } else {
                    let hotkey = active_chords[key];
                    if hotkey.is_configured() && sender.send(HotkeyEvent::KeyUp { hotkey }).is_err()
                    {
                        return;
                    }
                }
            }
            previous = down;
            thread::sleep(Duration::from_millis(4));
        }
    }

    fn is_modifier(key: u16) -> bool {
        matches!(key, 0x10..=0x12 | 0x5b..=0x5c | 0xa0..=0xa5)
    }
}
