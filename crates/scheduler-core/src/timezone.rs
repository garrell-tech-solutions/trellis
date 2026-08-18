//! Whether a submitted name is a real IANA timezone. `D-single-user`: the
//! owner has exactly one, for the whole product -- reading or writing where
//! it is stored is the adapter's job (`settings::store`); this is only the
//! rule that survives changing HTTP for something else.

/// A submitted name that names no known IANA timezone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTimezone;

/// The zone `name` names, or [`UnknownTimezone`] if it names none.
/// `#60`'s `free_time` module is the first caller that needs the resolved
/// zone itself rather than only the yes/no answer.
pub fn resolve(name: &str) -> Result<jiff::tz::TimeZone, UnknownTimezone> {
    jiff::tz::TimeZone::get(name).map_err(|_| UnknownTimezone)
}

/// Whether `name` resolves against the system's timezone database.
pub fn validate(name: &str) -> Result<(), UnknownTimezone> {
    resolve(name).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_iana_zone_is_valid() {
        assert_eq!(validate("Europe/London"), Ok(()));
    }

    #[test]
    fn utc_is_valid() {
        assert_eq!(validate("UTC"), Ok(()));
    }

    #[test]
    fn a_name_that_names_no_zone_is_rejected() {
        assert_eq!(validate("Mars/Olympus"), Err(UnknownTimezone));
    }

    #[test]
    fn a_near_miss_name_is_rejected() {
        assert_eq!(validate("Europe/Londonn"), Err(UnknownTimezone));
    }

    #[test]
    fn hostile_text_is_rejected_not_panicked_on() {
        assert_eq!(
            validate("<script>alert('boom')</script>"),
            Err(UnknownTimezone)
        );
    }
}
