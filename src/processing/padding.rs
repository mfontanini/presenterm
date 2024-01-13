use std::iter;

pub(crate) fn pad_right(number: usize, pad: usize) -> String {
    let line_number_width = number.ilog10() as usize + 1;
    let number_padding = pad - line_number_width;

    let mut output = String::new();
    output.extend(iter::repeat(' ').take(number_padding));
    output.push_str(&number.to_string());
    output
}
