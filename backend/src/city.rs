use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct City {
    pub id: String,
    pub name: String,
    pub country: String,
    pub country_code: String,
    pub secret_word: String,
    pub activities: Vec<String>,
    pub known_for: String,
    pub famous_person: String,
    pub popular_item: String,
    pub map_svg: String,
    pub geography_prompt: String,
    pub geography_options: Vec<String>,
    pub duolingo: MultipleChoiceStage,
    pub trivia: MultipleChoiceStage,
    pub drawing_prompt: String,
    pub drawing_template: Vec<[f32; 2]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultipleChoiceStage {
    pub prompt: String,
    pub options: Vec<String>,
    pub answer: String,
}
