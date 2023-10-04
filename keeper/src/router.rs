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
        .get("/latest_epoches", get_latest_epoches)
        .get("/pools", get_all_pools)
        .get("/quarries", get_all_quarries)
        .err_handler_with_info(error_handler)
        .build()
        .unwrap()
}

fn get_response_builder() -> hyper::http::response::Builder {
    Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .header(
            "Access-Control-Allow-Methods",
            "PUT, GET, POST, OPTIONS, DELETE, PATCH",
        )
}
async fn get_version(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let builder = get_response_builder();
    let response = builder.body(Body::from("0.1"));
    match response {
        Ok(value) => Ok(value),
        Err(_) => Ok(Response::new(Body::from("Internal server"))),
    }
}

async fn get_gauge_factory(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();
    match serde_json::to_string(&core.get_gauge_factory()) {
        Ok(res) => {
            let builder = get_response_builder();
            Ok(builder.body(Body::from(res)).unwrap())
        }
        Err(_) => Ok(Response::new(Body::from("Cannot encode gauge factory"))),
    }
}

async fn get_gauges(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();
    match serde_json::to_string(&core.get_gauges()) {
        Ok(res) => {
            let builder = get_response_builder();
            Ok(builder.body(Body::from(res)).unwrap())
        }
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
            Ok(res) => {
                let builder = get_response_builder();
                Ok(builder.body(Body::from(res)).unwrap())
            }
            Err(_) => Ok(Response::new(Body::from("Cannot encode epoch info"))),
        },
        Err(err) => Ok(Response::new(Body::from("Cannot get epoch info"))),
    }
}

async fn get_latest_epoches(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();

    match core.get_latest_epoches().await {
        Ok(info) => match serde_json::to_string(&info) {
            Ok(res) => {
                let builder = get_response_builder();
                Ok(builder.body(Body::from(res)).unwrap())
            }
            Err(_) => Ok(Response::new(Body::from("Cannot encode epoch info"))),
        },
        Err(err) => Ok(Response::new(Body::from("Cannot get epoch info"))),
    }
}

async fn get_all_pools(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();

    match core.get_all_pools().await {
        Ok(info) => match serde_json::to_string(&info) {
            Ok(res) => {
                let builder = get_response_builder();
                Ok(builder.body(Body::from(res)).unwrap())
            }
            Err(_) => Ok(Response::new(Body::from("Cannot encode epoch info"))),
        },
        Err(err) => Ok(Response::new(Body::from("Cannot get epoch info"))),
    }
}

async fn get_all_quarries(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let core = req.data::<Arc<Core>>().unwrap();

    match core.get_all_quarries().await {
        Ok(info) => match serde_json::to_string(&info) {
            Ok(res) => {
                let builder = get_response_builder();
                Ok(builder.body(Body::from(res)).unwrap())
            }
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
