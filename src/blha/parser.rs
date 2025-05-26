#![allow(unused)]

use super::{AmplitudeType, Contract, Subprocess, error::BLHAError};
use indexmap::IndexMap;
use peg::parser;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone)]
enum Value<'a> {
    Int(i64),
    Float(f64),
    String(&'a str),
    List(Vec<Value<'a>>),
}

impl<'a> Value<'a> {
    fn int(self) -> Result<i64, &'static str> {
        match self {
            Self::Int(i) => Ok(i),
            _ => Err("Int"),
        }
    }

    fn float(self) -> Result<f64, &'static str> {
        match self {
            Self::Float(x) => Ok(x),
            _ => Err("Int"),
        }
    }

    fn str(self) -> Result<&'a str, &'static str> {
        match self {
            Self::String(s) => Ok(s),
            _ => Err("String"),
        }
    }

    fn to_string(self) -> String {
        match self {
            Self::Int(i) => i.to_string(),
            Self::Float(x) => x.to_string(),
            Self::String(s) => s.to_owned(),
            Self::List(l) => format!("{l:?}"),
        }
    }
}

enum Statement<'a> {
    Option(&'a str, Value<'a>),
    SubProcess(i64, i64, Vec<i64>, Vec<i64>),
}

peg::parser!(
    grammar blha_contract() for str {
    rule whitespace() = quiet!{[' ' | '\t' | '\n' | '\r']}
        rule comment() = quiet!{"#" [^'\n']* "\n"}
        rule _() = quiet!{(comment() / whitespace())*}
        rule alphanumeric() = quiet!{['a'..='z' | 'A'..='Z' | '0'..='9']}
        rule ident_char() = quiet!{alphanumeric() / ['_' | '~']}

        rule str() -> Value<'input> = quiet!{s:$([^' ' | '\t' | '\n' | '\r']+) {Value::String(s)}} / expected!("String")
        rule int() -> Value<'input> = quiet!{int:$(['+' | '-']? ['0'..='9']+) {?
            match int.parse() {
                Ok(i) => Ok(Value::Int(i)),
                Err(_) => Err("int")
            }}
        } / expected!("Int")
        rule float() -> Value<'input> = float:$(['+' | '-']? ['0'..='9']+ "." ['0'..='9']*) {?
            match float.parse() {
                Ok(x) => Ok(Value::Float(x)),
                Err(_) => Err("float")
            }
        }

        rule value() -> Value<'input> = float() / int() / str()

        rule option() -> Result<Statement<'input>, BLHAError> =
            option:str() _ values:((!"|" v:value() {v}) **<1,> _) _ "|" _ answer:$([^'\n']+) {?
                if answer.to_lowercase().as_str() == "ok" {
                    return Ok(Ok(
                        Statement::Option(option.str()?, if values.len() == 1 {values[0].clone()} else {Value::List(values)})
                    ));
                } else {
                    return Ok(Err(BLHAError::ContractError(answer.to_owned())));
                }
            }

        rule subprocess() -> Statement<'input> =
            inc:(int() **<1,> _) _ "->" _ out:(int() **<1,> _) _ "|" _ n:int() _ id:int() _ {?
                return Ok(
                    Statement::SubProcess(
                        id.int()?,
                        n.int()?,
                        inc.into_iter().map(|i| i.int()).collect::<Result<_, _>>()?,
                        out.into_iter().map(|i| i.int()).collect::<Result<_, _>>()?,
                ));
            }

        pub rule contract() -> Result<Contract, BLHAError> =
            _ statements:( ( (s:subprocess() {Ok(s)}) / (o:option() {o}) ) ** _ ) _ {?
                let mut subprocesses = Vec::new();
                let mut options = IndexMap::new();
                let mut atype = AmplitudeType::Loop;
                for statement in statements {
                    match statement {
                        Ok(Statement::Option(option, value)) => {
                            match option.to_lowercase().as_str() {
                                "amplitudetype" => {
                                    atype = match value.clone().str()?.to_lowercase().as_str() {
                                        "tree" => AmplitudeType::Tree,
                                        "loop" => AmplitudeType::Loop,
                                        "cctree" => AmplitudeType::ccTree,
                                        "sctree" => AmplitudeType::scTree,
                                        "sctree2" => AmplitudeType::scTree2,
                                        "loopinduced" => AmplitudeType::LoopInduced,
                                        _ => {
                                            return Ok(Err(BLHAError::ContractError(
                                                format!("Unknown amplitude type: {}", value.to_string())
                                            )))
                                        }
                                    };
                                    options.insert(option.to_owned(), value.to_string());
                                }
                                _ => { options.insert(option.to_owned(), value.to_string()); }
                            }
                        },
                        Ok(Statement::SubProcess(id, n_hel, incoming, outgoing)) => {
                            subprocesses.push(
                                Subprocess {
                                    id,
                                    amplitude_type: atype,
                                    incoming_pdg: incoming,
                                    outgoing_pdg: outgoing
                                }
                            )
                        },
                        Err(e) => {return Ok(Err(e));}
                    }
                }
                subprocesses.sort_unstable_by_key(|s| s.id);
                return Ok(Ok(Contract {
                    options,
                    subprocesses
                }));
            }

    }
);

pub(crate) fn parse_contract(path: &Path) -> Result<Contract, BLHAError> {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return Err(BLHAError::IOError(path.to_str().unwrap().to_owned(), e));
        }
    };
    return match blha_contract::contract(&content) {
        Ok(c) => c,
        Err(e) => Err(BLHAError::ParseError(
            path.file_name().unwrap().to_str().unwrap().to_owned(),
            e,
        )),
    };
}

#[cfg(test)]
mod tests {
    use crate::blha::AmplitudeType::*;
    use crate::blha::*;

    #[test]
    fn contract_parser_test() {
        let content = r#"# OLE_order.lh
# Created by Sherpa-2.0.0
InterfaceVersion BLHA2 | OK
Model SMdiag | OK
MatrixElementSquareType CHsummed | OK
CorrectionType QCD | OK
IRregularisation DRED | OK
WidthScheme ComplexMass | OK
EWScheme alphaGF | OK
AccuracyTarget 0.0001 | OK
DebugUnstable True | OK
Extra Line1 | OK
Extra Line2 | OK
# process list
CouplingPower QCD 2 | OK
CouplingPower QED 0 | OK
1 -1 -> 6 -6 | 1 4
-1 1 -> 6 -6 | 1 2
21 21 -> 6 -6 | 1 5
CouplingPower QCD 3 | OK
CouplingPower QED 0 | OK
1 -1 -> 6 -6 21 | 1 1
1 21 -> 6 -6 1 | 1 3
-1 1 -> 6 -6 21 | 1 8
-1 21 -> 6 -6 -1 | 1 9
21 1 -> 6 -6 1 | 1 10
21 -1 -> 6 -6 -1 | 1 6
21 21 -> 6 -6 21 | 1 7"#;
        let contract = super::blha_contract::contract(&content).unwrap().unwrap();
        let contract_ref = Contract {
            options: IndexMap::from([
                ("InterfaceVersion".to_owned(), "BLHA2".to_owned()),
                ("Model".to_owned(), "SMdiag".to_owned()),
                ("MatrixElementSquareType".to_owned(), "CHsummed".to_owned()),
                ("CorrectionType".to_owned(), "QCD".to_owned()),
                ("IRregularisation".to_owned(), "DRED".to_owned()),
                ("WidthScheme".to_owned(), "ComplexMass".to_owned()),
                ("EWScheme".to_owned(), "alphaGF".to_owned()),
                ("AccuracyTarget".to_owned(), "0.0001".to_owned()),
                ("DebugUnstable".to_owned(), "True".to_owned()),
                ("Extra".to_owned(), "Line2".to_owned()),
                (
                    "CouplingPower".to_owned(),
                    "[String(\"QED\"), Int(0)]".to_owned(),
                ),
            ]),
            subprocesses: vec![
                Subprocess {
                    id: 1,
                    amplitude_type: Loop,
                    incoming_pdg: vec![1, -1],
                    outgoing_pdg: vec![6, -6, 21],
                },
                Subprocess {
                    id: 2,
                    amplitude_type: Loop,
                    incoming_pdg: vec![-1, 1],
                    outgoing_pdg: vec![6, -6],
                },
                Subprocess {
                    id: 3,
                    amplitude_type: Loop,
                    incoming_pdg: vec![1, 21],
                    outgoing_pdg: vec![6, -6, 1],
                },
                Subprocess {
                    id: 4,
                    amplitude_type: Loop,
                    incoming_pdg: vec![1, -1],
                    outgoing_pdg: vec![6, -6],
                },
                Subprocess {
                    id: 5,
                    amplitude_type: Loop,
                    incoming_pdg: vec![21, 21],
                    outgoing_pdg: vec![6, -6],
                },
                Subprocess {
                    id: 6,
                    amplitude_type: Loop,
                    incoming_pdg: vec![21, -1],
                    outgoing_pdg: vec![6, -6, -1],
                },
                Subprocess {
                    id: 7,
                    amplitude_type: Loop,
                    incoming_pdg: vec![21, 21],
                    outgoing_pdg: vec![6, -6, 21],
                },
                Subprocess {
                    id: 8,
                    amplitude_type: Loop,
                    incoming_pdg: vec![-1, 1],
                    outgoing_pdg: vec![6, -6, 21],
                },
                Subprocess {
                    id: 9,
                    amplitude_type: Loop,
                    incoming_pdg: vec![-1, 21],
                    outgoing_pdg: vec![6, -6, -1],
                },
                Subprocess {
                    id: 10,
                    amplitude_type: Loop,
                    incoming_pdg: vec![21, 1],
                    outgoing_pdg: vec![6, -6, 1],
                },
            ],
        };
        assert_eq!(contract, contract_ref);
    }
}
