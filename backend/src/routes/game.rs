use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};

use crate::{
    game::GameService,
    models::{
        ApiError, DailyGameEnvelope, RoundCheckResponse, RoundOneCheckResponse,
        RoundOneGuessRequest, SimpleAnswerRequest, StageThreeAnswerRequest,
    },
};

pub fn router() -> Router<Arc<GameService>> {
    Router::new()
        .route("/daily", get(get_daily_game))
        .route("/check/round1", post(check_round1))
        .route("/check/round2", post(check_round2))
        .route("/check/round3", post(check_round3))
}

async fn get_daily_game(
    State(game): State<Arc<GameService>>,
    headers: HeaderMap,
) -> Json<DailyGameEnvelope> {
    let date = GameService::today();
    let session = game.bootstrap_session(date, bearer_token(&headers));

    Json(DailyGameEnvelope {
        game: game.daily_game(date),
        progress: session.progress,
        session_token: session.token,
    })
}

async fn check_round1(
    State(game): State<Arc<GameService>>,
    headers: HeaderMap,
    Json(payload): Json<RoundOneGuessRequest>,
) -> Result<Json<RoundOneCheckResponse>, (StatusCode, Json<ApiError>)> {
    let date = GameService::today();
    let evaluation = game.check_round1(date, &payload.guess).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error,
            }),
        )
    })?;

    let session = game.record_round_attempt(date, bearer_token(&headers), 1, evaluation.correct);

    Ok(Json(RoundOneCheckResponse {
        correct: evaluation.correct,
        feedback: evaluation.feedback,
        progress: session.progress,
        session_token: session.token,
    }))
}

async fn check_round2(
    State(game): State<Arc<GameService>>,
    headers: HeaderMap,
    Json(payload): Json<SimpleAnswerRequest>,
) -> Json<RoundCheckResponse> {
    let date = GameService::today();
    let evaluation = game.check_round2(date, &payload.answer);
    let session = game.record_round_attempt(date, bearer_token(&headers), 2, evaluation.correct);

    Json(RoundCheckResponse {
        correct: evaluation.correct,
        message: evaluation.message,
        progress: session.progress,
        session_token: session.token,
    })
}

async fn check_round3(
    State(game): State<Arc<GameService>>,
    headers: HeaderMap,
    Json(payload): Json<StageThreeAnswerRequest>,
) -> Json<RoundCheckResponse> {
    let date = GameService::today();
    let evaluation = game.check_round3(date, &payload);
    let session = game.record_round_attempt(date, bearer_token(&headers), 3, evaluation.correct);

    Json(RoundCheckResponse {
        correct: evaluation.correct,
        message: evaluation.message,
        progress: session.progress,
        session_token: session.token,
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get("authorization")?.to_str().ok()?.trim();
    raw.strip_prefix("Bearer ")
}
