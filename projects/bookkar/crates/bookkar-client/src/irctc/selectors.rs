/// IRCTC CSS selectors — single source of truth.
///
/// These selectors target IRCTC's Angular-based frontend elements.
/// They must be verified and updated by inspecting the live IRCTC DOM.
/// IRCTC may change their DOM structure at any time — when automation
/// breaks, check and update these selectors first.

pub mod login {
    /// Login trigger button / link to open modal dialog
    pub const LOGIN_TRIGGER: &str = "button.btn-login, a.loginText, button[aria-label*='Login'], a[aria-label*='Login']";
    /// Username text input on the login page / dialog
    pub const USERNAME_INPUT: &str = "input#username, input[formcontrolname='userid']";
    /// Password text input on the login page / dialog
    pub const PASSWORD_INPUT: &str = "input#password, input[formcontrolname='password']";
    /// The main login submit button
    pub const LOGIN_BUTTON: &str = ".login-dialog button[type='submit'], button.btn-action.btn-login[type='submit'], button.btn-action.btn-login, app-login button[type='submit']";
    /// CAPTCHA image element (may not always be present)
    pub const CAPTCHA_IMAGE: &str = "img.captcha-img, img[src*='captcha'], .captcha-img, app-captcha";
    /// CAPTCHA text input
    pub const CAPTCHA_INPUT: &str = "input[formcontrolname='captcha'], input#captcha, input#nlpAnswer, input[placeholder*='captcha' i]";
    /// OTP input field (appears after login submit)
    pub const OTP_INPUT: &str = "input#otp";
    /// OTP submit button
    pub const OTP_SUBMIT: &str = "button.btn.btn-primary";
    /// Element that indicates successful login (e.g., username badge, user menu, logout)
    pub const LOGGED_IN_INDICATOR: &str = ".nav-link-1, a.logoutText, span.user-name, a.dropdown-toggle.profile, a.loginText";
    /// Login modal container
    pub const MODAL_CONTAINER: &str = "p-dialog#newLogin, .login-dialog, app-login, .modal-dialog";
    /// Login page URL fragment
    pub const LOGIN_URL: &str = "/eticket/train-search";
}

pub mod search {
    /// "From" station combobox trigger (beta site)
    pub const FROM_STATION_COMBOBOX: &str = "[role='combobox'][aria-label='From station'], [aria-label*='From' i]";
    /// "From" station autocomplete input
    pub const FROM_STATION_INPUT: &str = "input.from-search-input, input[formcontrolname='origin'], input[aria-label='From']";
    /// "To" station combobox trigger (beta site)
    pub const TO_STATION_COMBOBOX: &str = "[role='combobox'][aria-label='To station'], [aria-label*='To' i]";
    /// "To" station autocomplete input
    pub const TO_STATION_INPUT: &str = "input.to-search-input, input[formcontrolname='destination'], input[aria-label='To']";
    /// Autocomplete dropdown suggestion items
    pub const AUTOCOMPLETE_OPTION: &str = "div[role='option'].custom-option, .custom-dropdown-panel .custom-option, span.ui-autocomplete-list-item";
    /// Journey date picker input or button
    pub const DATE_INPUT: &str = "[role='button'][aria-label='Select travel date'], input[formcontrolname='journeyDate'], p-calendar input";
    /// Class selection dropdown
    pub const CLASS_DROPDOWN: &str = "select#journeyClass";
    /// Quota selection combobox or dropdown
    pub const QUOTA_DROPDOWN: &str = "[role='combobox'][aria-label='Quota'], select#journeyQuota";
    /// Quota option items
    pub const QUOTA_OPTION: &str = "div[role='option'].custom-option, .custom-dropdown-panel .custom-option";
    /// "Find Trains" / search button
    pub const SEARCH_BUTTON: &str = "button.search-btn, button.search_btn, button[type='submit'].search-btn";
    /// Train list container (appears after search)
    pub const TRAIN_LIST_CONTAINER: &str = "div.train-list, div.train-list-container";
    /// Individual train row in results
    pub const TRAIN_ROW: &str = "app-train-avl-enq, .train-card";
    /// Train number text within a row
    pub const TRAIN_NUMBER: &str = "strong.train-heading, strong, span.train-number";
    /// Availability link/button for a specific class in a train row
    pub const AVAILABILITY_LINK: &str = "button.btn-availability, td.pre-avl, .class-card, span.date-badge";
    /// "Book Now" button (appears after clicking availability)
    pub const BOOK_NOW_BUTTON: &str = "button.btn-book, button.btn-action.btn-book, button.btnDefault.train_Search, button.search-btn";
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
