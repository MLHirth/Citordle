use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DailyGameResponse {
    pub date: String,
    pub city_id: String,
    pub city_name: String,
    pub country: String,
    pub round1: RoundOnePrompt,
    pub round2: RoundTwoPrompt,
    pub round3: RoundThreePrompt,
}

#[derive(Debug, Serialize)]
pub struct DailyGameEnvelope {
    pub game: DailyGameResponse,
    pub progress: DailyProgress,
    pub session_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyProgress {
    pub date: String,
    pub city_id: String,
    pub round1_attempts: u32,
    pub round2_attempts: u32,
    pub round3_attempts: u32,
    pub round1_completed: bool,
    pub round2_completed: bool,
    pub round3_completed: bool,
    pub completed: bool,
}

#[derive(Debug, Serialize)]
pub struct RoundOnePrompt {
    pub word_length: usize,
    pub hints: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RoundTwoPrompt {
    pub prompt: String,
    pub country_code: String,
    pub map_svg: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageThreeKind {
    Duolingo,
    Draw,
    Trivia,
}

#[derive(Debug, Serialize)]
pub struct RoundThreePrompt {
    pub kind: StageThreeKind,
    pub prompt: String,
    pub options: Vec<String>,
    pub instructions: Option<String>,
    pub guide_points: Option<Vec<[f32; 2]>>,
}

#[derive(Debug, Deserialize)]
pub struct RoundOneGuessRequest {
    pub guess: String,
}

#[derive(Debug, Deserialize)]
pub struct SimpleAnswerRequest {
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct StageThreeAnswerRequest {
    pub answer: Option<String>,
    pub strokes: Option<Vec<Vec<[f32; 2]>>>,
}

#[derive(Debug, Serialize)]
pub struct RoundOneCheckResponse {
    pub correct: bool,
    pub feedback: Vec<LetterFeedback>,
    pub progress: DailyProgress,
    pub session_token: String,
}

#[derive(Debug, Serialize)]
pub struct RoundCheckResponse {
    pub correct: bool,
    pub message: String,
    pub progress: DailyProgress,
    pub session_token: String,
}

#[derive(Debug, Serialize)]
pub struct LetterFeedback {
    pub letter: String,
    pub status: LetterState,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LetterState {
    Correct,
    Present,
    Absent,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}
