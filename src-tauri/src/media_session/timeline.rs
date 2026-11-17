const TICKS_PER_MS: i64 = 10_000;
const UNIX_EPOCH_OFFSET_MS: i64 = 11_644_473_600_000;

#[derive(Debug, PartialEq)]
pub struct Timeline {
    pub position_ms: i64,
    pub duration_ms: i64,
    pub last_updated_utc_ms: Option<i64>,
    pub seek_bounds: Option<(i64, i64)>,
    pub start_ticks: i64,
}
pub struct RawTimeline {
    pub start: i64,
    pub end: i64,
    pub position: i64,
    pub updated: i64,
    pub min_seek: i64,
    pub max_seek: i64,
}
pub fn normalize(raw: RawTimeline, playing: bool, rate: f64, now_ms: i64) -> Option<Timeline> {
    let duration_ms = raw.end.checked_sub(raw.start)?.checked_div(TICKS_PER_MS)?;
    if duration_ms <= 0 || duration_ms > 7 * 24 * 60 * 60 * 1000 {
        return None;
    }
    let last_updated_utc_ms = raw
        .updated
        .checked_div(TICKS_PER_MS)?
        .checked_sub(UNIX_EPOCH_OFFSET_MS)
        .filter(|time| *time > 0 && *time <= now_ms);
    let mut position_ms = raw
        .position
        .saturating_sub(raw.start)
        .div_euclid(TICKS_PER_MS)
        .clamp(0, duration_ms);
    if playing && let Some(updated) = last_updated_utc_ms {
        // A valid OS position is an anchor at LastUpdatedTime, not at our query time.
        let elapsed = now_ms.saturating_sub(updated).clamp(0, duration_ms);
        let rate = if rate.is_finite() {
            rate.clamp(0.0, 4.0)
        } else {
            0.0
        };
        position_ms = position_ms
            .saturating_add((elapsed as f64 * rate) as i64)
            .clamp(0, duration_ms);
    }
    let seek_bounds =
        if raw.min_seek >= raw.start && raw.max_seek <= raw.end && raw.max_seek > raw.min_seek {
            Some((
                (raw.min_seek - raw.start) / TICKS_PER_MS,
                (raw.max_seek - raw.start) / TICKS_PER_MS,
            ))
        } else {
            None
        }
        .filter(|(min, max)| max > min);
    Some(Timeline {
        position_ms,
        duration_ms,
        last_updated_utc_ms,
        seek_bounds,
        start_ticks: raw.start,
    })
}
pub fn seek_ticks(position_ms: i64, start: i64, end: i64, min: i64, max: i64) -> Option<i64> {
    let duration = end.checked_sub(start)?.checked_div(TICKS_PER_MS)?;
    let (min, max) = (min.max(start), max.min(end));
    if duration <= 0 || max <= min {
        return None;
    }
    position_ms
        .clamp(0, duration)
        .checked_mul(TICKS_PER_MS)?
        .checked_add(start)
        .map(|ticks| ticks.clamp(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw() -> RawTimeline {
        RawTimeline {
            start: 20_000_000,
            end: 120_000_000,
            position: 50_000_000,
            updated: 0,
            min_seek: 20_000_000,
            max_seek: 120_000_000,
        }
    }
    #[test]
    fn nonzero_start_ticks_and_clamping() {
        let t = normalize(raw(), false, 1.0, 0).expect("valid");
        assert_eq!((t.position_ms, t.duration_ms), (3000, 10000));
        let mut r = raw();
        r.position = i64::MAX;
        assert_eq!(
            normalize(r, false, 1.0, 0).expect("valid").position_ms,
            10000
        );
        let mut r = raw();
        r.position = i64::MIN;
        assert_eq!(normalize(r, false, 1.0, 0).expect("valid").position_ms, 0);
    }
    #[test]
    fn pause_freezes_and_playing_uses_real_anchor() {
        let now = 1_700_000_000_000i64;
        let mut r = raw();
        r.updated = (now - 2000 + UNIX_EPOCH_OFFSET_MS) * TICKS_PER_MS;
        assert_eq!(
            normalize(r, true, 1.0, now).expect("valid").position_ms,
            5000
        );
        let mut r = raw();
        r.updated = (now - 2000 + UNIX_EPOCH_OFFSET_MS) * TICKS_PER_MS;
        assert_eq!(
            normalize(r, false, 1.0, now).expect("valid").position_ms,
            3000
        );
    }
    #[test]
    fn unknown_and_invalid_timelines_are_not_fabricated() {
        let mut r = raw();
        r.end = r.start;
        assert!(normalize(r, false, 1.0, 0).is_none());
        let mut r = raw();
        r.start = i64::MIN;
        r.end = i64::MAX;
        assert!(normalize(r, false, 1.0, 0).is_none());
        let mut r = raw();
        r.max_seek = 0;
        let t = normalize(r, false, 1.0, 0).expect("valid duration");
        assert!(t.seek_bounds.is_none() && t.last_updated_utc_ms.is_none());
    }
    #[test]
    fn seek_uses_absolute_ticks_and_current_bounds() {
        assert_eq!(
            seek_ticks(3000, 20_000_000, 120_000_000, 30_000_000, 100_000_000),
            Some(50_000_000)
        );
        assert_eq!(
            seek_ticks(i64::MAX, 20_000_000, 120_000_000, 30_000_000, 100_000_000),
            Some(100_000_000)
        );
        assert_eq!(
            seek_ticks(-1, 20_000_000, 120_000_000, 30_000_000, 100_000_000),
            Some(30_000_000)
        );
        assert_eq!(seek_ticks(0, i64::MIN, i64::MAX, 0, 0), None);
    }
}
