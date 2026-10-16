use std::{sync::mpsc::Sender, thread::JoinHandle};

use crate::settings::Point;

#[cfg_attr(not(windows), allow(dead_code))]
enum OverlayCommand {
    Show(bool),
    Points(Vec<Point>),
    Close,
}

pub struct Overlay {
    commands: Option<Sender<OverlayCommand>>,
    thread: Option<JoinHandle<()>>,
    visible: bool,
    error: Option<String>,
}

impl Overlay {
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            let (commands, receiver) = std::sync::mpsc::channel();
            let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
            let thread = std::thread::Builder::new()
                .name("position-overlay".to_owned())
                .spawn(move || windows::run(receiver, ready_sender));
            match thread {
                Ok(thread) => {
                    match ready_receiver.recv_timeout(std::time::Duration::from_secs(3)) {
                        Ok(Ok(())) => Self {
                            commands: Some(commands),
                            thread: Some(thread),
                            visible: false,
                            error: None,
                        },
                        Ok(Err(error)) => Self {
                            commands: Some(commands),
                            thread: Some(thread),
                            visible: false,
                            error: Some(error),
                        },
                        Err(error) => Self {
                            commands: Some(commands),
                            thread: Some(thread),
                            visible: false,
                            error: Some(format!(
                                "Não foi possível iniciar a camada visual: {error}"
                            )),
                        },
                    }
                }
                Err(error) => Self {
                    commands: None,
                    thread: None,
                    visible: false,
                    error: Some(format!("Não foi possível iniciar a camada visual: {error}")),
                },
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                commands: None,
                thread: None,
                visible: false,
                error: Some(
                    "O overlay transparente está disponível somente no Windows.".to_owned(),
                ),
            }
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self, points: Vec<Point>) -> Result<bool, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.commands.is_none() {
            return Err(self
                .error
                .clone()
                .unwrap_or_else(|| "O overlay não está disponível.".to_owned()));
        }
        self.set_points(points);
        self.visible = !self.visible;
        self.commands
            .as_ref()
            .unwrap()
            .send(OverlayCommand::Show(self.visible))
            .map_err(|error| format!("Não foi possível atualizar o overlay: {error}"))?;
        Ok(self.visible)
    }

    pub fn set_points(&self, points: Vec<Point>) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(OverlayCommand::Points(points));
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        if let Some(commands) = &self.commands {
            let _ = commands.send(OverlayCommand::Close);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
pub fn set_process_dpi_awareness() {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        #[link_name = "SetProcessDpiAwarenessContext"]
        fn set_process_dpi_awareness_context(context: *mut c_void) -> i32;
    }

    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 is the documented pseudo-handle -4.
    let per_monitor_v2 = (-4_isize) as *mut c_void;
    let _ = unsafe { set_process_dpi_awareness_context(per_monitor_v2) };
}

#[cfg(windows)]
mod windows {
    use std::{
        ffi::c_void,
        mem::zeroed,
        ptr::{null, null_mut},
        sync::mpsc::{Receiver, SyncSender, TryRecvError},
        thread,
        time::Duration,
    };

    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BLACK_BRUSH, BeginPaint, COLORREF, CreatePen, DeleteObject, Ellipse, EndPaint,
            GetStockObject, HBRUSH, HDC, HGDIOBJ, LineTo, MoveToEx, NULL_BRUSH, PAINTSTRUCT,
            PS_SOLID, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GetSystemMetrics, HWND_TOPMOST, InvalidateRect, LWA_COLORKEY, MSG,
            PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassW, SM_CXVIRTUALSCREEN,
            SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOWNOACTIVATE,
            SWP_NOACTIVATE, SWP_SHOWWINDOW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
            TranslateMessage, WM_DESTROY, WM_NCCREATE, WM_NCHITTEST, WM_PAINT, WNDCLASSW,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
            WS_POPUP,
        },
    };

    use crate::settings::Point;

    use super::OverlayCommand;

    const CLASS_NAME: &[u16] = &[
        82, 117, 115, 116, 65, 117, 116, 111, 67, 108, 105, 99, 107, 101, 114, 79, 118, 101, 114,
        108, 97, 121, 0,
    ];
    const WINDOW_TITLE: &[u16] = &[
        82, 117, 115, 116, 65, 117, 116, 111, 67, 108, 105, 99, 107, 101, 114, 79, 118, 101, 114,
        108, 97, 121, 0,
    ];

    struct OverlayState {
        points: Vec<Point>,
        virtual_left: i32,
        virtual_top: i32,
    }

    pub fn run(receiver: Receiver<OverlayCommand>, ready: SyncSender<Result<(), String>>) {
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            let _ = ready.send(Err(
                "Não foi possível localizar o módulo do aplicativo.".to_owned()
            ));
            return;
        }

        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: unsafe { GetStockObject(BLACK_BRUSH as i32) as HBRUSH },
            lpszMenuName: null(),
            lpszClassName: CLASS_NAME.as_ptr(),
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            let _ = ready.send(Err(
                "Não foi possível registrar a janela do overlay.".to_owned()
            ));
            return;
        }

        let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }.max(1);
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }.max(1);
        let mut state = Box::new(OverlayState {
            points: Vec::new(),
            virtual_left,
            virtual_top,
        });
        let state_ptr = (&mut *state) as *mut OverlayState;
        let window = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                CLASS_NAME.as_ptr(),
                WINDOW_TITLE.as_ptr(),
                WS_POPUP,
                virtual_left,
                virtual_top,
                width,
                height,
                null_mut(),
                null_mut(),
                instance,
                state_ptr.cast::<c_void>(),
            )
        };
        if window.is_null() {
            let _ = ready.send(Err("O Windows não criou a janela do overlay.".to_owned()));
            return;
        }
        let transparent_black: COLORREF = 0;
        if unsafe { SetLayeredWindowAttributes(window, transparent_black, 0, LWA_COLORKEY) } == 0 {
            unsafe { DestroyWindow(window) };
            let _ = ready.send(Err(
                "Não foi possível tornar o fundo do overlay transparente.".to_owned(),
            ));
            return;
        }
        let _ = ready.send(Ok(()));
        let mut visible = false;

        loop {
            loop {
                let mut message: MSG = unsafe { zeroed() };
                if unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } == 0 {
                    break;
                }
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }

            match receiver.recv_timeout(Duration::from_millis(16)) {
                Ok(OverlayCommand::Show(show)) => {
                    visible = show;
                    if show {
                        unsafe {
                            SetWindowPos(
                                window,
                                HWND_TOPMOST,
                                virtual_left,
                                virtual_top,
                                width,
                                height,
                                SWP_NOACTIVATE | SWP_SHOWWINDOW,
                            );
                            ShowWindow(window, SW_SHOWNOACTIVATE);
                            InvalidateRect(window, null(), 1);
                        }
                    } else {
                        unsafe { ShowWindow(window, SW_HIDE) };
                    }
                }
                Ok(OverlayCommand::Points(points)) => {
                    state.points = points;
                    if visible {
                        unsafe { InvalidateRect(window, null(), 1) };
                    }
                }
                Ok(OverlayCommand::Close) | Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {}
            }
        }
        unsafe { DestroyWindow(window) };
    }

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_NCCREATE {
            let create = unsafe {
                &*(lparam as *const windows_sys::Win32::UI::WindowsAndMessaging::CREATESTRUCTW)
            };
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    window,
                    windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    create.lpCreateParams as isize,
                );
            }
        }
        match message {
            WM_NCHITTEST => -1,
            WM_PAINT => {
                let mut paint: PAINTSTRUCT = unsafe { zeroed() };
                let dc = unsafe { BeginPaint(window, &mut paint) };
                let state_ptr = unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(
                        window,
                        windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA,
                    ) as *mut OverlayState
                };
                if !state_ptr.is_null() {
                    unsafe { draw_points(dc, &*state_ptr) };
                }
                unsafe { EndPaint(window, &paint) };
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    unsafe fn draw_points(dc: HDC, state: &OverlayState) {
        let pen = unsafe { CreatePen(PS_SOLID, 3, rgb(255, 190, 55)) };
        let old_pen: HGDIOBJ = unsafe { SelectObject(dc, pen) };
        let null_brush = unsafe { GetStockObject(NULL_BRUSH as i32) };
        let old_brush = unsafe { SelectObject(dc, null_brush) };
        unsafe {
            SetBkMode(dc, TRANSPARENT as i32);
            SetTextColor(dc, rgb(255, 255, 255));
        }
        for (index, point) in state.points.iter().enumerate() {
            let x = point.x - state.virtual_left;
            let y = point.y - state.virtual_top;
            unsafe {
                Ellipse(dc, x - 12, y - 12, x + 12, y + 12);
                MoveToEx(dc, x - 18, y, null_mut());
                LineTo(dc, x + 18, y);
                MoveToEx(dc, x, y - 18, null_mut());
                LineTo(dc, x, y + 18);
                let label: Vec<u16> = format!("{}  ({}, {})", index + 1, point.x, point.y)
                    .encode_utf16()
                    .collect();
                TextOutW(dc, x + 20, y - 9, label.as_ptr(), label.len() as i32);
            }
        }
        unsafe {
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            DeleteObject(pen);
        }
    }

    const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
        (red as COLORREF) | ((green as COLORREF) << 8) | ((blue as COLORREF) << 16)
    }
}
