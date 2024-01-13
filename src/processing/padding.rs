use std::iter;

pub(crate) struct NumberPadder {
    width: usize,
}

impl NumberPadder {
    pub(crate) fn new(upper_bound: usize) -> Self {
        let width = upper_bound.ilog10() as usize + 1;
        Self { width }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn pad_right(&self, number: usize) -> String {
        let line_number_width = number.ilog10() as usize + 1;
        let number_padding = self.width - line_number_width;

        let mut output = String::with_capacity(self.width);
        output.extend(iter::repeat(' ').take(number_padding));
        output.push_str(&number.to_string());
        output
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(&[1, 2], &["1", "2"])]
    #[case(&[1, 9], &["1", "9"])]
    #[case(&[1, 10], &[" 1", "10"])]
    #[case(&[1, 10, 100], &["  1", " 10", "100"])]
    fn right_padding(#[case] numbers: &[usize], #[case] expected: &[&str]) {
        let max = numbers.iter().max().expect("no numbers");
        let padder = NumberPadder::new(*max);
        let rendered: Vec<_> = numbers.iter().map(|n| padder.pad_right(*n)).collect();
        assert_eq!(rendered, expected);
    }
}
