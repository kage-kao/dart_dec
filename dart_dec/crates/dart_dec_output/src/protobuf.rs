use std::io::Write;
use anyhow::Result;

/// Protobuf output stub (for gRPC integration)
pub fn write_protobuf_stub<W: Write>(_writer: &mut W) -> Result<()> {
    Ok(())
}
