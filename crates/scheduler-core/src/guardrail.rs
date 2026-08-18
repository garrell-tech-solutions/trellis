//! A life area's weekly guardrail: the hours its work may be scheduled in
//! (`D-life-area-owns-its-time`, #59). Every rule here survives changing
//! HTTP for something else -- is a submitted band well-formed, does it
//! overlap a life area's own existing bands, did a submission choose a
//! guardrail or pool-only at all -- so none of it lives at the adapter.
//!
//! Times are minutes-since-midnight, civil wall-clock (`T-jiff-epoch-millis`:
//! a guardrail is never an instant). Nothing here converts one to an instant
//! -- that is `#60`'s free-interval algebra, reading what this slice writes.

/// One of the seven days a band may name. Ordered `Mon..Sun` so a life
/// area's bands display in the same order the owner thinks in, regardless
/// of insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// Every variant paired with its stored/displayed name, in `Mon..Sun` order
/// -- a table, not a match, so a seventh arm's-worth of branches does not
/// count against `T-complexity-8` for carrying no logic beyond the lookup
/// itself (`as_str`/`parse` are each one `.find()` over it).
const WEEKDAYS: [(Weekday, &str); 7] = [
    (Weekday::Mon, "Mon"),
    (Weekday::Tue, "Tue"),
    (Weekday::Wed, "Wed"),
    (Weekday::Thu, "Thu"),
    (Weekday::Fri, "Fri"),
    (Weekday::Sat, "Sat"),
    (Weekday::Sun, "Sun"),
];

impl Weekday {
    pub fn as_str(self) -> &'static str {
        WEEKDAYS
            .iter()
            .find(|(day, _)| *day == self)
            .map(|(_, name)| *name)
            .expect("every Weekday variant is listed in WEEKDAYS")
    }

    pub fn parse(value: &str) -> Option<Self> {
        WEEKDAYS
            .iter()
            .find(|(_, name)| *name == value)
            .map(|(day, _)| *day)
    }
}

/// One well-formed band: a single weekday and the civil minutes-of-day it
/// spans. A five-weekday submission ("Mon-Fri 09:00-17:00") is five of
/// these, one per day -- storage and this type both stay one row per
/// weekday; only the form that authors them spares the owner five repeats
/// of the same two times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    pub weekday: Weekday,
    pub start_minutes: i64,
    pub end_minutes: i64,
}

/// The band-shaped fields a submission carries, before anything has decided
/// whether it names a guardrail band, pool-only, or neither. `weekdays` may
/// be empty (no day was checked); `start_minutes`/`end_minutes` are `None`
/// when the corresponding field was absent or did not parse as a time --
/// the adapter's job, `T-jiff-epoch-millis` elsewhere in this codebase does
/// the analogous parse for a deadline.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GuardrailFields {
    pub weekdays: Vec<Weekday>,
    pub start_minutes: Option<i64>,
    pub end_minutes: Option<i64>,
    pub pool_only: bool,
}

/// Why a guardrail submission was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardrailRejection {
    /// Neither a band nor pool-only was submitted -- the life area would be
    /// left exactly as unwalled as it started.
    ChooseOne,
    /// A band was attempted (some time field was given) but named no day.
    NoWeekday,
    /// A band's times are missing, unparseable, or its end does not follow
    /// its start.
    InvalidTimes,
}

/// What a well-formed submission asks for: a guardrail band (one per
/// weekday named), or pool-only. Never both -- `pool_only` wins outright,
/// since a submission that checked it is not attempting a band at all
/// (`from_fields`'s own ordering, not a rule split across callers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WellFormedGuardrailSubmission {
    Bands(Vec<Band>),
    PoolOnly,
}

/// Whether `fields` attempted nothing at all -- no day, no start, no end.
/// Split out so `from_fields`'s own match carries one condition per branch,
/// not this one spelled inline (`T-complexity-8`: extract logic out of a
/// function that is otherwise a match, do not flatten the match).
fn attempted_nothing(fields: &GuardrailFields) -> bool {
    fields.weekdays.is_empty() && fields.start_minutes.is_none() && fields.end_minutes.is_none()
}

/// The band's own times, well-formed -- both present and end strictly after
/// start.
fn well_formed_times(fields: &GuardrailFields) -> Result<(i64, i64), GuardrailRejection> {
    match (fields.start_minutes, fields.end_minutes) {
        (Some(start), Some(end)) if end > start => Ok((start, end)),
        _ => Err(GuardrailRejection::InvalidTimes),
    }
}

impl WellFormedGuardrailSubmission {
    /// Pool-only first (an explicit choice needs nothing else to be true),
    /// then whether anything was attempted at all, then the band's own
    /// shape -- the same fixed-order composition `WellFormedTriage::
    /// from_fields` uses, so every adapter gets the same rejection for the
    /// same submission without re-deriving the order itself.
    pub fn from_fields(fields: &GuardrailFields) -> Result<Self, GuardrailRejection> {
        if fields.pool_only {
            return Ok(Self::PoolOnly);
        }
        if attempted_nothing(fields) {
            return Err(GuardrailRejection::ChooseOne);
        }
        if fields.weekdays.is_empty() {
            return Err(GuardrailRejection::NoWeekday);
        }
        let (start, end) = well_formed_times(fields)?;
        Ok(Self::Bands(
            fields
                .weekdays
                .iter()
                .map(|&weekday| Band {
                    weekday,
                    start_minutes: start,
                    end_minutes: end,
                })
                .collect(),
        ))
    }
}

/// What makes two stored weekday-rows the same band the owner authored:
/// they span the same civil minutes. A "Mon-Fri 09:00-17:00" submission is
/// five [`Band`]s sharing one span.
///
/// It has a name because **two places act on it and they must agree**: the
/// management page groups rows into bands by span to display them, and
/// removing a band deletes every row sharing its span. Those were two
/// independent statements of one rule -- one in Rust for the display, one
/// in SQL for the delete -- with nothing tying them, so the band a reader
/// sees and the band the Remove button takes were defined separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandSpan {
    pub start_minutes: i64,
    pub end_minutes: i64,
}

impl Band {
    pub fn span(self) -> BandSpan {
        BandSpan {
            start_minutes: self.start_minutes,
            end_minutes: self.end_minutes,
        }
    }
}

/// One band as the owner authored it: its span, every weekday it covers in
/// `Mon..Sun` order, and the `tag` its first member carried.
///
/// `tag` is deliberately opaque here. The adapter passes row ids and uses
/// the group's own as the handle its Remove control posts against; the core
/// only promises which member's tag it keeps, and knows nothing about what
/// a tag is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBand<T> {
    pub tag: T,
    pub span: BandSpan,
    pub weekdays: Vec<Weekday>,
}

/// Folds per-weekday bands back into the bands they were authored as.
///
/// Groups appear in the order their first member arrives, and each group's
/// weekdays come out sorted `Mon..Sun` regardless of the order they were
/// written in -- a band reads as the owner thinks of it, not as the rows
/// happened to land.
///
/// Two submissions that chose the same times merge into one band, which is
/// deliberate: they are indistinguishable once stored, and the alternative
/// is a page showing two rows whose Remove buttons each take both.
pub fn group<T>(bands: impl IntoIterator<Item = (T, Band)>) -> Vec<AuthoredBand<T>> {
    let mut grouped: Vec<AuthoredBand<T>> = Vec::new();
    for (tag, band) in bands {
        match grouped.iter_mut().find(|group| group.span == band.span()) {
            Some(group) => group.weekdays.push(band.weekday),
            None => grouped.push(AuthoredBand {
                tag,
                span: band.span(),
                weekdays: vec![band.weekday],
            }),
        }
    }
    for group in &mut grouped {
        group.weekdays.sort();
    }
    grouped
}

/// Whether `candidate` overlaps any of `existing` -- the same weekday and
/// intervals that genuinely intersect. Half-open on purpose: `09:00-12:00`
/// and `12:00-17:00` touch without overlapping, and both are kept
/// (`guardrails-touching-allowed-07`); a life area's guardrail is a mask,
/// not a schedule that would need to reject a shared boundary.
pub fn overlaps(existing: &[Band], candidate: &Band) -> bool {
    existing.iter().any(|band| {
        band.weekday == candidate.weekday
            && band.start_minutes < candidate.end_minutes
            && candidate.start_minutes < band.end_minutes
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_folds_one_submission_back_into_one_band() {
        let bands = vec![
            (1, band(Weekday::Mon, 540, 1020)),
            (2, band(Weekday::Tue, 540, 1020)),
            (3, band(Weekday::Wed, 540, 1020)),
        ];

        let grouped = group(bands);

        assert_eq!(grouped.len(), 1);
        assert_eq!(
            grouped[0].weekdays,
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed]
        );
        assert_eq!(grouped[0].tag, 1, "the first member names the group");
    }

    #[test]
    fn group_keeps_different_spans_apart() {
        let bands = vec![
            (1, band(Weekday::Mon, 360, 420)),
            (2, band(Weekday::Sat, 540, 660)),
        ];

        let grouped = group(bands);

        assert_eq!(grouped.len(), 2);
        assert_eq!(
            grouped[0].span,
            BandSpan {
                start_minutes: 360,
                end_minutes: 420
            }
        );
    }

    #[test]
    fn group_sorts_each_bands_weekdays_monday_first() {
        let bands = vec![
            (1, band(Weekday::Fri, 540, 600)),
            (2, band(Weekday::Mon, 540, 600)),
            (3, band(Weekday::Wed, 540, 600)),
        ];

        let grouped = group(bands);

        assert_eq!(
            grouped[0].weekdays,
            vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]
        );
    }

    /// Two submissions that chose the same times are one band afterwards.
    /// Recorded as a test rather than left implicit, because it is the one
    /// case where grouping is not simply undoing a fan-out.
    #[test]
    fn group_merges_separately_authored_bands_that_share_a_span() {
        let bands = vec![
            (1, band(Weekday::Mon, 540, 1020)),
            (9, band(Weekday::Thu, 540, 1020)),
        ];

        let grouped = group(bands);

        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].weekdays, vec![Weekday::Mon, Weekday::Thu]);
    }

    #[test]
    fn group_of_nothing_is_no_bands() {
        assert_eq!(group(Vec::<(i64, Band)>::new()), Vec::new());
    }

    #[test]
    fn a_bands_span_is_its_times_without_its_weekday() {
        assert_eq!(
            band(Weekday::Sun, 600, 660).span(),
            BandSpan {
                start_minutes: 600,
                end_minutes: 660
            }
        );
    }

    #[test]
    fn weekday_parse_round_trips_every_variant() {
        for day in [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ] {
            assert_eq!(Weekday::parse(day.as_str()), Some(day));
        }
    }

    #[test]
    fn weekday_parse_rejects_an_unknown_name() {
        assert_eq!(Weekday::parse("Someday"), None);
    }

    #[test]
    fn weekdays_sort_monday_first() {
        let mut days = vec![Weekday::Sun, Weekday::Wed, Weekday::Mon];
        days.sort();
        assert_eq!(days, vec![Weekday::Mon, Weekday::Wed, Weekday::Sun]);
    }

    fn band(weekday: Weekday, start: i64, end: i64) -> Band {
        Band {
            weekday,
            start_minutes: start,
            end_minutes: end,
        }
    }

    #[test]
    fn from_fields_pool_only_wins_even_with_band_fields_present() {
        let fields = GuardrailFields {
            weekdays: vec![Weekday::Mon],
            start_minutes: Some(540),
            end_minutes: Some(1020),
            pool_only: true,
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Ok(WellFormedGuardrailSubmission::PoolOnly)
        );
    }

    #[test]
    fn from_fields_with_nothing_submitted_asks_the_owner_to_choose_one() {
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&GuardrailFields::default()),
            Err(GuardrailRejection::ChooseOne)
        );
    }

    #[test]
    fn from_fields_with_times_but_no_weekday_is_rejected() {
        let fields = GuardrailFields {
            start_minutes: Some(540),
            end_minutes: Some(1020),
            ..GuardrailFields::default()
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Err(GuardrailRejection::NoWeekday)
        );
    }

    #[test]
    fn from_fields_with_a_weekday_but_no_times_has_invalid_times() {
        let fields = GuardrailFields {
            weekdays: vec![Weekday::Mon],
            ..GuardrailFields::default()
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Err(GuardrailRejection::InvalidTimes)
        );
    }

    #[test]
    fn from_fields_rejects_an_end_before_its_start() {
        let fields = GuardrailFields {
            weekdays: vec![Weekday::Mon],
            start_minutes: Some(1020),
            end_minutes: Some(540),
            ..GuardrailFields::default()
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Err(GuardrailRejection::InvalidTimes)
        );
    }

    #[test]
    fn from_fields_rejects_an_end_equal_to_its_start() {
        let fields = GuardrailFields {
            weekdays: vec![Weekday::Mon],
            start_minutes: Some(540),
            end_minutes: Some(540),
            ..GuardrailFields::default()
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Err(GuardrailRejection::InvalidTimes)
        );
    }

    #[test]
    fn from_fields_produces_one_band_per_named_weekday() {
        let fields = GuardrailFields {
            weekdays: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
            start_minutes: Some(360),
            end_minutes: Some(420),
            ..GuardrailFields::default()
        };
        assert_eq!(
            WellFormedGuardrailSubmission::from_fields(&fields),
            Ok(WellFormedGuardrailSubmission::Bands(vec![
                band(Weekday::Mon, 360, 420),
                band(Weekday::Wed, 360, 420),
                band(Weekday::Fri, 360, 420),
            ]))
        );
    }

    #[test]
    fn overlaps_is_true_for_genuinely_intersecting_bands_on_the_same_day() {
        let existing = [band(Weekday::Mon, 540, 720)];
        assert!(overlaps(&existing, &band(Weekday::Mon, 660, 1020)));
    }

    #[test]
    fn overlaps_is_false_for_bands_that_only_touch() {
        let existing = [band(Weekday::Mon, 540, 720)];
        assert!(!overlaps(&existing, &band(Weekday::Mon, 720, 1020)));
    }

    #[test]
    fn overlaps_is_false_for_the_same_hours_on_a_different_day() {
        let existing = [band(Weekday::Mon, 540, 720)];
        assert!(!overlaps(&existing, &band(Weekday::Tue, 540, 720)));
    }

    #[test]
    fn overlaps_is_false_against_no_existing_bands() {
        assert!(!overlaps(&[], &band(Weekday::Mon, 540, 720)));
    }
}
