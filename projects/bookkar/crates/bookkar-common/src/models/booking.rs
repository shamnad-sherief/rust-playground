use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::passenger::Passenger;

/// Train class categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrainClass {
    /// First AC (1A)
    FirstAC,
    /// Second AC (2A)
    SecondAC,
    /// Third AC (3A)
    ThirdAC,
    /// Third AC Economy (3E)
    ThirdACEconomy,
    /// AC Chair Car (CC)
    ChairCar,
    /// Executive Chair Car (EC)
    ExecChairCar,
    /// Sleeper (SL)
    Sleeper,
    /// Second Sitting (2S)
    SecondSitting,
}

impl TrainClass {
    /// Returns the value used in IRCTC's class dropdown
    pub fn irctc_value(&self) -> &str {
        match self {
            TrainClass::FirstAC => "1A",
            TrainClass::SecondAC => "2A",
            TrainClass::ThirdAC => "3A",
            TrainClass::ThirdACEconomy => "3E",
            TrainClass::ChairCar => "CC",
            TrainClass::ExecChairCar => "EC",
            TrainClass::Sleeper => "SL",
            TrainClass::SecondSitting => "2S",
        }
    }

    /// Whether this is an AC class (Tatkal opens at 10 AM) or non-AC (11 AM)
    pub fn is_ac(&self) -> bool {
        !matches!(self, TrainClass::Sleeper | TrainClass::SecondSitting)
    }

    /// Hour when Tatkal booking opens for this class
    pub fn tatkal_hour(&self) -> u32 {
        if self.is_ac() { 10 } else { 11 }
    }
}

impl std::fmt::Display for TrainClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            TrainClass::FirstAC => "First AC (1A)",
            TrainClass::SecondAC => "Second AC (2A)",
            TrainClass::ThirdAC => "Third AC (3A)",
            TrainClass::ThirdACEconomy => "Third AC Economy (3E)",
            TrainClass::ChairCar => "AC Chair Car (CC)",
            TrainClass::ExecChairCar => "Executive Chair Car (EC)",
            TrainClass::Sleeper => "Sleeper (SL)",
            TrainClass::SecondSitting => "Second Sitting (2S)",
        };
        write!(f, "{}", label)
    }
}

/// Booking quota
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Quota {
    Tatkal,
    PremiumTatkal,
}

impl Quota {
    pub fn irctc_value(&self) -> &str {
        match self {
            Quota::Tatkal => "TQ",
            Quota::PremiumTatkal => "PT",
        }
    }
}

impl std::fmt::Display for Quota {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quota::Tatkal => write!(f, "Tatkal"),
            Quota::PremiumTatkal => write!(f, "Premium Tatkal"),
        }
    }
}

/// Payment method for booking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentMethod {
    Upi { vpa: String },
    IrctcEwallet,
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentMethod::Upi { vpa } => write!(f, "UPI ({})", vpa),
            PaymentMethod::IrctcEwallet => write!(f, "IRCTC eWallet"),
        }
    }
}

/// Booking status lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BookingStatus {
    Configured,
    BrowserLaunched,
    LoggingIn,
    WaitingForLoginOtp,
    LoggedIn,
    SearchReady,
    WaitingForTatkalWindow,
    Searching,
    TrainSelected,
    FillingPassengers,
    WaitingForAadhaarOtp,
    AadhaarVerified,
    ProcessingPayment,
    WaitingForPaymentApproval,
    Success,
    Failed { reason: String },
}

impl std::fmt::Display for BookingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookingStatus::Configured => write!(f, "⚙️  Configured"),
            BookingStatus::BrowserLaunched => write!(f, "🌐 Browser launched"),
            BookingStatus::LoggingIn => write!(f, "🔐 Logging in..."),
            BookingStatus::WaitingForLoginOtp => write!(f, "⏳ Waiting for login OTP"),
            BookingStatus::LoggedIn => write!(f, "✅ Logged in"),
            BookingStatus::SearchReady => write!(f, "📝 Search form ready"),
            BookingStatus::WaitingForTatkalWindow => write!(f, "⏰ Waiting for Tatkal window"),
            BookingStatus::Searching => write!(f, "🔍 Searching trains..."),
            BookingStatus::TrainSelected => write!(f, "🚂 Train selected"),
            BookingStatus::FillingPassengers => write!(f, "👥 Filling passenger details"),
            BookingStatus::WaitingForAadhaarOtp => write!(f, "⏳ Waiting for Aadhaar OTP"),
            BookingStatus::AadhaarVerified => write!(f, "✅ Aadhaar verified"),
            BookingStatus::ProcessingPayment => write!(f, "💳 Processing payment"),
            BookingStatus::WaitingForPaymentApproval => write!(f, "⏳ Approve payment on your app"),
            BookingStatus::Success => write!(f, "🎉 Booking confirmed!"),
            BookingStatus::Failed { reason } => write!(f, "❌ Failed: {}", reason),
        }
    }
}

/// Full booking configuration provided by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingConfig {
    /// IRCTC username
    pub username: String,
    /// IRCTC password
    pub password: String,
    /// Source station code (e.g., "NDLS")
    pub from_station: String,
    /// Destination station code (e.g., "MAS")
    pub to_station: String,
    /// Journey date
    pub journey_date: NaiveDate,
    /// Train class
    pub class: TrainClass,
    /// Booking quota
    pub quota: Quota,
    /// Specific train number (None = first available)
    pub train_number: Option<String>,
    /// List of passengers (1-6)
    pub passengers: Vec<Passenger>,
    /// Payment method
    pub payment: PaymentMethod,
    /// Whether to auto-upgrade if higher class is available
    pub auto_upgrade: bool,
}

/// Result of a successful booking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingResult {
    /// PNR number
    pub pnr: String,
    /// Train number
    pub train_number: String,
    /// Train name
    pub train_name: String,
    /// Booking status message
    pub status_message: String,
}
