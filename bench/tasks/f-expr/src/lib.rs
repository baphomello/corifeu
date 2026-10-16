//! Evaluate an integer arithmetic expression.
//!
//! Built for D7: [ADR-0051](../../../docs/adr/0051-checkpoint-before-every-write.md)
//! checkpoints every write and restores a state that scores worse, and across
//! round five that fired four times in 1,076 steps. Three of the four were in
//! Task E, whose slices are whole algorithms rather than four-line helpers,
//! which is the first sign the mechanism needs something to regress.
//!
//! This task is that something. `eval` holds sixteen cases that pull against
//! each other: getting precedence right is the usual way to break unary minus,
//! and making division total is the usual way to break left associativity. A
//! slice where every case is independent cannot regress; this one can.

/// Split an expression into tokens. Numbers, operators, parentheses.
///
/// Whitespace separates but is never a token. A `-` is returned as an operator
/// here even where it will turn out to be a sign: deciding that needs the
/// position, which is `eval`'s business.
fn tokens(expr: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut digits = String::new();
    for c in expr.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            out.push(std::mem::take(&mut digits));
        }
        match c {
            ' ' | '\t' => {}
            '+' | '-' | '*' | '/' | '(' | ')' => out.push(c.to_string()),
            _ => return None,
        }
    }
    if !digits.is_empty() {
        out.push(digits);
    }
    Some(out)
}

/// Evaluate `expr`, or None if it is malformed or divides by zero.
///
/// `*` and `/` bind tighter than `+` and `-`; both groups are left
/// associative. A `-` with no value before it is a sign. Division truncates
/// toward zero.
pub fn eval(expr: &str) -> Option<i64> {
    let tokens = tokens(expr)?;
    let mut at = 0;
    let value = sum(&tokens, &mut at)?;
    if at != tokens.len() {
        return None;
    }
    Some(value)
}

fn sum(tokens: &[String], at: &mut usize) -> Option<i64> {
    let mut left = product(tokens, at)?;
    while let Some(op) = tokens.get(*at) {
        let op = match op.as_str() {
            "+" => 1,
            "-" => -1,
            _ => break,
        };
        *at += 1;
        left = left.checked_add(op * product(tokens, at)?)?;
    }
    Some(left)
}

fn product(tokens: &[String], at: &mut usize) -> Option<i64> {
    let mut left = unary(tokens, at)?;
    while let Some(op) = tokens.get(*at) {
        let divide = match op.as_str() {
            "*" => false,
            "/" => true,
            _ => break,
        };
        *at += 1;
        let right = unary(tokens, at)?;
        left = if divide {
            if right == 0 {
                return None;
            }
            left.checked_div(right)?
        } else {
            left.checked_mul(right)?
        };
    }
    Some(left)
}

fn unary(tokens: &[String], at: &mut usize) -> Option<i64> {
    match tokens.get(*at)?.as_str() {
        "-" => {
            *at += 1;
            unary(tokens, at)?.checked_neg()
        }
        "+" => {
            *at += 1;
            unary(tokens, at)
        }
        "(" => {
            *at += 1;
            let inner = sum(tokens, at)?;
            if tokens.get(*at)?.as_str() != ")" {
                return None;
            }
            *at += 1;
            Some(inner)
        }
        text => {
            let value = text.parse().ok()?;
            *at += 1;
            Some(value)
        }
    }
}

#[cfg(test)]
mod slice1_tokens {
    use super::*;

    #[test] fn splits_numbers_from_operators() {
        assert_eq!(tokens("1+2"), Some(vec!["1".into(), "+".into(), "2".into()]));
    }
    #[test] fn a_number_can_have_several_digits() {
        assert_eq!(tokens("120*3"), Some(vec!["120".into(), "*".into(), "3".into()]));
    }
    #[test] fn whitespace_separates_but_is_not_a_token() {
        assert_eq!(tokens(" 1 + 2 "), Some(vec!["1".into(), "+".into(), "2".into()]));
        assert_eq!(tokens("1 2"), Some(vec!["1".into(), "2".into()]));
    }
    #[test] fn parentheses_are_tokens() {
        assert_eq!(tokens("(1)"), Some(vec!["(".into(), "1".into(), ")".into()]));
    }
    #[test] fn an_unknown_character_is_refused() {
        assert_eq!(tokens("1 & 2"), None);
        assert_eq!(tokens("1.5"), None);
    }
    #[test] fn an_empty_expression_has_no_tokens() {
        assert_eq!(tokens(""), Some(vec![]));
    }
}

#[cfg(test)]
mod slice2_eval {
    use super::*;

    #[test] fn a_single_number() {
        assert_eq!(eval("7"), Some(7));
        assert_eq!(eval("  42  "), Some(42));
    }
    #[test] fn addition_and_subtraction() {
        assert_eq!(eval("1+2"), Some(3));
        assert_eq!(eval("10-3"), Some(7));
    }
    /// The usual casualty of fixing precedence.
    #[test] fn subtraction_is_left_associative() {
        assert_eq!(eval("10-3-2"), Some(5));
        assert_eq!(eval("1-2-3-4"), Some(-8));
    }
    #[test] fn multiplication_binds_tighter() {
        assert_eq!(eval("2+3*4"), Some(14));
        assert_eq!(eval("3*4+2"), Some(14));
    }
    #[test] fn division_truncates_toward_zero() {
        assert_eq!(eval("7/2"), Some(3));
        assert_eq!(eval("-7/2"), Some(-3));
    }
    #[test] fn division_is_left_associative() {
        assert_eq!(eval("100/5/2"), Some(10));
    }
    #[test] fn parentheses_override_precedence() {
        assert_eq!(eval("(2+3)*4"), Some(20));
        assert_eq!(eval("2*(3+4)"), Some(14));
    }
    #[test] fn parentheses_nest() {
        assert_eq!(eval("((1+2)*(3+4))"), Some(21));
    }
    /// The usual casualty of making the parser table-driven.
    #[test] fn a_leading_minus_is_a_sign() {
        assert_eq!(eval("-5"), Some(-5));
        assert_eq!(eval("-5+8"), Some(3));
    }
    #[test] fn a_sign_applies_inside_parentheses_and_after_an_operator() {
        assert_eq!(eval("3*-2"), Some(-6));
        assert_eq!(eval("(-2)*3"), Some(-6));
    }
    #[test] fn a_doubled_sign_cancels() {
        assert_eq!(eval("--5"), Some(5));
    }
    #[test] fn division_by_zero_has_no_value() {
        assert_eq!(eval("1/0"), None);
        assert_eq!(eval("1/(2-2)"), None);
    }
    #[test] fn a_malformed_expression_has_no_value() {
        assert_eq!(eval("1+"), None);
        assert_eq!(eval("*2"), None);
        assert_eq!(eval(""), None);
    }
    #[test] fn unbalanced_parentheses_have_no_value() {
        assert_eq!(eval("(1+2"), None);
        assert_eq!(eval("1+2)"), None);
    }
    #[test] fn trailing_rubbish_is_refused() {
        assert_eq!(eval("1 2"), None);
    }
    #[test] fn an_overflow_has_no_value() {
        assert_eq!(eval("9223372036854775807+1"), None);
    }
}
