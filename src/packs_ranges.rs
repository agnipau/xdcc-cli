pub type PackRange = std::ops::RangeInclusive<i32>;

#[derive(Clone)]
pub struct PacksRanges(pub Vec<PackRange>);

impl PacksRanges {
    pub fn from(s: &[&str]) -> Self {
        let mut ranges = Vec::new();
        for pack in s {
            if pack.find(|p| p == '-').is_some() {
                let mut parts = pack.split('-').take(2);
                if let Some(Ok(fst)) = parts.next().map(|p| p.parse::<i32>()) {
                    if let Some(Ok(snd)) = parts.next().map(|p| p.parse::<i32>()) {
                        if snd >= fst {
                            ranges.push(fst..=snd)
                        }
                    }
                }
            } else if let Ok(p) = pack.parse::<i32>() {
                ranges.push(p..=p);
            }
        }
        Self(ranges)
    }
}
