use crate::{OutputMeta, OutputStats};
use serde::Serialize;
use std::io::Write;
use anyhow::Result;

#[derive(Serialize)]
pub struct JsonOutput {
    pub meta: OutputMeta,
    pub statistics: OutputStats,
    pub libraries: Vec<serde_json::Value>,
    pub strings: Vec<StringEntry>,
    pub security_findings: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct StringEntry {
    pub value: String,
    pub refs_count: usize,
}

pub fn write_json<W: Write>(output: &JsonOutput, writer: &mut W) -> Result<()> {
    serde_json::to_writer_pretty(writer, output)?;
    Ok(())
}

pub fn write_jsonl<W: Write>(output: &JsonOutput, writer: &mut W) -> Result<()> {
    serde_json::to_writer(&mut *writer, &output.meta)?;
    writeln!(writer)?;
    serde_json::to_writer(&mut *writer, &output.statistics)?;
    writeln!(writer)?;
    for lib in &output.libraries {
        serde_json::to_writer(&mut *writer, lib)?;
        writeln!(writer)?;
    }
    Ok(())
}
