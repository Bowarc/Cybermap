use {
    reqwest::StatusCode,
    std::convert::Infallible,
    warp::{reject::Rejection, reply::Reply},
    warp_rate_limit::{RateLimitRejection, add_rate_limit_headers_from_rejection},
};

use crate::osm_proxy::OSMProxyRejection;

pub async fn handle_rejection(rejection: Rejection) -> Result<impl Reply, Infallible> {
    if rejection.is_not_found() {
        return Ok(warp::reply::with_status(
            "The content you requested does not exist",
            StatusCode::NOT_FOUND,
        )
        .into_response());
    }

    if let Some(proxy_rejection) = rejection.find::<OSMProxyRejection>() {
        let response = match proxy_rejection {
            OSMProxyRejection::CacheFailure
            | OSMProxyRejection::APIServerFailure
            | OSMProxyRejection::APIResponseUnpackingFailed => {
                warp::reply::with_status("Internal server error", StatusCode::INTERNAL_SERVER_ERROR)
                    .into_response()
            }

            OSMProxyRejection::InvalidUserData => {
                warp::reply::with_status("Bad request", StatusCode::BAD_REQUEST).into_response()
            }
        };
        return Ok(response);
    };

    if let Some(rate_limit_rejection) = rejection.find::<RateLimitRejection>() {
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

        return Ok(response);
    }

    error!("Unable to find rejection in: {rejection:?}");

    Ok(
        warp::reply::with_status("Something went wrong", StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    )
}
