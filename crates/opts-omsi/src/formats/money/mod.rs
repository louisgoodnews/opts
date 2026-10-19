//! Parser and data model for OMSI money system configuration files (`Money/<pack>/*.cfg`).
//!
//! A money system defines the currency, coin models, and banknote models
//! used on a map for passenger fares and driver change transactions.
//!
//! See specification in `docs/src/formats/text/money.md`.

pub mod error;
pub use error::MoneyParseError;

use crate::formats::text::{decode_bytes, parse_f64, scan_blocks};

/// Description of the currency unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Currency {
    /// Currency name label (e.g. `"Euro"`, `"DM"`, `"Hamburg109_Euro"`).
    pub name: String,
    /// Number of decimal places (typically 2).
    pub decimals: u32,
}

/// A physical denomination (coin or banknote) in the money system.
#[derive(Debug, Clone, PartialEq)]
pub struct Denomination {
    /// Relative path to the `.o3d` mesh file representing this coin or bill.
    pub mesh: String,
    /// Face value in main currency units (e.g. `0.05`, `1.0`, `5.0`).
    pub value: f64,
}

/// A parsed money system specification.
#[derive(Debug, Clone, PartialEq)]
pub struct MoneySystem {
    /// Currency declaration.
    pub currency: Currency,
    /// List of coin denominations.
    pub coins: Vec<Denomination>,
    /// List of banknote denominations.
    pub bills: Vec<Denomination>,
}

impl MoneySystem {
    /// Parses a money configuration file from text.
    ///
    /// # Errors
    /// Returns [`MoneyParseError`] if required blocks or parameters are missing or invalid.
    pub fn parse(text: &str) -> Result<Self, MoneyParseError> {
        let blocks = scan_blocks(text);

        let mut currency: Option<Currency> = None;
        let mut coins = Vec::new();
        let mut bills = Vec::new();

        for block in blocks {
            match block.keyword.as_str() {
                "currency" => {
                    if block.parameters.len() < 2 {
                        return Err(MoneyParseError::MissingParameters {
                            keyword: "currency".to_string(),
                            line: block.line_number,
                            expected: 2,
                            found: block.parameters.len(),
                        });
                    }
                    let name = block.parameters[0].clone();
                    let decimals = block.parameters[1].parse::<u32>().map_err(|_| {
                        MoneyParseError::InvalidNumber {
                            line: block.line_number,
                            value: block.parameters[1].clone(),
                        }
                    })?;
                    currency = Some(Currency { name, decimals });
                }
                "coin" => {
                    if block.parameters.len() < 2 {
                        return Err(MoneyParseError::MissingParameters {
                            keyword: "coin".to_string(),
                            line: block.line_number,
                            expected: 2,
                            found: block.parameters.len(),
                        });
                    }
                    let mesh = block.parameters[0].clone();
                    let value = parse_f64(&block.parameters[1]).map_err(|_| {
                        MoneyParseError::InvalidNumber {
                            line: block.line_number,
                            value: block.parameters[1].clone(),
                        }
                    })?;
                    coins.push(Denomination { mesh, value });
                }
                "bill" => {
                    if block.parameters.len() < 2 {
                        return Err(MoneyParseError::MissingParameters {
                            keyword: "bill".to_string(),
                            line: block.line_number,
                            expected: 2,
                            found: block.parameters.len(),
                        });
                    }
                    let mesh = block.parameters[0].clone();
                    let value = parse_f64(&block.parameters[1]).map_err(|_| {
                        MoneyParseError::InvalidNumber {
                            line: block.line_number,
                            value: block.parameters[1].clone(),
                        }
                    })?;
                    bills.push(Denomination { mesh, value });
                }
                _ => {
                    // Ignore unrecognized or unneeded blocks
                }
            }
        }

        let currency = currency.ok_or(MoneyParseError::MissingCurrency)?;

        Ok(Self {
            currency,
            coins,
            bills,
        })
    }

    /// Parses a money configuration file from raw bytes.
    ///
    /// # Errors
    /// Returns [`MoneyParseError`] on syntax or decoding failure.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, MoneyParseError> {
        let text = decode_bytes(bytes);
        Self::parse(&text)
    }
}

/// Convenience function to parse money configuration from a string slice.
pub fn parse_money(text: &str) -> Result<MoneySystem, MoneyParseError> {
    MoneySystem::parse(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dm_cfg() {
        let sample = r#"
[currency]
DM
2

[coin]
DM_muenzen_5Pf.o3d
0.05

[coin]
DM_muenzen_10Pf.o3d
0.1

[coin]
DM_muenzen_1DM.o3d
1

[bill]
DM_schein_5DM.o3d
5
"#;
        let system = parse_money(sample).unwrap();
        assert_eq!(system.currency.name, "DM");
        assert_eq!(system.currency.decimals, 2);
        assert_eq!(system.coins.len(), 3);
        assert_eq!(system.coins[0].mesh, "DM_muenzen_5Pf.o3d");
        assert_eq!(system.coins[0].value, 0.05);
        assert_eq!(system.coins[1].value, 0.1);
        assert_eq!(system.coins[2].value, 1.0);
        assert_eq!(system.bills.len(), 1);
        assert_eq!(system.bills[0].mesh, "DM_schein_5DM.o3d");
        assert_eq!(system.bills[0].value, 5.0);
    }

    #[test]
    fn test_missing_currency_fails() {
        let sample = r#"
[coin]
coin.o3d
1.0
"#;
        assert_eq!(parse_money(sample), Err(MoneyParseError::MissingCurrency));
    }

    #[test]
    fn test_insufficient_parameters_fails() {
        let sample = r#"
[currency]
Euro

[coin]
coin.o3d
"#;
        match parse_money(sample) {
            Err(MoneyParseError::MissingParameters { keyword, .. }) => {
                assert_eq!(keyword, "currency");
            }
            res => panic!("Expected MissingParameters, got: {:?}", res),
        }
    }
}
