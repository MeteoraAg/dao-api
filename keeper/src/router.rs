use crate::core::Core;

use hyper::{Body, Request, Response, StatusCode};
use log::debug;
use routerify::prelude::*;
use routerify::{Middleware, RequestInfo, Router};
use std::convert::Infallible;
use std::sync::Arc;

pub fn router(core: Arc<Core>) -> Router<Body, Infallible> {
    Router::builder()
        .data(core)
        .middleware(Middleware::pre(logger))
        .get("/version", get_version)
        .get("/gauge_factory", get_gauge_factory)
        .get("/gauges", get_gauges)
        .get("/epoch/:epoch", get_epoch)
        .err_handler_with_info(error_handler)
        .build()
        .unwrap()
}

async fn get_version(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("0.1")))
}

async fn get_gauge_factory(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();
    match serde_json::to_string(&core.get_gauge_factory()) {
        Ok(res) => Ok(Response::new(Body::from(res))),
        Err(_) => Ok(Response::new(Body::from("Cannot encode gauge factory"))),
    }
}

async fn get_gauges(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();
    match serde_json::to_string(&core.get_gauges()) {
        Ok(res) => Ok(Response::new(Body::from(res))),
        Err(_) => Ok(Response::new(Body::from("Cannot encode gauges"))),
    }
}

async fn get_epoch(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();

    let epoch = match parse_epoch(&req) {
        Ok(value) => value,
        Err(_) => {
            return Ok(Response::new(Body::from("Cannot decode epoch")));
        }
    };

    match core.get_epoch_info(epoch).await {
        Ok(info) => match serde_json::to_string(&info) {
            Ok(res) => Ok(Response::new(Body::from(res))),
            Err(_) => Ok(Response::new(Body::from("Cannot encode epoch info"))),
        },
        Err(err) => Ok(Response::new(Body::from("Cannot get epoch info"))),
    }
}

fn parse_epoch(req: &Request<Body>) -> anyhow::Result<u64> {
    let epoch = req
        .param("epoch")
        .ok_or(anyhow::Error::msg("Cannot get epoch"))?;
    let epoch = epoch.parse::<u64>()?;
    Ok(epoch)
}

async fn error_handler(err: routerify::RouteError, _: RequestInfo) -> Response<Body> {
    debug!("{}", err);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(format!("Something went wrong: {}", err)))
        .unwrap()
}

async fn logger(req: Request<Body>) -> Result<Request<Body>, Infallible> {
    debug!(
        "{} {} {}",
        req.remote_addr(),
        req.method(),
        req.uri().path()
    );
    Ok(req)
}