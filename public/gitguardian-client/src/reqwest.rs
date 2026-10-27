use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use gitguardian_api::models::page::Page;
use gitguardian_api::{ApiCall, ApiConfig, Paginated};
use http::Response;
use tower_service::Service;

use crate::error::Error;

#[derive(Clone, Debug)]
pub struct Client<S = ::reqwest::Client> {
    http: S,
    config: ApiConfig,
}

impl Client<::reqwest::Client> {
    pub fn new(config: ApiConfig) -> Self {
        Self::with_http_client(config, ::reqwest::Client::new())
    }
}

impl<S> Client<S> {
    pub fn with_http_client(config: ApiConfig, http: S) -> Self {
        Self { http, config }
    }

    pub fn config(&self) -> &ApiConfig {
        &self.config
    }
}

impl<S> Client<S>
where
    for<'a> &'a S: Service<::reqwest::Request, Response = ::reqwest::Response>,
    for<'a> <&'a S as Service<::reqwest::Request>>::Error:
        std::error::Error + Send + Sync + 'static,
{
    pub async fn send<C: ApiCall>(&self, call: &C) -> Result<C::Output, Error> {
        let request = call.build(&self.config)?;
        let mut http = &self.http;
        std::future::poll_fn(|cx| http.poll_ready(cx))
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;
        let response = http
            .call(::reqwest::Request::try_from(request)?)
            .await
            .map_err(|error| Error::Transport(Box::new(error)))?;

        let status = response.status();
        let headers = response.headers().clone();
        let mut out = Response::new(response.bytes().await?);
        *out.status_mut() = status;
        *out.headers_mut() = headers;

        Ok(call.parse(out)?)
    }

    pub fn paginate<T, C>(&self, call: C) -> Pages<'_, T, C, S>
    where
        C: Paginated<Output = Page<T>> + Send + Sync + Unpin + 'static,
        T: Send + 'static,
    {
        Pages { client: self, state: State::Ready(call) }
    }
}

type Pending<'a, T, C> = Pin<Box<dyn Future<Output = (Result<Page<T>, Error>, C)> + Send + 'a>>;

enum State<'a, T, C> {
    Ready(C),
    InFlight(Pending<'a, T, C>),
    Done,
}

pub struct Pages<'a, T, C, S = ::reqwest::Client> {
    client: &'a Client<S>,
    state: State<'a, T, C>,
}

impl<T, C, S> Stream for Pages<'_, T, C, S>
where
    S: Send + Sync,
    for<'a> &'a S: Service<::reqwest::Request, Response = ::reqwest::Response>,
    for<'a> <&'a S as Service<::reqwest::Request>>::Error:
        std::error::Error + Send + Sync + 'static,
    for<'a> <&'a S as Service<::reqwest::Request>>::Future: Send,
    C: Paginated<Output = Page<T>> + Send + Sync + Unpin + 'static,
    T: Send + 'static,
{
    type Item = Result<Page<T>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let client = this.client;

        let mut pending: Pending<'_, T, C> = match std::mem::replace(&mut this.state, State::Done) {
            State::Ready(call) => Box::pin(async move { (client.send(&call).await, call) }),
            State::InFlight(pending) => pending,
            State::Done => return Poll::Ready(None),
        };

        match pending.as_mut().poll(cx) {
            Poll::Pending => {
                this.state = State::InFlight(pending);
                Poll::Pending
            }
            Poll::Ready((Ok(page), mut call)) => {
                if let Some(cursor) = page.next.clone() {
                    call.set_cursor(cursor);
                    this.state = State::Ready(call);
                }
                Poll::Ready(Some(Ok(page)))
            }
            Poll::Ready((Err(error), _)) => Poll::Ready(Some(Err(error))),
        }
    }
}
