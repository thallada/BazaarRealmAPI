use http_api_problem::HttpApiProblem;
use tracing::error;
use warp::http::StatusCode;
use warp::{reject, Rejection, Reply};

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
                if let Some(code) = db_error.code() {
                    if let Some(constraint) = db_error.constraint_name() {
                        if code == "23503" && constraint == "shops_owner_id_fkey" {
                            // foreign_key_violation
                            return HttpApiProblem::with_title_and_type_from_status(
                                StatusCode::BAD_REQUEST,
                            )
                            .set_detail("Owner does not exist");
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
