use std::{
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener as StdTcpListener},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use axum::Router;
use tokio::{net::TcpListener, sync::oneshot};

use crate::error::{CodexLagError, Result};

pub const LOOPBACK_BIND_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8787));
const BIND_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const BIND_WAIT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct GatewayHost {
    listen_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayHostStatus {
    pub is_running: bool,
    pub listen_addr: SocketAddr,
}

impl GatewayHost {
    pub fn start(router: Router) -> Result<Self> {
        let listener = bind_loopback_listener()?;
        let listen_addr = listener.local_addr().map_err(|error| {
            CodexLagError::new(format!(
                "failed to read gateway host listen address: {error}"
            ))
        })?;
        listener.set_nonblocking(true).map_err(|error| {
            CodexLagError::new(format!("failed to set nonblocking listener: {error}"))
        })?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (startup_tx, startup_rx) = mpsc::sync_channel(1);

        let task = thread::spawn(move || {
            let startup_result = (|| -> Result<(tokio::runtime::Runtime, TcpListener)> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        CodexLagError::new(format!(
                            "failed to create gateway host runtime: {error}"
                        ))
                    })?;
                let listener = {
                    let _runtime_guard = runtime.enter();
                    TcpListener::from_std(listener).map_err(|error| {
                        CodexLagError::new(format!(
                            "failed to convert gateway host listener to tokio: {error}"
                        ))
                    })?
                };
                Ok((runtime, listener))
            })();

            match startup_result {
                Ok((runtime, listener)) => {
                    let _ = startup_tx.send(Ok(()));
                    runtime.block_on(async move {
                        let server =
                            axum::serve(listener, router).with_graceful_shutdown(async move {
                                let _ = shutdown_rx.await;
                            });
                        let _ = server.await;
                    });
                }
                Err(error) => {
                    let _ = startup_tx.send(Err(error.to_string()));
                }
            }
        });

        match startup_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                listen_addr,
                shutdown_tx: Some(shutdown_tx),
                task: Some(task),
            }),
            Ok(Err(error)) => {
                let _ = task.join();
                Err(CodexLagError::new(error))
            }
            Err(_) => {
                let _ = task.join();
                Err(CodexLagError::new(
                    "gateway host thread exited before confirming startup",
                ))
            }
        }
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn status(&self) -> GatewayHostStatus {
        GatewayHostStatus {
            is_running: self.task.as_ref().is_some_and(|task| !task.is_finished()),
            listen_addr: self.listen_addr,
        }
    }

    pub fn shutdown(mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(task) = self.task.take() {
            task.join()
                .map_err(|_| CodexLagError::new("gateway host task panicked"))?;
        }
        Ok(())
    }
}

impl Drop for GatewayHost {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

fn bind_loopback_listener() -> Result<StdTcpListener> {
    let deadline = Instant::now() + BIND_WAIT_TIMEOUT;
    loop {
        match StdTcpListener::bind(LOOPBACK_BIND_ADDR) {
            Ok(listener) => return Ok(listener),
            Err(error) if error.kind() == ErrorKind::AddrInUse && Instant::now() < deadline => {
                thread::sleep(BIND_RETRY_INTERVAL);
            }
            Err(error) => {
                return Err(CodexLagError::new(format!(
                    "failed to bind loopback gateway host {LOOPBACK_BIND_ADDR}: {error}"
                )));
            }
        }
    }
}
