use anyhow::Result;
use http::{HeaderMap, HeaderValue, Response, StatusCode, Version};
use hyper::body::{to_bytes, Body, Bytes};
use warp::Reply;

#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub status: StatusCode,
    pub version: Version,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Bytes,
}

impl CachedResponse {
    pub async fn from_reply<T>(reply: T) -> Result<Self>
    where
        T: Reply,
    {
        let mut response = reply.into_response();
        Ok(CachedResponse {
            status: response.status(),
            version: response.version(),
            headers: response.headers().clone(),
            body: to_bytes(response.body_mut()).await?,
        })
    }
}

impl Reply for CachedResponse {
    fn into_response(self) -> warp::reply::Response {
        match Response::builder()
            .status(self.status)
            .version(self.version)
            .body(Body::from(self.body))
        {
            Ok(mut response) => {
                let headers = response.headers_mut();
                for (header, value) in self.headers.iter() {
                    headers.insert(header, value.clone());
                }
                response
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
