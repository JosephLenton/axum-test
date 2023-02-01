use ::anyhow::Context;
use ::anyhow::Result;
use ::hyper::body::to_bytes;
use ::hyper::body::Body;
use ::hyper::body::Bytes;
use ::hyper::body::HttpBody;
use ::hyper::client::ResponseFuture;
use ::hyper::header;
use ::hyper::http::response::Parts;
use ::hyper::http::Method;
use ::hyper::http::Request;
use ::hyper::Client;
use ::serde::Serialize;
use ::serde_json::to_vec as json_to_vec;
use ::std::convert::AsRef;
use ::std::fmt::Debug;
use ::std::fmt::Formatter;
use ::std::fmt::Result as FmtResult;
use ::std::future::Future;
use ::std::pin::Pin;
use ::std::task::Context as TaskContext;
use ::std::task::Poll;

use crate::TestResponse;

/// This contains the response from the server.
///
/// Inside are the contents of the response, the status code, and some
/// debugging information.
///
/// You can get the contents out as it's raw string, or deserialise it.
/// One can also also use the `assert_*` functions to test against the
/// response.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TestRequestFuture {
    method: Method,
    path: String,
    body: Option<Body>,

    /// This is what we use for logging for when we display the path to the user.
    debug_path: String,

    is_expecting_failure: bool,

    state: Option<RequestState>,
}

enum RequestState {
    Sending(Pin<Box<ResponseFuture>>, String),
    ReadingResponse(
        Box<Parts>,
        Pin<Box<dyn Future<Output = Result<Bytes, <Body as HttpBody>::Error>>>>,
        String,
    ),
    Complete,
}

impl Debug for RequestState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Sending(inner_future, debug_path) => {
                write!(
                    f,
                    "RequestState::Sending({:?}, {:?})",
                    inner_future, debug_path
                )
            }
            Self::ReadingResponse(parts, _, debug_path) => {
                write!(
                    f,
                    "RequestState::ReadingResponse({:?}, inner_future, {:?})",
                    parts, debug_path
                )
            }
            Self::Complete => {
                write!(f, "RequestState::Complete")
            }
        }
    }
}

impl TestRequestFuture {
    pub(crate) fn new(method: Method, path: String, debug_path: String) -> Self {
        Self {
            method,
            path,
            body: None,
            debug_path,
            is_expecting_failure: false,
            state: None,
        }
    }

    fn build_hyper_response(&mut self) -> Result<(ResponseFuture, String)> {
        let mut maybe_body = None;
        ::std::mem::swap(&mut self.body, &mut maybe_body);
        let body = maybe_body.unwrap_or(Body::empty());

        let request = Request::builder()
            .uri(&self.path)
            .header(header::CONTENT_TYPE, "application/json")
            .method(self.method.clone())
            .body(body)
            .with_context(|| {
                format!(
                    "Expect valid hyper Request to be built on request to {}",
                    self.debug_path
                )
            })?;

        let hyper_response = Client::new().request(request);

        let response_with_path = (hyper_response, self.debug_path.clone());

        Ok(response_with_path)
    }
}

impl Future for TestRequestFuture {
    type Output = TestResponse;

    fn poll(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        loop {
            let debug_path = &self.debug_path;
            let state = &mut self.state;

            match state {
                // This is what gets called on first `.await`.
                // It kicks off the Hyper Request, starting the processing.
                None => {
                    let (hyper_response, debug_path) = self
                        .build_hyper_response()
                        .with_context(|| {
                            format!(
                                "Should be able to build request using Hyper to {}",
                                self.debug_path
                            )
                        })
                        .unwrap();
                    let pinned_future = Box::pin(hyper_response);
                    self.state = Some(RequestState::Sending(pinned_future, debug_path));
                }

                // This chunk deals with listening to the Hyper Request.
                Some(RequestState::Sending(inner_future, _)) => {
                    let poll = inner_future.as_mut().poll(cx);

                    match poll {
                        Poll::Pending => {
                            return Poll::Pending;
                        }

                        // Turn from a Hyper Request to reading in the data.
                        // This will then loop into the reading response section.
                        Poll::Ready(response_result) => {
                            let response = response_result
                                .with_context(|| {
                                    format!(
                                        "Expect Hyper Response to succeed on request to {}",
                                        debug_path
                                    )
                                })
                                .unwrap();

                            let (parts, response_body) = response.into_parts();
                            let response_bytes_future = to_bytes(response_body);

                            let pinned_future = Box::pin(response_bytes_future);
                            self.state = Some(RequestState::ReadingResponse(
                                Box::new(parts),
                                pinned_future,
                                debug_path.clone(),
                            ));
                        }
                    }
                }

                Some(RequestState::ReadingResponse(parts, inner_future, _)) => {
                    let poll = inner_future.as_mut().poll(cx);

                    match poll {
                        Poll::Pending => {
                            return Poll::Pending;
                        }

                        // Turn from a Hyper Request to reading in the data.
                        // This will then loop into the reading response section.
                        Poll::Ready(response_result) => {
                            let response_bytes = response_result
                                .with_context(|| {
                                    format!("Error unwrapping response to {}", debug_path)
                                })
                                .unwrap();

                            let mut test_response =
                                TestResponse::new(debug_path.clone(), response_bytes, parts.status);

                            self.state = Some(RequestState::Complete);

                            // Assert if ok or not.
                            if self.is_expecting_failure {
                                test_response = test_response.assert_status_not_ok();
                            } else {
                                test_response = test_response.assert_status_ok();
                            }

                            return Poll::Ready(test_response);
                        }
                    }
                }

                Some(RequestState::Complete) => {
                    panic!(
                        "Polling future when this is already completed, on request to {}",
                        self.debug_path
                    );
                }
            }
        }
    }
}
