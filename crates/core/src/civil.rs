use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub fn weekday(day: i64) -> usize {
    (day + 3).rem_euclid(7) as usize
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(pub i64);

impl Day {
    pub fn parse(s: &str) -> Option<Day> {
        let mut it = s.trim().splitn(3, '-').map(|p| p.parse::<i64>().ok());
        let (y, m, d) = (it.next()??, it.next()??, it.next()??);
        let day = Day(days_from_civil(y, m, d));
        (day.ymd() == (y, m as u32, d as u32)).then_some(day)
    }

    pub fn ymd(self) -> (i64, u32, u32) {
        civil_from_days(self.0)
    }

    pub fn br(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{d:02}/{m:02}/{y:04}")
    }
}

impl std::fmt::Display for Day {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

impl Serialize for Day {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Day {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Day::parse(&s).ok_or_else(|| serde::de::Error::custom("data inválida"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for z in -800_000..800_000 {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, i64::from(m), i64::from(d)), z);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2026, 10, 26), 20_752);
    }

    #[test]
    fn day_text() {
        let d = Day::parse("2026-10-26").unwrap();
        assert_eq!(d.0, 20_752);
        assert_eq!(d.to_string(), "2026-10-26");
        assert_eq!(d.br(), "26/10/2026");
        assert_eq!(weekday(d.0), 0);
        assert_eq!(Day::parse("2026-02-30"), None);
        assert_eq!(Day::parse("x"), None);
    }
}
