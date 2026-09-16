use super::connection::serve;
use crate::{App, ShutdownHandle};
use std::{
    io,
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

/// Bound listener. Calling run starts workers and accepts connections.
pub struct Server {
    listener: TcpListener,
    app: Arc<App>,
    shutdown: ShutdownHandle,
    stats: super::ServerStats,
}
impl Server {
    pub(crate) fn bind(app: App, address: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            app: Arc::new(app),
            shutdown: ShutdownHandle::new(),
            stats: Default::default(),
        })
    }
    pub fn stats(&self) -> super::ServerStats {
        self.stats.clone()
    }
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown.clone()
    }

    /// Stop accepting on shutdown, then drain accepted work. User handlers must return.
    pub fn run(self) -> io::Result<()> {
        let (sender, receiver) =
            mpsc::sync_channel::<(std::net::TcpStream, Instant)>(self.app.config().queue_capacity);
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::new();
        for index in 0..self.app.config().workers {
            let receiver = receiver.clone();
            let app = self.app.clone();
            let stats = self.stats.clone();
            let shutdown = self.shutdown.clone();
            match thread::Builder::new()
                .name(format!("framework-worker-{index}"))
                .spawn(move || loop {
                    let work = match receiver.lock() {
                        Ok(queue) => queue.recv(),
                        Err(_) => break,
                    };
                    match work {
                        Ok((stream, accepted)) => {
                            let outcome =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    serve(stream, accepted, &app, &shutdown)
                                }));
                            stats.finish(matches!(outcome, Ok(true)));
                        }
                        Err(_) => break,
                    }
                }) {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    drop(sender);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(error);
                }
            }
        }
        let mut result = Ok(());
        while !self.shutdown.is_requested() {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if self.shutdown.is_requested() {
                        drop(stream);
                        break;
                    }
                    self.stats.accept();
                    // Never block the listener writing an overload response.
                    match sender.try_send((stream, Instant::now())) {
                        Ok(()) => {}
                        Err(mpsc::TrySendError::Full(work)) => {
                            self.stats.reject();
                            drop(work);
                        }
                        Err(mpsc::TrySendError::Disconnected(work)) => {
                            self.stats.reject();
                            drop(work);
                            result = Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "worker queue disconnected",
                            ));
                            break;
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }
        }
        // Release the listening port before draining queued connections.
        drop(self.listener);
        drop(sender);
        for worker in workers {
            if worker.join().is_err() && result.is_ok() {
                result = Err(io::Error::other("server worker panicked"));
            }
        }
        result
    }
}
