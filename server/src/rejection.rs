use warp::reject::{InvalidHeader, InvalidQuery, MissingHeader};

use {
    reqwest::StatusCode,
    std::convert::Infallible,
    warp::{reject::Rejection, reply::Reply},
    warp_rate_limit::{RateLimitRejection, add_rate_limit_headers_from_rejection},
};

use crate::osm_proxy::OSMProxyRejection;

pub async fn not_found(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if !rejection.is_not_found() {
        return Err(rejection);
    }

    Ok(warp::reply::with_status(
        "The content you requested does not exist",
        StatusCode::NOT_FOUND,
    )
    .into_response())
}

pub async fn proxy(rejection: Rejection) -> Result<impl Reply, Rejection> {
    let Some(proxy_rejection) = rejection.find::<OSMProxyRejection>() else {
        return Err(rejection);
    };

    Ok(match proxy_rejection {
        OSMProxyRejection::CacheFailure
        | OSMProxyRejection::APIServerFailure
        | OSMProxyRejection::APIResponseUnpackingFailed => {
            warp::reply::with_status("Internal server error", StatusCode::INTERNAL_SERVER_ERROR)
                .into_response()
        }

        OSMProxyRejection::InvalidUserData => {
            warp::reply::with_status("Bad request", StatusCode::BAD_REQUEST).into_response()
        }
    })
}

pub async fn missing_header(rejection: Rejection) -> Result<impl Reply, Rejection> {
    let Some(missing_header_rejection) = rejection.find::<MissingHeader>() else {
        return Err(rejection);
    };

    Ok(warp::reply::with_status(
        format!("Missing {} header", missing_header_rejection.name()),
        StatusCode::BAD_REQUEST,
    )
    .into_response())
}

pub async fn invalid_header(rejection: Rejection) -> Result<impl Reply, Rejection> {
    let Some(invalid_header_rejection) = rejection.find::<InvalidHeader>() else {
        return Err(rejection);
    };

    Ok(warp::reply::with_status(
        format!("Invalid {} header", invalid_header_rejection.name()),
        StatusCode::BAD_REQUEST,
    )
    .into_response())
}

pub async fn invalid_query(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if rejection.find::<InvalidQuery>().is_none() {
        return Err(rejection);
    };

    Ok(warp::reply::with_status("Invalid query", StatusCode::BAD_REQUEST).into_response())
}

pub async fn rate_limit(rejection: Rejection) -> Result<impl Reply, Rejection> {
    let Some(rate_limit_rejection) = rejection.find::<RateLimitRejection>() else {
        return Err(rejection);
    };

    let message = format!(
        "Rate limit exceeded. Try again after {:?}.",
        rate_limit_rejection.retry_after
    );

    let mut response =
        warp::reply::with_status(message, StatusCode::TOO_MANY_REQUESTS).into_response();

    if let Err(e) =
        add_rate_limit_headers_from_rejection(response.headers_mut(), rate_limit_rejection)
    {
        error!("Failed to set rejection rate limit headers due to: {e}")
    }

    Ok(response)
}

pub async fn unknown(rejection: Rejection) -> Result<impl Reply, Infallible> {
    error!("Unable to find rejection in: {rejection:?}");

    Ok(
        warp::reply::with_status("Something went wrong", StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    )
}
