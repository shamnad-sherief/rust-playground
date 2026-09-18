use serde::{Deserialize, Serialize};

/// Gender options for passenger details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Gender {
    Male,
    Female,
    Transgender,
}

impl Gender {
    /// Returns the value used in IRCTC's form dropdown
    pub fn irctc_value(&self) -> &str {
        match self {
            Gender::Male => "M",
            Gender::Female => "F",
            Gender::Transgender => "T",
        }
    }
}

impl std::fmt::Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Gender::Male => write!(f, "Male"),
            Gender::Female => write!(f, "Female"),
            Gender::Transgender => write!(f, "Transgender"),
        }
    }
}

/// Berth preference for sleeping accommodation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BerthPreference {
    Lower,
    Middle,
    Upper,
    SideLower,
    SideUpper,
    NoPreference,
}

impl BerthPreference {
    /// Returns the value used in IRCTC's form dropdown
    pub fn irctc_value(&self) -> &str {
        match self {
            BerthPreference::Lower => "LB",
            BerthPreference::Middle => "MB",
            BerthPreference::Upper => "UB",
            BerthPreference::SideLower => "SL",
            BerthPreference::SideUpper => "SU",
            BerthPreference::NoPreference => "No Preference",
        }
    }
}

impl std::fmt::Display for BerthPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BerthPreference::Lower => write!(f, "Lower"),
            BerthPreference::Middle => write!(f, "Middle"),
            BerthPreference::Upper => write!(f, "Upper"),
            BerthPreference::SideLower => write!(f, "Side Lower"),
            BerthPreference::SideUpper => write!(f, "Side Upper"),
            BerthPreference::NoPreference => write!(f, "No Preference"),
        }
    }
}

/// A single passenger entry for booking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passenger {
    pub name: String,
    pub age: u8,
    pub gender: Gender,
    pub berth_preference: BerthPreference,
    /// Optional: nationality (default "Indian")
    pub nationality: Option<String>,
}

impl std::fmt::Display for Passenger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} | {} | {} | {}",
            self.name, self.gender, self.age, self.berth_preference
        )
    }
}
