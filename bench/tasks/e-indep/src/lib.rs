//! Four conversions that have nothing to do with each other.
//!
//! This is the control for Task C. There, `classify` calls `glob_match` calls
//! `match_seg` calls `split_segs`, so a coder implementing one slice has the
//! next one in front of it — which is the easiest possible case for the slice
//! overflow [ADR-0058] is built on. Here nothing calls anything, and the four
//! are deliberately unlike one another: a lookup table, a string loop, bit
//! packing, and a validator.
//!
//! [ADR-0058]: https://github.com/baphomello/corifeu/blob/main/docs/adr/0058-slice-boundaries-are-observed.md

/// Roman numerals for 1 to 3999. Anything outside that range is an empty
/// string, because there is no Roman numeral for it.
pub fn roman(n: u32) -> String {
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    if n == 0 || n > 3999 {
        return String::new();
    }
    let mut left = n;
    let mut out = String::new();
    for (value, numeral) in TABLE {
        while left >= value {
            out.push_str(numeral);
            left -= value;
        }
    }
    out
}

/// Greedy word wrap. Words are whatever is between runs of whitespace, and a
/// word longer than `width` gets a line to itself rather than being split.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Base32 as RFC 4648 defines it: the alphabet `A-Z2-7`, five bytes to eight
/// characters, padded with `=` to a multiple of eight.
pub fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    for group in bytes.chunks(5) {
        let mut buffer: u64 = 0;
        for i in 0..5 {
            buffer = (buffer << 8) | u64::from(group.get(i).copied().unwrap_or(0));
        }
        // Each input byte contributes 8 bits; every 5 bits is one character,
        // so n bytes round up to 1 + n * 8 / 5 characters.
        let characters = (group.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < characters {
                let index = ((buffer >> (35 - i * 5)) & 0b11111) as usize;
                out.push(ALPHABET[index] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The Luhn checksum. Spaces may appear anywhere and are ignored; any other
/// character makes the input invalid, as does fewer than two digits.
pub fn luhn(digits: &str) -> bool {
    let mut values = Vec::new();
    for character in digits.chars() {
        if character == ' ' {
            continue;
        }
        match character.to_digit(10) {
            Some(value) => values.push(value),
            None => return false,
        }
    }
    if values.len() < 2 {
        return false;
    }
    let sum: u32 = values
        .iter()
        .rev()
        .enumerate()
        .map(|(position, value)| {
            if position % 2 == 0 {
                *value
            } else if *value > 4 {
                value * 2 - 9
            } else {
                value * 2
            }
        })
        .sum();
    sum % 10 == 0
}

#[cfg(test)]
mod slice1_roman {
    use super::*;

    #[test] fn single_symbols() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(5), "V");
        assert_eq!(roman(1000), "M");
    }
    #[test] fn additive_runs() {
        assert_eq!(roman(3), "III");
        assert_eq!(roman(2000), "MM");
    }
    #[test] fn subtractive_pairs() {
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(40), "XL");
        assert_eq!(roman(900), "CM");
    }
    #[test] fn mixed_numbers() {
        assert_eq!(roman(1994), "MCMXCIV");
        assert_eq!(roman(2024), "MMXXIV");
    }
    #[test] fn the_largest_value() {
        assert_eq!(roman(3999), "MMMCMXCIX");
    }
    #[test] fn out_of_range_is_empty() {
        assert_eq!(roman(0), "");
        assert_eq!(roman(4000), "");
    }
}

#[cfg(test)]
mod slice2_wrap {
    use super::*;

    #[test] fn short_text_is_one_line() {
        assert_eq!(wrap("hello there", 20), vec!["hello there"]);
    }
    #[test] fn breaks_at_the_width() {
        assert_eq!(wrap("aaa bbb ccc", 7), vec!["aaa bbb", "ccc"]);
    }
    #[test] fn a_word_longer_than_the_width_gets_its_own_line() {
        assert_eq!(wrap("a verylongword b", 4), vec!["a", "verylongword", "b"]);
    }
    #[test] fn runs_of_whitespace_collapse() {
        assert_eq!(wrap("a   b\n\tc", 5), vec!["a b c"]);
    }
    #[test] fn empty_text_has_no_lines() {
        assert!(wrap("", 10).is_empty());
        assert!(wrap("   ", 10).is_empty());
    }
    #[test] fn a_line_may_reach_the_width_exactly() {
        assert_eq!(wrap("ab cd ef", 5), vec!["ab cd", "ef"]);
    }
    #[test] fn width_is_counted_in_characters_not_bytes() {
        assert_eq!(wrap("ação pão", 8), vec!["ação pão"]);
    }
}

#[cfg(test)]
mod slice3_base32 {
    use super::*;

    #[test] fn nothing_encodes_to_nothing() {
        assert_eq!(base32(b""), "");
    }
    #[test] fn one_byte_pads_to_eight() {
        assert_eq!(base32(b"f"), "MY======");
    }
    #[test] fn the_rfc_vectors() {
        assert_eq!(base32(b"fo"), "MZXQ====");
        assert_eq!(base32(b"foo"), "MZXW6===");
        assert_eq!(base32(b"foob"), "MZXW6YQ=");
    }
    #[test] fn a_full_group_has_no_padding() {
        assert_eq!(base32(b"fooba"), "MZXW6YTB");
    }
    #[test] fn more_than_one_group() {
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI======");
    }
    #[test] fn bytes_are_not_text() {
        assert_eq!(base32(&[0x00]), "AA======");
        assert_eq!(base32(&[0xff, 0xff]), "777Q====");
    }
}

#[cfg(test)]
mod slice4_luhn {
    use super::*;

    #[test] fn a_valid_number() {
        assert!(luhn("4539 3195 0343 6467"));
    }
    #[test] fn an_invalid_number() {
        assert!(!luhn("8273 1232 7352 0569"));
    }
    #[test] fn spaces_anywhere_are_ignored() {
        assert!(luhn("059"));
        assert!(luhn("0 5 9"));
        assert!(luhn(" 059 "));
    }
    #[test] fn anything_but_a_digit_or_a_space_is_invalid() {
        assert!(!luhn("059a"));
        assert!(!luhn("055-444-285"));
        assert!(!luhn("0\u{332}59"));
    }
    #[test] fn fewer_than_two_digits_is_invalid() {
        assert!(!luhn(""));
        assert!(!luhn("0"));
        assert!(!luhn(" 0 "));
    }
    #[test] fn doubling_above_nine_subtracts_nine() {
        assert!(luhn("091"));
    }
    #[test] fn a_long_string_of_zeroes_is_valid() {
        assert!(luhn("0000 0000 0000 0000"));
    }
}
