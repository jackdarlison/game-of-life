use std::collections::HashMap;

use serde::de::{self, Deserializer};
use serde::Deserialize;

use crate::rule_parsing::{parse_conditions, CompareTo, Condition, Op};
/*
{
    "neighbourhood": String

    "states": {
        RULE_NAME: {
            "colour": COLOUR,
            "rules": {
                "RULE_ENCODING": {
                    "next": String,
                    "paint": COLOUR,
                }
            }
        }
    }
}
*/

#[derive(Deserialize, Debug)]
pub(crate) struct Ruleset {
    pub default_state: String,
    pub neighbourhood: Neighbourhood,
    pub states: HashMap<String, RulesetState>,
}

#[derive(Deserialize, Debug)]
pub enum Neighbourhood {
    Individual,
    Moore { range: usize, include_self: bool },
    VonNeuman { range: usize, include_self: bool },
}

impl Neighbourhood {
    pub fn get_neighbours(&self) -> Vec<(isize, isize)> {
        match self {
            Self::Individual => vec![(0, 0)],
            Self::Moore {
                range,
                include_self,
            } => {
                let mut neighbours = vec![];
                for dx in -(*range as isize)..=(*range as isize) {
                    for dy in -(*range as isize)..=(*range as isize) {
                        if !include_self && dx == 0 && dy == 0 {
                            continue; // Skip the cell itself if `include_self` is false
                        }
                        neighbours.push((dx, dy));
                    }
                }
                neighbours
            }
            Self::VonNeuman {
                range,
                include_self,
            } => {
                let mut neighbours = vec![];
                if *include_self {
                    neighbours.push((0, 0));
                }
                for d in 1..=(*range as isize) {
                    for &(dx, dy) in &[(d, 0), (0, d), (-d, 0), (0, -d)] {
                        neighbours.push((dx, dy))
                    }
                }

                neighbours
            }
        }
    }
}

#[derive(Debug)]
pub struct RulesetState {
    pub colour: RulesetColour,
    pub rules: HashMap<String, RulesetOutcome>,
    pub otherwise: Option<RulesetOutcome>,
    parsed_rules: Vec<(Vec<Condition>, RulesetOutcome)>,
}

impl<'de> Deserialize<'de> for RulesetState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RulesetStateHelper {
            colour: RulesetColour,
            rules: HashMap<String, RulesetOutcome>,
            otherwise: Option<RulesetOutcome>,
        }

        let helper = RulesetStateHelper::deserialize(deserializer)?;
        let mut state = RulesetState {
            colour: helper.colour,
            rules: helper.rules,
            otherwise: helper.otherwise,
            parsed_rules: vec![],
        };

        // Run the parse_rules function after deserialization
        state.parse_rules().map_err(de::Error::custom)?;

        Ok(state)
    }
}

impl RulesetState {
    pub fn transition(&self, neighbours: &HashMap<String, usize>) -> Option<&RulesetOutcome> {
        let rules = &self.parsed_rules;
        // println!("{:?}", rules);

        rules
            .iter()
            .find(|(conditions, _)| {
                conditions.iter().all(|c| {
                    let neighbour_value = neighbours.get(&c.state).unwrap_or(&0);

                    let value: &usize = match &c.compare_to {
                        CompareTo::State(name) => neighbours.get(name).unwrap_or(&0),
                        CompareTo::Value(v) => v,
                    };

                    // println!("{} -- {:?} -- {}", neighbour_value, c.op, value);

                    match c.op {
                        Op::Eq => neighbour_value == value,
                        Op::Gt => neighbour_value > value,
                        Op::Ge => neighbour_value >= value,
                        Op::Lt => neighbour_value < value,
                        Op::Le => neighbour_value <= value,
                        Op::Ne => neighbour_value != value,
                    }
                })
            })
            .map(|(_, o)| o)
            .or(self.otherwise.as_ref())
    }

    fn parse_rules(&mut self) -> Result<(), String> {
        let mut rules = vec![];

        for (s, out) in &self.rules {
            let conditions = match parse_conditions(s) {
                Ok((_, c)) => c,
                Err(e) => return Err(format!("{:?}", e)),
            };
            rules.push((conditions, out.clone()))
        }

        self.parsed_rules = rules;

        Ok(())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub(crate) struct RulesetOutcome {
    pub next: String,
    pub paint: Option<RulesetColour>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(untagged)]
pub enum RulesetColour {
    Rgba(u8, u8, u8, u8),
    Hex(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rulesetstate_deserialization() {
        let json_data = json!({
            "colour": [255, 0, 0, 255],
            "rules": {
                "state1 > 2": {
                    "next": "state2",
                    "paint": "#00ffffc2"
                }
            }
        });

        let deserialized: RulesetState =
            serde_json::from_value(json_data).expect("Deserialization failed");

        assert_eq!(deserialized.colour, RulesetColour::Rgba(255, 0, 0, 255));
        assert!(deserialized.rules.contains_key("state1 > 2"));
        assert_eq!(deserialized.rules["state1 > 2"].next, "state2");
        assert_eq!(
            deserialized.rules["state1 > 2"].paint,
            Some(RulesetColour::Hex("#00ffffc2".to_string()))
        );
    }

    #[test]
    fn test_rulesetstate_transition() {
        let mut state = RulesetState {
            colour: RulesetColour::Rgba(255, 0, 0, 255),
            rules: HashMap::new(),
            otherwise: None,
            parsed_rules: vec![],
        };

        state.rules.insert(
            "$state1 > $state2".to_string(),
            RulesetOutcome {
                next: "state2".to_string(),
                paint: Some(RulesetColour::Hex("#00FF0000".to_string())),
            },
        );

        state.parse_rules().expect("Failed to parse rules");

        let neighbours = HashMap::from([("state1".to_string(), 3)]);
        let result = state.transition(&neighbours);

        assert!(result.is_some());
        assert_eq!(result.unwrap().next, "state2");
    }

    #[test]
    fn test_rulesetstate_parse_rules_error() {
        let mut state = RulesetState {
            colour: RulesetColour::Rgba(255, 0, 0, 255),
            rules: HashMap::new(),
            otherwise: None,
            parsed_rules: vec![],
        };

        state.rules.insert(
            "invalid_rule".to_string(),
            RulesetOutcome {
                next: "state2".to_string(),
                paint: Some(RulesetColour::Hex("#00FF00".to_string())),
            },
        );

        let result = state.parse_rules();
        assert!(result.is_err());
    }

    #[test]
    fn test_rulesetcolour_deserialization() {
        let json_data = r##""#00FF00AA""##;
        let colour: RulesetColour =
            serde_json::from_str(json_data).expect("Deserialization failed");

        match colour {
            RulesetColour::Hex(hex) => assert_eq!(hex, "#00FF00AA"),
            _ => panic!("Expected HEX colour"),
        }
    }
}
