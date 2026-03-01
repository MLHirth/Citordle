use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, NaiveDate, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::models::DailyProgress;

type HmacSha256 = Hmac<Sha256>;

const TOKEN_LIFETIME_DAYS: i64 = 45;
const MAX_DAILY_ENTRIES: usize = 14;

#[derive(Clone)]
pub struct SessionManager {
    secret: Vec<u8>,
}

pub struct SessionSnapshot {
    pub progress: DailyProgress,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionClaims {
    sub: String,
    iat: usize,
    exp: usize,
    daily: Vec<DailyProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
}

impl SessionManager {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    pub fn bootstrap(
        &self,
        token: Option<&str>,
        date: NaiveDate,
        city_id: &str,
    ) -> SessionSnapshot {
        let mut claims = self
            .decode_claims(token)
            .unwrap_or_else(|| self.new_claims());
        touch_current_progress(&mut claims, date, city_id);
        refresh_claim_times(&mut claims);
        trim_daily_entries(&mut claims);

        let progress = current_progress(&claims, date, city_id)
            .unwrap_or_else(|| default_progress(date, city_id));
        SessionSnapshot {
            progress,
            token: self.encode_claims(&claims),
        }
    }

    pub fn apply_round_attempt(
        &self,
        token: Option<&str>,
        date: NaiveDate,
        city_id: &str,
        round: u8,
        correct: bool,
    ) -> SessionSnapshot {
        let mut claims = self
            .decode_claims(token)
            .unwrap_or_else(|| self.new_claims());
        touch_current_progress(&mut claims, date, city_id);

        if let Some(progress) = current_progress_mut(&mut claims, date, city_id) {
            match round {
                1 => {
                    progress.round1_attempts = progress.round1_attempts.saturating_add(1);
                    progress.round1_completed = progress.round1_completed || correct;
                }
                2 => {
                    progress.round2_attempts = progress.round2_attempts.saturating_add(1);
                    progress.round2_completed = progress.round2_completed || correct;
                }
                3 => {
                    progress.round3_attempts = progress.round3_attempts.saturating_add(1);
                    progress.round3_completed = progress.round3_completed || correct;
                }
                _ => {}
            }

            progress.completed =
                progress.round1_completed && progress.round2_completed && progress.round3_completed;
        }

        refresh_claim_times(&mut claims);
        trim_daily_entries(&mut claims);

        let progress = current_progress(&claims, date, city_id)
            .unwrap_or_else(|| default_progress(date, city_id));
        SessionSnapshot {
            progress,
            token: self.encode_claims(&claims),
        }
    }

    fn decode_claims(&self, token: Option<&str>) -> Option<SessionClaims> {
        let raw = token?.trim();
        if raw.is_empty() {
            return None;
        }

        let mut parts = raw.split('.');
        let header_part = parts.next()?;
        let payload_part = parts.next()?;
        let signature_part = parts.next()?;
        if parts.next().is_some() {
            return None;
        }

        if !self.verify_signature(header_part, payload_part, signature_part) {
            return None;
        }

        let header_json = URL_SAFE_NO_PAD.decode(header_part).ok()?;
        let header: JwtHeader = serde_json::from_slice(&header_json).ok()?;
        if header.alg != "HS256" || header.typ != "JWT" {
            return None;
        }

        let payload_json = URL_SAFE_NO_PAD.decode(payload_part).ok()?;
        let claims: SessionClaims = serde_json::from_slice(&payload_json).ok()?;

        let now = Utc::now().timestamp() as usize;
        if claims.exp <= now {
            return None;
        }

        Some(claims)
    }

    fn encode_claims(&self, claims: &SessionClaims) -> String {
        let header = JwtHeader {
            alg: "HS256".to_string(),
            typ: "JWT".to_string(),
        };

        let header_json = match serde_json::to_vec(&header) {
            Ok(value) => value,
            Err(_) => return String::new(),
        };

        let payload_json = match serde_json::to_vec(claims) {
            Ok(value) => value,
            Err(_) => return String::new(),
        };

        let header_part = URL_SAFE_NO_PAD.encode(header_json);
        let payload_part = URL_SAFE_NO_PAD.encode(payload_json);
        let signing_input = format!("{}.{}", header_part, payload_part);
        let signature = self.sign(signing_input.as_bytes());
        let signature_part = URL_SAFE_NO_PAD.encode(signature);

        format!("{}.{}.{}", header_part, payload_part, signature_part)
    }

    fn sign(&self, input: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("hmac key");
        mac.update(input);
        mac.finalize().into_bytes().to_vec()
    }

    fn verify_signature(&self, header: &str, payload: &str, signature: &str) -> bool {
        let Ok(signature_bytes) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };

        let signing_input = format!("{}.{}", header, payload);
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("hmac key");
        mac.update(signing_input.as_bytes());
        mac.verify_slice(&signature_bytes).is_ok()
    }

    fn new_claims(&self) -> SessionClaims {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::days(TOKEN_LIFETIME_DAYS)).timestamp() as usize;

        SessionClaims {
            sub: format!("anon-{}", now.timestamp_nanos_opt().unwrap_or_default()),
            iat,
            exp,
            daily: Vec::new(),
        }
    }
}

fn default_progress(date: NaiveDate, city_id: &str) -> DailyProgress {
    DailyProgress {
        date: date.to_string(),
        city_id: city_id.to_string(),
        round1_attempts: 0,
        round2_attempts: 0,
        round3_attempts: 0,
        round1_completed: false,
        round2_completed: false,
        round3_completed: false,
        completed: false,
    }
}

fn touch_current_progress(claims: &mut SessionClaims, date: NaiveDate, city_id: &str) {
    if current_progress_mut(claims, date, city_id).is_none() {
        claims.daily.push(default_progress(date, city_id));
    }
}

fn current_progress(
    claims: &SessionClaims,
    date: NaiveDate,
    city_id: &str,
) -> Option<DailyProgress> {
    let day = date.to_string();
    claims
        .daily
        .iter()
        .rev()
        .find(|item| item.date == day && item.city_id == city_id)
        .cloned()
}

fn current_progress_mut<'a>(
    claims: &'a mut SessionClaims,
    date: NaiveDate,
    city_id: &str,
) -> Option<&'a mut DailyProgress> {
    let day = date.to_string();
    claims
        .daily
        .iter_mut()
        .rev()
        .find(|item| item.date == day && item.city_id == city_id)
}

fn refresh_claim_times(claims: &mut SessionClaims) {
    let now = Utc::now();
    claims.iat = now.timestamp() as usize;
    claims.exp = (now + Duration::days(TOKEN_LIFETIME_DAYS)).timestamp() as usize;
}

fn trim_daily_entries(claims: &mut SessionClaims) {
    if claims.daily.len() <= MAX_DAILY_ENTRIES {
        return;
    }

    let start = claims.daily.len().saturating_sub(MAX_DAILY_ENTRIES);
    claims.daily = claims.daily[start..].to_vec();
}
