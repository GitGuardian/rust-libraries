use bytes::Bytes;
use gitguardian_api::models::page::Page;
use gitguardian_api::{ApiCall, ApiConfig, Paginated};
use http::Response;

use crate::error::Error;

const BODY_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Client {
    agent: ::ureq::Agent,
    config: ApiConfig,
}

impl Client {
    pub fn new(config: ApiConfig) -> Self {
        Self::with_agent(
            config,
            ::ureq::Agent::config_builder().http_status_as_error(false).build().new_agent(),
        )
    }

    /// The agent is recommended to be built with `http_status_as_error(false)`, so that the
    /// API's error responses are parsed into [`Error::Api`] rather than reported as
    /// transport errors.
    pub fn with_agent(config: ApiConfig, agent: ::ureq::Agent) -> Self {
        Self { agent, config }
    }

    pub fn config(&self) -> &ApiConfig {
        &self.config
    }

    pub fn send<C: ApiCall>(&self, call: &C) -> Result<C::Output, Error> {
        let request = call.build(&self.config)?;
        let response = self.agent.run(request.map(|body| body.to_vec()))?;

        let (parts, mut body) = response.into_parts();
        let bytes = body.with_config().limit(BODY_LIMIT).read_to_vec()?;

        Ok(call.parse(Response::from_parts(parts, Bytes::from(bytes)))?)
    }

    pub fn paginate<T, C>(&self, call: C) -> Pages<'_, C>
    where
        C: Paginated<Output = Page<T>>,
    {
        Pages { client: self, call: Some(call) }
    }
}

pub struct Pages<'a, C> {
    client: &'a Client,
    call: Option<C>,
}

impl<T, C> Iterator for Pages<'_, C>
where
    C: Paginated<Output = Page<T>>,
{
    type Item = Result<Page<T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut call = self.call.take()?;
        match self.client.send(&call) {
            Ok(page) => {
                if let Some(cursor) = page.next.clone() {
                    call.set_cursor(cursor);
                    self.call = Some(call);
                }
                Some(Ok(page))
            }
            Err(error) => Some(Err(error)),
        }
    }
}
