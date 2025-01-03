use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    pub amount: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScheduledMeal {
    pub recipie_name: String,
    pub ingredients: Ingredient,
}
