//! Platform signal handling for the program runtime.

use crate::InterruptMsg;
use crate::event::EventSender;

/// Spawns listeners for terminal interrupt/terminate signals.
pub fn spawn_interrupt_listener(
    event_tx: EventSender,
    enabled: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !enabled {
            futures::future::pending::<()>().await;
            return;
        }

        #[cfg(unix)]
        {
            use crate::event::{Msg, QuitMsg};
            use tokio::signal::unix::{SignalKind, signal};

            let mut sigint = signal(SignalKind::interrupt()).ok();
            let mut sigterm = signal(SignalKind::terminate()).ok();

            tokio::select! {
                _ = async {
                    if let Some(ref mut s) = sigint {
                        s.recv().await;
                    } else {
                        futures::future::pending().await;
                    }
                } => {
                    let _ = event_tx.send(Box::new(InterruptMsg) as Msg);
                }
                _ = async {
                    if let Some(ref mut s) = sigterm {
                        s.recv().await;
                    } else {
                        futures::future::pending().await;
                    }
                } => {
                    let _ = event_tx.send(Box::new(QuitMsg) as Msg);
                }
            }
        }

        #[cfg(not(unix))]
        {
            use crate::event::Msg;
            let _ = tokio::signal::ctrl_c().await;
            let _ = event_tx.send(Box::new(InterruptMsg) as Msg);
        }
    })
}

/// Spawns a SIGWINCH listener that emits [`WindowSizeMsg`] (Unix only).
#[cfg(unix)]
pub fn spawn_resize_listener(
    event_tx: EventSender,
    enabled: bool,
) -> Option<tokio::task::JoinHandle<()>> {
    if !enabled {
        return None;
    }

    Some(tokio::spawn(async move {
        use crate::event::{Msg, WindowSizeMsg};
        use tokio::signal::unix::{SignalKind, signal};

        let Ok(mut sigwinch) = signal(SignalKind::window_change()) else {
            return;
        };

        while sigwinch.recv().await.is_some() {
            if let Ok(size) = term::open_tty_size() {
                let _ = event_tx.send(Box::new(WindowSizeMsg {
                    width: size.width,
                    height: size.height,
                }) as Msg);
            }
        }
    }))
}

/// Windows does not receive SIGWINCH; resize events come from crossterm.
#[cfg(not(unix))]
pub fn spawn_resize_listener(
    _event_tx: EventSender,
    _enabled: bool,
) -> Option<tokio::task::JoinHandle<()>> {
    None
}

/// Whether the platform supports job-control suspend/resume.
pub const SUSPEND_SUPPORTED: bool = cfg!(unix);

/// Suspends the process until the shell sends SIGCONT (Unix only).
#[cfg(unix)]
pub fn suspend_process() {
    use std::sync::atomic::{AtomicBool, Ordering, fence};

    static CONT: AtomicBool = AtomicBool::new(false);

    extern "C" fn on_sigcont(_: libc::c_int) {
        CONT.store(true, Ordering::SeqCst);
    }

    unsafe {
        libc::signal(libc::SIGCONT, on_sigcont as libc::sighandler_t);
        CONT.store(false, Ordering::SeqCst);
        libc::kill(0, libc::SIGTSTP);
        while !CONT.load(Ordering::SeqCst) {
            libc::pause();
        }
        fence(Ordering::SeqCst);
        libc::signal(libc::SIGCONT, libc::SIG_DFL);
    }
}

/// No-op on platforms without job control.
#[cfg(not(unix))]
pub fn suspend_process() {}
