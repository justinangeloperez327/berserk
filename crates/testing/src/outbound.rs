use berserk_client::{ClientError, ErrorKind, HttpClient, Request, Response, Result};
use std::{collections::VecDeque, sync::Mutex};

pub struct FakeHttpClient {
    responses: Mutex<VecDeque<Result<Response>>>,
    requests: Mutex<Vec<Request>>,
}
impl FakeHttpClient {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(VecDeque::new()),
            requests: Mutex::new(Vec::new()),
        }
    }
    pub fn push_response(&self, response: Response) {
        self.responses
            .lock()
            .expect("fake HTTP response lock poisoned")
            .push_back(Ok(response));
    }
    pub fn push_error(&self, error: ClientError) {
        self.responses
            .lock()
            .expect("fake HTTP response lock poisoned")
            .push_back(Err(error));
    }
    pub fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .expect("fake HTTP request lock poisoned")
            .clone()
    }
    pub fn request_count(&self) -> usize {
        self.requests
            .lock()
            .expect("fake HTTP request lock poisoned")
            .len()
    }
    pub fn take_requests(&self) -> Vec<Request> {
        std::mem::take(
            &mut *self
                .requests
                .lock()
                .expect("fake HTTP request lock poisoned"),
        )
    }
}
impl Default for FakeHttpClient {
    fn default() -> Self {
        Self::new()
    }
}
impl HttpClient for FakeHttpClient {
    fn send(&self, request: Request) -> Result<Response> {
        self.requests
            .lock()
            .map_err(|_| ClientError::new(ErrorKind::Transport, "fake HTTP request lock poisoned"))?
            .push(request);
        self.responses
            .lock()
            .map_err(|_| {
                ClientError::new(ErrorKind::Transport, "fake HTTP response lock poisoned")
            })?
            .pop_front()
            .unwrap_or_else(|| {
                Err(ClientError::new(
                    ErrorKind::Transport,
                    "fake HTTP response queue is empty",
                ))
            })
    }
}
