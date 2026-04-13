use serde::Serialize;
use std::io::Write;
use anyhow::Result;

#[derive(Serialize)]
pub struct SarifReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Serialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Serialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Serialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
}

#[derive(Serialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
}

#[derive(Serialize)]
pub struct SarifMessage {
    pub text: String,
}

pub fn write_sarif<W: Write>(report: &SarifReport, writer: &mut W) -> Result<()> {
    serde_json::to_writer_pretty(writer, report)?;
    Ok(())
}

pub fn new_report() -> SarifReport {
    SarifReport {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "dart_dec".to_string(),
                    version: "0.1.0".to_string(),
                    rules: vec![
                        SarifRule {
                            id: "DART001".to_string(),
                            name: "HardcodedSecret".to_string(),
                            short_description: SarifMessage { text: "Hardcoded secret found".to_string() },
                        },
                        SarifRule {
                            id: "DART002".to_string(),
                            name: "WeakCrypto".to_string(),
                            short_description: SarifMessage { text: "Weak cryptographic primitive".to_string() },
                        },
                        SarifRule {
                            id: "DART003".to_string(),
                            name: "InsecureUrl".to_string(),
                            short_description: SarifMessage { text: "Insecure HTTP URL found".to_string() },
                        },
                    ],
                },
            },
            results: vec![],
        }],
    }
}
