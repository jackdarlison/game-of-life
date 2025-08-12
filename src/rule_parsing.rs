/*
Rules are define as boolean logic

CONDITIONS := CONDITIONS ; CONDITION || CONDITION
CONDITION := STATE_NAME OP COMPARE_TO
OP := '==' || '>' || '<' || '>=' || '<=' || '!='
COMPARE_TO := STATE_NAME || numeric
STATE_NAME := $ alpha_numeric+
*/

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, digit1, multispace0},
    combinator::{map, map_res},
    multi::separated_list1,
    sequence::{delimited, preceded},
    IResult, Parser,
};

#[derive(PartialEq, Debug)]
pub(crate) struct Condition {
    pub state: String,
    pub op: Op,
    pub compare_to: CompareTo,
}

#[derive(PartialEq, Debug)]
pub(crate) enum Op {
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
    Ne,
}

#[derive(PartialEq, Debug)]
pub(crate) enum CompareTo {
    State(String),
    Value(usize),
}

pub fn parse_conditions(input: &str) -> IResult<&str, Vec<Condition>> {
    separated_list1(
        delimited(multispace0, char(';'), multispace0),
        parse_condition,
    )
    .parse(input)
}

fn parse_condition(input: &str) -> IResult<&str, Condition> {
    map(
        (
            parse_state_name,
            delimited(multispace0, parse_operator, multispace0),
            parse_compare_to,
        ),
        |(state, op, compare_to)| Condition {
            state,
            op,
            compare_to,
        },
    )
    .parse(input)
}

fn parse_state_name(input: &str) -> IResult<&str, String> {
    map(
        preceded(
            char('$'),
            take_while1(|c: char| c.is_alphanumeric() || c == '_'),
        ),
        String::from,
    )
    .parse(input)
}

fn parse_operator(input: &str) -> IResult<&str, Op> {
    alt((
        map(tag("=="), |_| Op::Eq),
        map(tag(">="), |_| Op::Ge),
        map(tag("<="), |_| Op::Le),
        map(tag("!="), |_| Op::Ne),
        map(tag(">"), |_| Op::Gt),
        map(tag("<"), |_| Op::Lt),
    ))
    .parse(input)
}

fn parse_compare_to(input: &str) -> IResult<&str, CompareTo> {
    alt((
        map(parse_state_name, CompareTo::State),
        map_res(digit1, |s: &str| s.parse::<usize>().map(CompareTo::Value)),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_operator() {
        assert_eq!(parse_operator("==").unwrap().1, Op::Eq);
        assert_eq!(parse_operator(">=").unwrap().1, Op::Ge);
        assert_eq!(parse_operator("<=").unwrap().1, Op::Le);
        assert_eq!(parse_operator("!=").unwrap().1, Op::Ne);
        assert_eq!(parse_operator(">").unwrap().1, Op::Gt);
        assert_eq!(parse_operator("<").unwrap().1, Op::Lt);
    }

    #[test]
    fn test_parse_compare_to_state() {
        let input = "$state_name";
        let result = parse_compare_to(input).unwrap();
        assert_eq!(result.1, CompareTo::State("state_name".to_string()));
    }

    #[test]
    fn test_parse_compare_to_value() {
        let input = "123";
        let result = parse_compare_to(input).unwrap();
        assert_eq!(result.1, CompareTo::Value(123));
    }

    #[test]
    fn test_parse_state_name() {
        let input = "$state1";
        let result = parse_state_name(input).unwrap();
        assert_eq!(result.1, "state1".to_string());
    }

    #[test]
    fn test_parse_condition() {
        let input = "$state1 == 123";
        let result = parse_condition(input).unwrap();
        assert_eq!(result.1.state, "state1".to_string());
        assert_eq!(result.1.op, Op::Eq);
        assert_eq!(result.1.compare_to, CompareTo::Value(123));
    }

    #[test]
    fn test_parse_conditions() {
        let input = "$state1 == 123; $state2 != $state3";
        let result = parse_conditions(input).unwrap();
        let conditions = result.1;

        assert_eq!(conditions.len(), 2);

        assert_eq!(conditions[0].state, "state1".to_string());
        assert_eq!(conditions[0].op, Op::Eq);
        assert_eq!(conditions[0].compare_to, CompareTo::Value(123));

        assert_eq!(conditions[1].state, "state2".to_string());
        assert_eq!(conditions[1].op, Op::Ne);
        assert_eq!(
            conditions[1].compare_to,
            CompareTo::State("state3".to_string())
        );
    }
}
