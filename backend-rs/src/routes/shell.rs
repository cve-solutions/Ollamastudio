use std::ffi::CString;
use std::os::unix::io::RawFd;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use nix::pty::{openpty, OpenptyResult};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::WaitPidFlag;
use nix::unistd::{close, dup2, execvpe, fork, setsid, ForkResult, Pid};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::state::AppState;

// ── Control messages from the frontend ──

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ControlMessage {
    Resize { rows: u16, cols: u16 },
    Signal { signal: String },
}

// ── WebSocket upgrade handler ──

/// GET /ws -- upgrade to WebSocket and spawn a PTY shell session.
pub async fn shell_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let cwd = state.config.workspace_root.clone();
    ws.on_upgrade(move |socket| handle_shell(socket, cwd, state))
}

async fn handle_shell(
    socket: WebSocket,
    cwd: std::path::PathBuf,
    state: AppState,
) {
    state.debug.log("INFO", "ws", "Nouvelle connexion WebSocket shell");

    match run_pty_session(socket, cwd, &state).await {
        Ok(()) => {
            state.debug.log("INFO", "ws", "Session shell terminee proprement");
        }
        Err(e) => {
            tracing::error!("Shell session error: {e}");
            state
                .debug
                .log("ERROR", "ws", &format!("Erreur session shell: {e}"));
        }
    }
}

/// Convert an OwnedFd to a raw fd, consuming ownership so the fd is not auto-closed.
fn owned_to_raw(fd: std::os::fd::OwnedFd) -> RawFd {
    use std::os::fd::IntoRawFd;
    fd.into_raw_fd()
}

async fn run_pty_session(
    socket: WebSocket,
    cwd: std::path::PathBuf,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Open a PTY pair
    let OpenptyResult { master, slave } = openpty(None, None)?;
    let master_fd: RawFd = owned_to_raw(master);
    let slave_fd: RawFd = owned_to_raw(slave);

    // Fork the child process
    // SAFETY: we immediately exec in the child; the parent only uses master_fd.
    let fork_result = unsafe { fork()? };

    match fork_result {
        ForkResult::Child => {
            // ── Child process ──
            // Close the master side
            let _ = close(master_fd);

            // Create a new session and set the controlling terminal
            setsid().ok();
            unsafe {
                libc::ioctl(slave_fd, libc::TIOCSCTTY as libc::c_ulong, 0);
            }

            // Redirect stdin/stdout/stderr to the slave PTY
            dup2(slave_fd, libc::STDIN_FILENO).ok();
            dup2(slave_fd, libc::STDOUT_FILENO).ok();
            dup2(slave_fd, libc::STDERR_FILENO).ok();
            if slave_fd > libc::STDERR_FILENO {
                let _ = close(slave_fd);
            }

            // Change to workspace directory
            let cwd_c = CString::new(cwd.to_string_lossy().as_bytes().to_vec())
                .unwrap_or_else(|_| CString::new("/tmp").unwrap());
            unsafe {
                libc::chdir(cwd_c.as_ptr());
            }

            // Build environment
            let env_vars: Vec<CString> = vec![
                CString::new(format!(
                    "HOME={}",
                    cwd.to_string_lossy()
                ))
                .unwrap(),
                CString::new("TERM=xterm-256color").unwrap(),
                CString::new("COLORTERM=truecolor").unwrap(),
                CString::new("LANG=en_US.UTF-8").unwrap(),
                CString::new(
                    "PS1=\\[\\033[01;32m\\]\\u@ollamastudio\\[\\033[00m\\]:\\[\\033[01;34m\\]\\w\\[\\033[00m\\]\\$ ",
                )
                .unwrap(),
                CString::new(format!(
                    "PATH={}",
                    std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into())
                ))
                .unwrap(),
            ];

            let bash = CString::new("/bin/bash").unwrap();
            let args = [bash.clone(), CString::new("--login").unwrap()];

            // execvpe replaces the child process with bash
            let _ = execvpe(&bash, &args, &env_vars);
            // If execvpe fails, exit immediately
            unsafe {
                libc::_exit(1);
            }
        }
        ForkResult::Parent { child } => {
            // ── Parent process ──
            // Close slave side -- we only use master
            let _ = close(slave_fd);

            state
                .debug
                .log("INFO", "ws", &format!("Shell PTY started, PID={child}, FD={master_fd}"));

            let result =
                io_bridge(socket, master_fd, child, state).await;

            // Cleanup: kill child, waitpid, close fd
            cleanup(child, master_fd, state);

            result
        }
    }
}

/// Read from a raw fd using libc, returning bytes read or 0 on error/EOF.
fn raw_read(fd: RawFd, buf: &mut [u8]) -> isize {
    unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) }
}

/// Write to a raw fd using libc.
fn raw_write(fd: RawFd, buf: &[u8]) -> isize {
    unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) }
}

/// Write all bytes to a file descriptor, handling partial writes.
fn write_all(fd: RawFd, mut data: &[u8]) {
    while !data.is_empty() {
        let n = raw_write(fd, data);
        if n <= 0 {
            break;
        }
        data = &data[n as usize..];
    }
}

/// Bidirectional bridge between WebSocket and PTY.
async fn io_bridge(
    mut socket: WebSocket,
    master_fd: RawFd,
    child_pid: Pid,
    state: &AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Channel for PTY -> WS data
    let (pty_tx, mut pty_rx) = mpsc::channel::<Vec<u8>>(64);

    // Spawn a blocking task to read from the PTY fd
    let pty_reader = tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        loop {
            // Use poll(2) with a short timeout so we can detect channel closure
            let mut pfd = libc::pollfd {
                fd: master_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd as *mut libc::pollfd, 1, 100) };

            if ret == 0 {
                // Timeout -- check if receiver still alive
                if pty_tx.is_closed() {
                    break;
                }
                continue;
            }
            if ret < 0 {
                break; // poll error
            }

            let n = raw_read(master_fd, &mut buf);
            if n <= 0 {
                break; // EOF or error
            }
            if pty_tx.blocking_send(buf[..n as usize].to_vec()).is_err() {
                break; // WS side gone
            }
        }
    });

    // Main loop: multiplex PTY reads and WS reads
    loop {
        tokio::select! {
            // Data from PTY -> send to WebSocket
            Some(data) = pty_rx.recv() => {
                if socket.send(Message::Binary(data.into())).await.is_err() {
                    break; // WS closed
                }
            }
            // Data from WebSocket -> write to PTY or handle control
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        if data.starts_with(b"{") {
                            handle_control(&data, master_fd, child_pid, state);
                        } else {
                            let bytes = data.to_vec();
                            let fd = master_fd;
                            tokio::task::spawn_blocking(move || {
                                write_all(fd, &bytes);
                            }).await.ok();
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        let data = text.as_bytes();
                        if data.starts_with(b"{") {
                            handle_control(data, master_fd, child_pid, state);
                        } else {
                            let bytes = data.to_vec();
                            let fd = master_fd;
                            tokio::task::spawn_blocking(move || {
                                write_all(fd, &bytes);
                            }).await.ok();
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break; // Client disconnected
                    }
                    Some(Ok(Message::Ping(d))) => {
                        let _ = socket.send(Message::Pong(d)).await;
                    }
                    _ => {}
                }
            }
        }
    }

    // Drop the receiver so the blocking reader task exits
    drop(pty_rx);
    pty_reader.abort();

    Ok(())
}

/// Handle JSON control messages (resize, signal).
fn handle_control(raw: &[u8], master_fd: RawFd, child_pid: Pid, state: &AppState) {
    let ctrl: ControlMessage = match serde_json::from_slice(raw) {
        Ok(c) => c,
        Err(_) => return, // Not a valid control message -- ignore
    };

    match ctrl {
        ControlMessage::Resize { rows, cols } => {
            let ws = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let ret = unsafe { libc::ioctl(master_fd, libc::TIOCSWINSZ, &ws) };
            if ret == 0 {
                state
                    .debug
                    .log("DEBUG", "ws", &format!("Terminal resize {rows}x{cols}"));
            } else {
                state.debug.log(
                    "WARN",
                    "ws",
                    &format!("Echec resize terminal: ioctl returned {ret}"),
                );
            }
        }
        ControlMessage::Signal { signal: sig_name } => {
            let sig = match sig_name.as_str() {
                "SIGINT" => Signal::SIGINT,
                "SIGTERM" => Signal::SIGTERM,
                "SIGKILL" => Signal::SIGKILL,
                "SIGQUIT" => Signal::SIGQUIT,
                "SIGTSTP" => Signal::SIGTSTP,
                "SIGCONT" => Signal::SIGCONT,
                _ => Signal::SIGINT,
            };
            match signal::kill(child_pid, sig) {
                Ok(()) => {
                    state.debug.log(
                        "DEBUG",
                        "ws",
                        &format!("Signal {sig_name} envoye au PID {child_pid}"),
                    );
                }
                Err(e) => {
                    state.debug.log(
                        "WARN",
                        "ws",
                        &format!("Processus {child_pid} introuvable pour signal {sig_name}: {e}"),
                    );
                }
            }
        }
    }
}

/// Clean up the child process and close the master fd.
fn cleanup(child_pid: Pid, master_fd: RawFd, state: &AppState) {
    state.debug.log(
        "DEBUG",
        "ws",
        &format!("Nettoyage session shell PID={child_pid} FD={master_fd}"),
    );

    // Send SIGTERM
    if let Err(e) = signal::kill(child_pid, Signal::SIGTERM) {
        state.debug.log(
            "WARN",
            "ws",
            &format!("Cleanup kill PID {child_pid}: {e}"),
        );
    }

    // Reap the child (non-blocking)
    match nix::sys::wait::waitpid(child_pid, Some(WaitPidFlag::WNOHANG)) {
        Ok(_) => {}
        Err(e) => {
            state.debug.log(
                "WARN",
                "ws",
                &format!("Cleanup waitpid PID {child_pid}: {e}"),
            );
        }
    }

    // Close master fd
    if let Err(e) = close(master_fd) {
        state.debug.log(
            "WARN",
            "ws",
            &format!("Cleanup close FD {master_fd}: {e}"),
        );
    }
}
