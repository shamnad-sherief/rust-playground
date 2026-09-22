/// IRCTC CSS selectors — single source of truth.
///
/// These selectors target IRCTC's Angular-based frontend elements.
/// They must be verified and updated by inspecting the live IRCTC DOM.
/// IRCTC may change their DOM structure at any time — when automation
/// breaks, check and update these selectors first.

pub mod login {
    /// Username text input on the login page
    pub const USERNAME_INPUT: &str = "input[formcontrolname='userid']";
    /// Password text input on the login page
    pub const PASSWORD_INPUT: &str = "input[formcontrolname='password']";
    /// The main login submit button
    pub const LOGIN_BUTTON: &str = "button[type='submit']";
    /// CAPTCHA image element (may not always be present)
    pub const CAPTCHA_IMAGE: &str = "img.captcha-img, img[src*='captcha'], .captcha-img, app-captcha";
    /// CAPTCHA text input
    pub const CAPTCHA_INPUT: &str = "input[formcontrolname='captcha'], input#captcha, input#nlpAnswer, input[placeholder*='captcha' i]";
    /// OTP input field (appears after login submit)
    pub const OTP_INPUT: &str = "input#otp";
    /// OTP submit button
    pub const OTP_SUBMIT: &str = "button.btn.btn-primary";
    /// Element that indicates successful login (e.g., user menu)
    pub const LOGGED_IN_INDICATOR: &str = "a.loginText";
    /// Login page URL fragment
    pub const LOGIN_URL: &str = "/nget/train-search";
}

pub mod search {
    /// "From" station autocomplete input
    pub const FROM_STATION_INPUT: &str = "input[aria-label='From']";
    /// "To" station autocomplete input
    pub const TO_STATION_INPUT: &str = "input[aria-label='To']";
    /// Autocomplete dropdown suggestion items
    pub const AUTOCOMPLETE_OPTION: &str = "span.ui-autocomplete-list-item";
    /// Journey date picker input
    pub const DATE_INPUT: &str = "input[formcontrolname='journeyDate']";
    /// Class selection dropdown
    pub const CLASS_DROPDOWN: &str = "select#journeyClass";
    /// Quota selection dropdown
    pub const QUOTA_DROPDOWN: &str = "select#journeyQuota";
    /// "Find Trains" / search button
    pub const SEARCH_BUTTON: &str = "button.search_btn";
    /// Train list container (appears after search)
    pub const TRAIN_LIST_CONTAINER: &str = "div.train-list";
    /// Individual train row in results
    pub const TRAIN_ROW: &str = "app-train-avl-enq";
    /// Train number text within a row
    pub const TRAIN_NUMBER: &str = "strong.train-heading";
    /// Availability link/button for a specific class in a train row
    pub const AVAILABILITY_LINK: &str = "td.pre-avl";
    /// "Book Now" button (appears after clicking availability)
    pub const BOOK_NOW_BUTTON: &str = "button.btnDefault.train_Search";
}

pub mod booking {
    /// Passenger name input (indexed per passenger)
    pub const PASSENGER_NAME: &str = "input[formcontrolname='passengerName']";
    /// Passenger age input
    pub const PASSENGER_AGE: &str = "input[formcontrolname='passengerAge']";
    /// Passenger gender dropdown
    pub const PASSENGER_GENDER: &str = "select[formcontrolname='passengerGender']";
    /// Berth preference dropdown
    pub const BERTH_PREFERENCE: &str = "select[formcontrolname='passengerBerthChoice']";
    /// Nationality input (auto-filled as "Indian" usually)
    pub const NATIONALITY: &str = "select[formcontrolname='passengerNationality']";
    /// "Add Passenger" link/button
    pub const ADD_PASSENGER_BUTTON: &str = "span.add_passenger";
    /// Auto-upgrade checkbox
    pub const AUTO_UPGRADE_CHECKBOX: &str = "input#autoUpgradation";
    /// "Continue" / submit passenger details button
    pub const CONTINUE_BUTTON: &str = "button.train_Search.btnDefault";
    /// Aadhaar OTP input field
    pub const AADHAAR_OTP_INPUT: &str = "input[formcontrolname='aadhaarOtp']";
    /// Aadhaar OTP verify/submit button
    pub const AADHAAR_VERIFY_BUTTON: &str = "button.btn.btn-primary";
    /// Master list passenger selection (if using pre-saved passengers)
    pub const MASTER_LIST_CHECKBOX: &str = "input[type='checkbox'].master-passenger";
}

pub mod payment {
    /// UPI payment radio button / option
    pub const UPI_OPTION: &str = "div.bank-type";
    /// UPI fallback — often a specific bank type container
    pub const UPI_OPTION_ALT: &str = "div[class*='upi']";
    /// eWallet payment option
    pub const EWALLET_OPTION: &str = "div.bank-type";
    /// UPI VPA (Virtual Payment Address) input
    pub const UPI_VPA_INPUT: &str = "input[formcontrolname='vpa']";
    /// Pay / submit payment button
    pub const PAY_BUTTON: &str = "button.btn.btn-primary.pay-btn";
    /// Payment processing overlay/indicator
    pub const PAYMENT_PROCESSING: &str = "div.payment-processing";
    /// PNR number on confirmation page
    pub const CONFIRMATION_PNR: &str = "td.pnr-val";
    /// Booking success message container
    pub const SUCCESS_CONTAINER: &str = "div.booking-confirmed";
    /// Alternative PNR display
    pub const PNR_TEXT: &str = "span.pnr-number";
}

/// General page indicators
pub mod page {
    /// Loading spinner / overlay
    pub const LOADING_SPINNER: &str = "div.loadingText";
    /// Error message toast/alert
    pub const ERROR_ALERT: &str = "div.alert-danger";
    /// Success message toast/alert
    pub const SUCCESS_ALERT: &str = "div.alert-success";
    /// Session timeout modal
    pub const SESSION_TIMEOUT: &str = "div.modal-content";
    /// Any modal close button
    pub const MODAL_CLOSE: &str = "button.close";
}
