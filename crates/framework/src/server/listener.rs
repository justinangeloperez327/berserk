use super::hyper_connection::serve;
use crate::{App, ShutdownHandle};
use std::{
    io,
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, Semaphore},
    task::JoinSet,
};

/// Bound listener. Calling run starts the Tokio runtime and serves HTTP/1 with Hyper.
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
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.app.config().workers)
            .enable_all()
            .build()?;
        runtime.block_on(self.run_async())
    }

    async fn run_async(self) -> io::Result<()> {
        let config = self.app.config().clone();
        let listener = tokio::net::TcpListener::from_std(self.listener)?;
        let (sender, mut receiver) =
            mpsc::channel::<(tokio::net::TcpStream, Instant)>(config.queue_capacity);
        let permits = Arc::new(Semaphore::new(config.workers));

        let app = Arc::clone(&self.app);
        let stats = self.stats.clone();
        let dispatch_stats = self.stats.clone();
        let dispatch_permits = Arc::clone(&permits);
        let dispatcher = tokio::spawn(async move {
            let mut connections = JoinSet::new();

            loop {
                let permit = match Arc::clone(&dispatch_permits).acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => break,
                };
                let Some((stream, accepted)) = receiver.recv().await else {
                    drop(permit);
                    break;
                };

                let app = Arc::clone(&app);
                let stats = dispatch_stats.clone();
                connections.spawn(async move {
                    let _permit = permit;
                    stats.finish(serve(stream, accepted, app).await);
                });

                while let Some(result) = connections.try_join_next() {
                    if result.is_err() {
                        dispatch_stats.finish(false);
                    }
                }
            }

            while let Some(result) = connections.join_next().await {
                if result.is_err() {
                    dispatch_stats.finish(false);
                }
            }
        });

        let mut result = Ok(());
        while !self.shutdown.is_requested() {
            let accepted = tokio::time::timeout(Duration::from_millis(5), listener.accept()).await;
            match accepted {
                Ok(Ok((stream, _))) => {
                    if self.shutdown.is_requested() {
                        drop(stream);
                        break;
                    }
                    stats.accept();
                    let work = (stream, Instant::now());
                    match sender.try_send(work) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full((stream, _))) => {
                            stats.reject();
                            drop(stream);
                        }
                        Err(mpsc::error::TrySendError::Closed((stream, _))) => {
                            stats.reject();
                            drop(stream);
                            result = Err(io::Error::new(
                                io::ErrorKind::BrokenPipe,
                                "connection queue disconnected",
                            ));
                            break;
                        }
                    }
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => {
                    result = Err(error);
                    break;
                }
                Err(_) => {}
            }
        }

        // Release the listening port before draining queued and active connections.
        drop(listener);
        drop(sender);
        if dispatcher.await.is_err() && result.is_ok() {
            result = Err(io::Error::other("server dispatcher panicked"));
        }
        result
    }
}
