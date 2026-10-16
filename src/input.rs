use std::io;

use crate::settings::{MouseButton, Point};

pub trait ClickInput: Send {
    fn move_cursor(&mut self, point: Point) -> io::Result<()>;
    fn send_button(&mut self, button: MouseButton, down: bool) -> io::Result<()>;
}

pub fn create_backend() -> io::Result<Box<dyn ClickInput>> {
    #[cfg(windows)]
    {
        Ok(Box::new(windows::WindowsInput))
    }
    #[cfg(not(windows))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "O envio de cliques só está disponível no Windows.",
        ))
    }
}

#[cfg(windows)]
mod windows {
    use std::{io, mem::size_of};

    use windows_sys::{
        Win32::UI::{
            Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
                MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
                MOUSEEVENTF_RIGHTUP, MOUSEINPUT, SendInput,
            },
            WindowsAndMessaging::SetCursorPos,
        },
        core::BOOL,
    };

    use crate::settings::{MouseButton, Point};

    use super::ClickInput;

    pub struct WindowsInput;

    impl ClickInput for WindowsInput {
        fn move_cursor(&mut self, point: Point) -> io::Result<()> {
            let result: BOOL = unsafe { SetCursorPos(point.x, point.y) };
            if result == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        fn send_button(&mut self, button: MouseButton, down: bool) -> io::Result<()> {
            let flags = match (button, down) {
                (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
                (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
                (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
                (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
                (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
                (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
            };
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
            if sent == 1 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }
}
