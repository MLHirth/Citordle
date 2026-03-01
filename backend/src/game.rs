use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate, Utc};

use crate::{
    city::City,
    models::{
        DailyGameResponse, LetterFeedback, LetterState, RoundOnePrompt, RoundThreePrompt,
        RoundTwoPrompt, StageThreeAnswerRequest, StageThreeKind,
    },
    session::{SessionManager, SessionSnapshot},
};

pub struct GameService {
    cities: Vec<City>,
    allowed_words: HashSet<String>,
    session_manager: SessionManager,
}

pub struct RoundOneEvaluation {
    pub correct: bool,
    pub feedback: Vec<LetterFeedback>,
}

pub struct RoundEvaluation {
    pub correct: bool,
    pub message: String,
}

impl GameService {
    pub fn new(
        cities: Vec<City>,
        allowed_words: HashSet<String>,
        session_manager: SessionManager,
    ) -> Self {
        Self {
            cities,
            allowed_words,
            session_manager,
        }
    }

    pub fn today() -> NaiveDate {
        Utc::now().date_naive()
    }

    pub fn daily_game(&self, date: NaiveDate) -> DailyGameResponse {
        let city = self.city_for_date(date);
        let stage_kind = Self::stage_kind_for_date(date);

        let hints = vec![
            format!(
                "Try thinking about what visitors do: {}.",
                city.activities.join(", ")
            ),
            format!("This city is known for {}.", city.known_for),
            format!(
                "A famous person linked to this city: {}.",
                city.famous_person
            ),
            format!("A common local bite or item is {}.", city.popular_item),
        ];

        let round3 = match stage_kind {
            StageThreeKind::Duolingo => RoundThreePrompt {
                kind: StageThreeKind::Duolingo,
                prompt: city.duolingo.prompt.clone(),
                options: city.duolingo.options.clone(),
                instructions: Some("Pick the best translation.".to_string()),
                guide_points: None,
            },
            StageThreeKind::Trivia => RoundThreePrompt {
                kind: StageThreeKind::Trivia,
                prompt: city.trivia.prompt.clone(),
                options: city.trivia.options.clone(),
                instructions: Some("Choose the correct trivia answer.".to_string()),
                guide_points: None,
            },
            StageThreeKind::Draw => RoundThreePrompt {
                kind: StageThreeKind::Draw,
                prompt: city.drawing_prompt.clone(),
                options: Vec::new(),
                instructions: Some(
                    "Trace the faint landmark guide first, then add your own lines and submit."
                        .to_string(),
                ),
                guide_points: Some(city.drawing_template.clone()),
            },
        };

        DailyGameResponse {
            date: date.to_string(),
            city_id: city.id.clone(),
            city_name: city.name.clone(),
            country: city.country.clone(),
            round1: RoundOnePrompt {
                word_length: normalize_word(&city.secret_word).len(),
                hints,
            },
            round2: RoundTwoPrompt {
                prompt: city.geography_prompt.clone(),
                country_code: city.country_code.clone(),
                map_svg: city.map_svg.clone(),
                options: city.geography_options.clone(),
            },
            round3,
        }
    }

    pub fn check_round1(&self, date: NaiveDate, guess: &str) -> Result<RoundOneEvaluation, String> {
        let city = self.city_for_date(date);
        let secret = normalize_word(&city.secret_word);
        let normalized_guess = normalize_word(guess);

        if normalized_guess.is_empty() {
            return Err("Guess must include letters.".to_string());
        }

        if normalized_guess.len() != secret.len() {
            return Err(format!("Guess must be {} letters long.", secret.len()));
        }

        if !self.allowed_words.contains(&normalized_guess) {
            return Err("Guess is not in the allowed dictionary list.".to_string());
        }

        let feedback = build_feedback(&secret, &normalized_guess);
        Ok(RoundOneEvaluation {
            correct: normalized_guess == secret,
            feedback,
        })
    }

    pub fn check_round2(&self, date: NaiveDate, answer: &str) -> RoundEvaluation {
        let city = self.city_for_date(date);
        let expected = normalize_text(&city.country);
        let received = normalize_text(answer);
        let correct = expected == received;

        RoundEvaluation {
            correct,
            message: if correct {
                "Nice map read. Round 2 cleared.".to_string()
            } else {
                "Not quite. Try another geography option.".to_string()
            },
        }
    }

    pub fn check_round3(
        &self,
        date: NaiveDate,
        request: &StageThreeAnswerRequest,
    ) -> RoundEvaluation {
        let city = self.city_for_date(date);
        let stage_kind = Self::stage_kind_for_date(date);

        match stage_kind {
            StageThreeKind::Duolingo => {
                let expected = normalize_text(&city.duolingo.answer);
                let received = normalize_text(request.answer.as_deref().unwrap_or(""));
                let correct = expected == received;

                RoundEvaluation {
                    correct,
                    message: if correct {
                        "Correct translation. You are done for today!".to_string()
                    } else {
                        "Close, but that translation is not the best fit.".to_string()
                    },
                }
            }
            StageThreeKind::Trivia => {
                let expected = normalize_text(&city.trivia.answer);
                let received = normalize_text(request.answer.as_deref().unwrap_or(""));
                let correct = expected == received;

                RoundEvaluation {
                    correct,
                    message: if correct {
                        "Trivia nailed. You are done for today!".to_string()
                    } else {
                        "Not that one. Try another trivia choice.".to_string()
                    },
                }
            }
            StageThreeKind::Draw => {
                let strokes = request.strokes.as_deref().unwrap_or(&[]);

                let similarity = drawing_similarity_score(&city.drawing_template, strokes);
                let has_strokes = strokes.iter().any(|stroke| stroke.len() >= 2);
                let point_count = strokes.iter().map(Vec::len).sum::<usize>();
                let close_enough = similarity <= 0.58;
                let strong_attempt = point_count >= 110 && similarity <= 0.78;
                let correct = has_strokes && (close_enough || strong_attempt);

                RoundEvaluation {
                    correct,
                    message: if correct {
                        format!(
                            "Great sketch submission (similarity {:.2}). You are done for today!",
                            similarity
                        )
                    } else {
                        format!(
                            "Keep drawing the landmark silhouette. Similarity was {:.2}; try matching the famous building more closely.",
                            similarity
                        )
                    },
                }
            }
        }
    }

    pub fn bootstrap_session(
        &self,
        date: NaiveDate,
        session_token: Option<&str>,
    ) -> SessionSnapshot {
        let city = self.city_for_date(date);
        self.session_manager
            .bootstrap(session_token, date, city.id.as_str())
    }

    pub fn record_round_attempt(
        &self,
        date: NaiveDate,
        session_token: Option<&str>,
        round: u8,
        correct: bool,
    ) -> SessionSnapshot {
        let city = self.city_for_date(date);
        self.session_manager.apply_round_attempt(
            session_token,
            date,
            city.id.as_str(),
            round,
            correct,
        )
    }

    fn city_for_date(&self, date: NaiveDate) -> &City {
        let city_count = self.cities.len();
        if city_count == 1 {
            return &self.cities[0];
        }

        let day_number = i64::from(date.num_days_from_ce());
        let cycle = day_number.div_euclid(city_count as i64);
        let offset = day_number.rem_euclid(city_count as i64) as usize;

        let mut order = city_order_for_cycle(&self.cities, cycle);

        let previous_order = city_order_for_cycle(&self.cities, cycle - 1);
        if previous_order[city_count - 1] == order[0] {
            order.rotate_left(1);
        }

        &self.cities[order[offset]]
    }

    fn stage_kind_for_date(date: NaiveDate) -> StageThreeKind {
        let seed = hash_seed(format!("{}-stage3", date).as_str());
        match seed % 3 {
            0 => StageThreeKind::Duolingo,
            1 => StageThreeKind::Draw,
            _ => StageThreeKind::Trivia,
        }
    }
}

fn hash_seed(input: &str) -> usize {
    seeded_hash(input.as_bytes(), 0x9E37_79B9_7F4A_7C15) as usize
}

fn city_order_for_cycle(cities: &[City], cycle: i64) -> Vec<usize> {
    let mut order = (0..cities.len()).collect::<Vec<_>>();
    let cycle_seed = cycle as u64;

    order.sort_by(|left, right| {
        let left_city = &cities[*left];
        let right_city = &cities[*right];

        let left_hash = seeded_hash(left_city.id.as_bytes(), cycle_seed);
        let right_hash = seeded_hash(right_city.id.as_bytes(), cycle_seed);

        left_hash
            .cmp(&right_hash)
            .then_with(|| left_city.id.cmp(&right_city.id))
    });

    order
}

fn seeded_hash(input: &[u8], seed: u64) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS ^ seed.rotate_left(13);
    for byte in input {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn normalize_word(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn normalize_text(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn build_feedback(secret: &str, guess: &str) -> Vec<LetterFeedback> {
    let secret_chars: Vec<char> = secret.chars().collect();
    let guess_chars: Vec<char> = guess.chars().collect();

    let mut states = vec![LetterState::Absent; guess_chars.len()];
    let mut counts: HashMap<char, usize> = HashMap::new();

    for &ch in &secret_chars {
        *counts.entry(ch).or_insert(0) += 1;
    }

    for idx in 0..guess_chars.len() {
        if guess_chars[idx] == secret_chars[idx] {
            states[idx] = LetterState::Correct;
            if let Some(count) = counts.get_mut(&guess_chars[idx]) {
                *count -= 1;
            }
        }
    }

    for idx in 0..guess_chars.len() {
        if matches!(states[idx], LetterState::Correct) {
            continue;
        }
        if let Some(count) = counts.get_mut(&guess_chars[idx]) {
            if *count > 0 {
                states[idx] = LetterState::Present;
                *count -= 1;
            }
        }
    }

    guess_chars
        .iter()
        .enumerate()
        .map(|(idx, ch)| LetterFeedback {
            letter: ch.to_string(),
            status: states[idx],
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Point2 {
    x: f32,
    y: f32,
}

fn drawing_similarity_score(template: &[[f32; 2]], strokes: &[Vec<[f32; 2]>]) -> f32 {
    let template_points = template
        .iter()
        .map(|point| Point2 {
            x: point[0],
            y: point[1],
        })
        .collect::<Vec<_>>();

    let drawn_points = strokes
        .iter()
        .flat_map(|stroke| {
            stroke.iter().map(|point| Point2 {
                x: point[0],
                y: point[1],
            })
        })
        .collect::<Vec<_>>();

    if template_points.len() < 2 || drawn_points.len() < 6 {
        return 1.0;
    }

    let normalized_template = normalize_points(&template_points);
    let normalized_drawn = normalize_points(&drawn_points);

    if normalized_template.is_empty() || normalized_drawn.is_empty() {
        return 1.0;
    }

    let resampled_template = resample_points(&normalized_template, 64);
    let resampled_drawn = resample_points(&normalized_drawn, 64);

    if resampled_template.len() < 2 || resampled_drawn.len() < 2 {
        return 1.0;
    }

    let forward = average_distance(&resampled_template, &resampled_drawn);

    let mut reversed = resampled_drawn.clone();
    reversed.reverse();
    let backward = average_distance(&resampled_template, &reversed);

    forward.min(backward)
}

fn normalize_points(points: &[Point2]) -> Vec<Point2> {
    if points.is_empty() {
        return Vec::new();
    }

    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);

    let width = (max_x - min_x).max(0.001);
    let height = (max_y - min_y).max(0.001);
    let scale = width.max(height);

    points
        .iter()
        .map(|point| Point2 {
            x: (point.x - min_x) / scale,
            y: (point.y - min_y) / scale,
        })
        .collect()
}

fn resample_points(points: &[Point2], target_count: usize) -> Vec<Point2> {
    if points.is_empty() || target_count == 0 {
        return Vec::new();
    }

    if points.len() == 1 {
        return vec![points[0]; target_count];
    }

    let total_length = points
        .windows(2)
        .map(|pair| distance(pair[0], pair[1]))
        .sum::<f32>();

    if total_length <= 0.0001 {
        return vec![points[0]; target_count];
    }

    let segment = total_length / (target_count.saturating_sub(1) as f32);
    let mut sampled = Vec::with_capacity(target_count);
    sampled.push(points[0]);

    let mut accumulated = 0.0;
    let mut last_point = points[0];
    let mut index = 1;

    while index < points.len() {
        let current = points[index];
        let dist = distance(last_point, current);

        if accumulated + dist >= segment {
            let ratio = (segment - accumulated) / dist.max(0.0001);
            let interpolated = Point2 {
                x: last_point.x + ratio * (current.x - last_point.x),
                y: last_point.y + ratio * (current.y - last_point.y),
            };
            sampled.push(interpolated);
            last_point = interpolated;
            accumulated = 0.0;
        } else {
            accumulated += dist;
            last_point = current;
            index += 1;
        }
    }

    while sampled.len() < target_count {
        sampled.push(*points.last().unwrap_or(&points[0]));
    }

    sampled
}

fn average_distance(first: &[Point2], second: &[Point2]) -> f32 {
    let count = first.len().min(second.len()).max(1);
    first
        .iter()
        .zip(second.iter())
        .take(count)
        .map(|(a, b)| distance(*a, *b))
        .sum::<f32>()
        / (count as f32)
}

fn distance(first: Point2, second: Point2) -> f32 {
    let dx = first.x - second.x;
    let dy = first.y - second.y;
    (dx * dx + dy * dy).sqrt()
}
