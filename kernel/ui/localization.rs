//! Bounded localization identifiers and deterministic English fallback.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum StringId {
    ProductName = 1,
    Welcome,
    PrivacyTagline,
    PasswordPlaceholder,
    SignIn,
    SecurityKey,
    AddUser,
    ForgotPassword,
    Options,
    ShutDown,
    Restart,
    Network,
    Accessibility,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LocaleId(pub [u8; 8]);

pub const EN_US: LocaleId = LocaleId(*b"en-US\0\0\0");

// ------------------------=
// FUNC: resolve
// DESC: Resolves semantic text IDs with a deterministic built-in English fallback.
// ------------------=
pub const fn resolve(_locale: LocaleId, id: StringId) -> &'static [u8] {
    match id {
        StringId::ProductName => b"InfinityOS",
        StringId::Welcome => b"Welcome to InfinityOS",
        StringId::PrivacyTagline => b"Secure. Private. Limitless.",
        StringId::PasswordPlaceholder => b"Enter your password",
        StringId::SignIn => b"Sign In",
        StringId::SecurityKey => b"Sign In with Security Key",
        StringId::AddUser => b"Add User",
        StringId::ForgotPassword => b"Forgot Password?",
        StringId::Options => b"Options",
        StringId::ShutDown => b"Shut Down",
        StringId::Restart => b"Restart",
        StringId::Network => b"Network",
        StringId::Accessibility => b"Accessibility",
    }
}
