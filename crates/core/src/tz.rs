use crate::Error;
use jiff::tz::TimeZone;
use jiff::Timestamp;

pub fn system_zone() -> String {
    TimeZone::try_system()
        .ok()
        .and_then(|tz| tz.iana_name().map(String::from))
        .unwrap_or_else(|| "UTC".into())
}

pub fn zone(name: &str) -> Result<TimeZone, Error> {
    TimeZone::get(name).map_err(|_| Error::Zone(name.into()))
}

const MARGIN: i64 = 2 * 86_400;

pub(crate) fn shift(origin: &TimeZone, display: &TimeZone, ts: &[i64]) -> (Vec<i64>, u32, u32) {
    let (Some(&lo), Some(&hi)) = (ts.first(), ts.last()) else {
        return (Vec::new(), 0, 0);
    };
    let (mut points, mut folds, mut gaps) = (vec![lo - MARGIN], Vec::new(), Vec::new());
    let in_range = |t: i64| t <= hi + MARGIN;

    if let Ok(start) = Timestamp::from_second(lo - MARGIN) {
        for tr in origin.following(start) {
            let t = tr.timestamp().as_second();
            if !in_range(t) {
                break;
            }
            let after = i64::from(tr.offset().seconds());
            let before = i64::from(
                origin
                    .to_offset(Timestamp::from_second(t - 1).unwrap_or(start))
                    .seconds(),
            );
            let (a, b) = (t + before, t + after);
            points.extend([a, b]);
            match after.cmp(&before) {
                std::cmp::Ordering::Greater => gaps.push((a, b)),
                std::cmp::Ordering::Less => folds.push((b, a)),
                std::cmp::Ordering::Equal => {}
            }
        }
        for tr in display.following(start) {
            let t = tr.timestamp();
            if !in_range(t.as_second()) {
                break;
            }
            points.push(t.as_second() + i64::from(origin.to_offset(t).seconds()));
        }
    }
    points.sort_unstable();
    points.dedup();

    let delta_at = |p: i64| -> i64 {
        let Ok(naive) = Timestamp::from_second(p) else {
            return 0;
        };
        let dt = naive.to_zoned(TimeZone::UTC).datetime();
        match origin.to_ambiguous_timestamp(dt).compatible() {
            Ok(inst) => inst.as_second() + i64::from(display.to_offset(inst).seconds()) - p,
            Err(_) => 0,
        }
    };
    let deltas: Vec<i64> = points.iter().map(|&p| delta_at(p)).collect();

    let mut k = 0;
    let local = ts
        .iter()
        .map(|&t| {
            while k + 1 < points.len() && points[k + 1] <= t {
                k += 1;
            }
            t + deltas[k]
        })
        .collect();
    let count = |w: &[(i64, i64)]| {
        w.iter()
            .map(|&(a, b)| ts.partition_point(|&t| t < b) - ts.partition_point(|&t| t < a))
            .sum::<usize>() as u32
    };
    (local, count(&folds), count(&gaps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civil::days_from_civil;

    fn at(y: i64, m: i64, d: i64, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3600 + min * 60
    }

    #[test]
    fn same_zone_is_identity() {
        let tz = zone("America/Sao_Paulo").unwrap();
        let ts = [at(2026, 1, 6, 9, 0), at(2026, 10, 26, 23, 59)];
        assert_eq!(shift(&tz, &tz, &ts), (ts.to_vec(), 0, 0));
    }

    #[test]
    fn fixed_offsets() {
        let sp = zone("America/Sao_Paulo").unwrap();
        let utc = zone("UTC").unwrap();
        let ts = [at(2026, 1, 6, 22, 30)];
        assert_eq!(shift(&sp, &utc, &ts).0, [at(2026, 1, 7, 1, 30)]);
        assert_eq!(shift(&utc, &sp, &ts).0, [at(2026, 1, 6, 19, 30)]);
    }

    #[test]
    fn dst_gap_and_fold_are_counted() {
        let ny = zone("America/New_York").unwrap();
        let utc = zone("UTC").unwrap();
        let ts = [
            at(2026, 3, 8, 1, 30),
            at(2026, 3, 8, 2, 30),
            at(2026, 3, 8, 12, 0),
            at(2026, 11, 1, 1, 30),
            at(2026, 11, 1, 3, 0),
        ];
        let (local, folds, gaps) = shift(&ny, &utc, &ts);
        assert_eq!((folds, gaps), (1, 1));
        assert_eq!(local[0], at(2026, 3, 8, 6, 30));
        assert_eq!(local[1], at(2026, 3, 8, 7, 30));
        assert_eq!(local[2], at(2026, 3, 8, 16, 0));
        assert_eq!(local[3], at(2026, 11, 1, 5, 30));
        assert_eq!(local[4], at(2026, 11, 1, 8, 0));
    }

    #[test]
    fn unknown_zone_is_error() {
        assert!(zone("Marte/Olympus").is_err());
    }
}
