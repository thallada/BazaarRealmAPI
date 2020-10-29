use anyhow::{anyhow, Error};
use http::StatusCode;
use http_api_problem::HttpApiProblem;
use tracing::error;
use warp::{reject, Rejection, Reply};

pub fn forbidden_permission() -> Error {
    anyhow!(
        HttpApiProblem::with_title_and_type_from_status(StatusCode::FORBIDDEN,)
            .set_detail("Api-Key does not have required permissions")
    )
}

pub fn unauthorized_no_owner() -> Error {
    anyhow!(
        HttpApiProblem::with_title_and_type_from_status(StatusCode::UNAUTHORIZED,)
            .set_detail("Api-Key not recognized")
    )
}

pub fn unauthorized_no_api_key() -> Error {
    anyhow!(
        HttpApiProblem::with_title_and_type_from_status(StatusCode::UNAUTHORIZED,)
            .set_detail("Api-Key header not present")
    )
}

pub fn from_anyhow(error: anyhow::Error) -> HttpApiProblem {
    let error = match error.downcast::<HttpApiProblem>() {
        Ok(problem) => return problem,
        Err(error) => error,
    };

    if let Some(sqlx_error) = error.downcast_ref::<sqlx::error::Error>() {
        match sqlx_error {
            sqlx::error::Error::RowNotFound => {
                return HttpApiProblem::with_title_and_type_from_status(StatusCode::NOT_FOUND)
            }
            sqlx::error::Error::Database(db_error) => {
                error!(
                    "Database error: {}. {}",
                    db_error.message(),
                    db_error.details().unwrap_or("")
                );
                dbg!(&db_error);
                if let Some(code) = db_error.code() {
                    dbg!(&code);
                    if let Some(constraint) = db_error.constraint_name() {
                        dbg!(&constraint);
                        if code == "23503" && constraint == "shops_owner_id_fkey" {
                            // foreign_key_violation
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Owner does not exist");
                        } else if code == "23505" && constraint == "owners_api_key_key" {
                            // unique_violation
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Owner with Api-Key already exists");
                        } else if code == "23505" && constraint == "owners_unique_name_and_api_key"
                        {
                            // unique_violation
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Duplicate owner with same name and Api-Key exists");
                        } else if code == "23505" && constraint == "shops_unique_name_and_owner_id"
                        {
                            // unique_violation
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Owner already has a shop with that name");
                        } else if code == "23514" && constraint == "merchandise_quantity_gt_zero" {
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Quantity of merchandise must be greater than zero");
                        }
                    }
                }
            }
            _ => {}
        }
    }

    error!("Recovering unhandled error: {:?}", error);
    // TODO: this leaks internal info, should not stringify error
    HttpApiProblem::new(format!("Internal Server Error: {:?}", error))
        .set_status(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn unpack_problem(rejection: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(problem) = rejection.find::<HttpApiProblem>() {
        let code = problem.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let reply = warp::reply::json(problem);
        let reply = warp::reply::with_status(reply, code);
        let reply = warp::reply::with_header(
            reply,
            warp::http::header::CONTENT_TYPE,
            http_api_problem::PROBLEM_JSON_MEDIA_TYPE,
        );

        return Ok(reply);
    }

    Err(rejection)
}

pub fn reject_anyhow(error: anyhow::Error) -> Rejection {
    reject::custom(from_anyhow(error))
}
