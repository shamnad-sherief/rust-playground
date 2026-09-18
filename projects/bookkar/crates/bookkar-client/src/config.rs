use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use bookkar_common::{
    BerthPreference, BookingConfig, Gender, Passenger, PaymentMethod, Quota, TrainClass,
};

/// User-facing configuration that gets saved to disk as TOML.
/// Separated from BookingConfig because we don't want to save passwords in TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    pub from_station: String,
    pub to_station: String,
    pub journey_date: String,
    pub class: String,
    pub quota: String,
    pub train_number: Option<String>,
    pub passengers: Vec<SavedPassenger>,
    pub payment_method: String,
    pub upi_id: Option<String>,
    pub auto_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPassenger {
    pub name: String,
    pub age: u8,
    pub gender: String,
    pub berth_preference: String,
}

impl SavedConfig {
    /// Convert to a full BookingConfig by adding credentials.
    pub fn to_booking_config(&self, username: &str, password: &str) -> Result<BookingConfig> {
        let class = match self.class.to_uppercase().as_str() {
            "1A" => TrainClass::FirstAC,
            "2A" => TrainClass::SecondAC,
            "3A" => TrainClass::ThirdAC,
            "3E" => TrainClass::ThirdACEconomy,
            "CC" => TrainClass::ChairCar,
            "EC" => TrainClass::ExecChairCar,
            "SL" => TrainClass::Sleeper,
            "2S" => TrainClass::SecondSitting,
            other => return Err(anyhow::anyhow!("Unknown class: {}", other)),
        };

        let quota = match self.quota.to_uppercase().as_str() {
            "TQ" | "TATKAL" => Quota::Tatkal,
            "PT" | "PREMIUM_TATKAL" | "PREMIUM" => Quota::PremiumTatkal,
            other => return Err(anyhow::anyhow!("Unknown quota: {}", other)),
        };

        let payment = match self.payment_method.to_uppercase().as_str() {
            "UPI" => PaymentMethod::Upi {
                vpa: self.upi_id.clone().unwrap_or_default(),
            },
            "EWALLET" | "WALLET" => PaymentMethod::IrctcEwallet,
            other => return Err(anyhow::anyhow!("Unknown payment method: {}", other)),
        };

        let passengers: Result<Vec<Passenger>> = self
            .passengers
            .iter()
            .map(|p| {
                let gender = match p.gender.to_uppercase().as_str() {
                    "M" | "MALE" => Gender::Male,
                    "F" | "FEMALE" => Gender::Female,
                    "T" | "TRANSGENDER" => Gender::Transgender,
                    other => return Err(anyhow::anyhow!("Unknown gender: {}", other)),
                };

                let berth = match p.berth_preference.to_uppercase().as_str() {
                    "LB" | "LOWER" => BerthPreference::Lower,
                    "MB" | "MIDDLE" => BerthPreference::Middle,
                    "UB" | "UPPER" => BerthPreference::Upper,
                    "SL" | "SIDE LOWER" => BerthPreference::SideLower,
                    "SU" | "SIDE UPPER" => BerthPreference::SideUpper,
                    _ => BerthPreference::NoPreference,
                };

                Ok(Passenger {
                    name: p.name.clone(),
                    age: p.age,
                    gender,
                    berth_preference: berth,
                    nationality: None,
                })
            })
            .collect();

        let journey_date = NaiveDate::parse_from_str(&self.journey_date, "%d-%m-%Y")
            .or_else(|_| NaiveDate::parse_from_str(&self.journey_date, "%Y-%m-%d"))?;

        Ok(BookingConfig {
            username: username.to_string(),
            password: password.to_string(),
            from_station: self.from_station.clone(),
            to_station: self.to_station.clone(),
            journey_date,
            class,
            quota,
            train_number: self.train_number.clone(),
            passengers: passengers?,
            payment,
            auto_upgrade: self.auto_upgrade,
        })
    }
}

/// Get the config file path (platform-appropriate).
pub fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bookkar");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.toml")
}

/// Load saved config from disk.
pub fn load_config() -> Result<Option<SavedConfig>> {
    let path = config_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)?;
    let config: SavedConfig = toml::from_str(&content)?;
    Ok(Some(config))
}

/// Save config to disk (excludes credentials).
pub fn save_config(config: &SavedConfig) -> Result<()> {
    let path = config_path();
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}
