use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;

pub struct DisplaySlice<'a, D: Display> {
    items: &'a [D]
}
impl<'a, D: Display> Display for DisplaySlice<'a, D> where D: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let len = self.items.len();
        for (index, item) in self.items.iter().enumerate() {
            write!(f, "{item}")?;
            if index != len - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'a, D: Display> From<&'a [D]> for DisplaySlice<'a, D> {
    fn from(items: &'a [D]) -> Self {
        DisplaySlice { items }
    }
}

pub struct DisplayRangeInclusive<'a, D: Display> {
    range: &'a RangeInclusive<D>
}

impl<'a, D: Display> From<&'a RangeInclusive<D>> for DisplayRangeInclusive<'a, D> {
    fn from(range: &'a RangeInclusive<D>) -> Self {
        DisplayRangeInclusive { range }
    }
}

impl<'a, D: Display> Display for DisplayRangeInclusive<'a, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.range.start(), self.range.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_slice() {
        let data = vec![4, 5, 6, 8];
        assert_eq!("[4, 5, 6, 8]", format!("{}", DisplaySlice::from(&*data)));

        let data: Vec<usize> = vec![];
        assert_eq!("[]", format!("{}", DisplaySlice::from(&*data)));
    }
}