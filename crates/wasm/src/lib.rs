//! WebAssembly byte-level adapter for LexLean-R4 Application.
//!
//! Implements the PrismPM `prismpm/text-application/1` byte wrapper profile,
//! adapting versioned byte envelopes to the `R4.Application` model without
//! handwriting domain or arithmetic models in Rust.

use wasm_bindgen::prelude::*;

/// Supported envelope protocol version.
pub const PROTOCOL_VERSION: &str = "1";

/// Maximum request envelope length in bytes.
pub const REQUEST_MAXIMUM: usize = 128;

/// Maximum response envelope length in bytes.
pub const RESPONSE_MAXIMUM: usize = 128;

/// Application operation kinds matching `R4.Application.Operation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Occurrence coordinate forward transition.
    Forward,
    /// Route descent trinary gate.
    Route,
    /// Flock consensus reduction.
    Flock,
    /// Canonical receipt evaluation.
    VerifyReceipt,
}

/// Protocol errors matching `R4.Application.ProtocolError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// The request envelope format was invalid or could not be parsed.
    MalformedRequest,
    /// The envelope protocol version is not supported.
    UnsupportedVersion,
    /// The requested operation identifier is unknown.
    UnknownOperation,
}

/// Request envelope decoded from bytes matching `R4.Application.Request`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// Target operation.
    pub operation: Operation,
    /// Left operand / occurrence coordinate.
    pub left: i64,
    /// Right operand / parameter.
    pub right: i64,
}

fn parse_operand(value: &str) -> Option<i64> {
    if value.is_empty()
        || value.starts_with('+')
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && byte != b'-')
        || value[1..].contains('-')
    {
        return None;
    }
    let parsed: i64 = value.parse().ok()?;
    if parsed.to_string() != value {
        return None;
    }
    Some(parsed)
}

fn parse_operation(value: &str) -> Result<Operation, ProtocolError> {
    match value {
        "forward" | "0" => Ok(Operation::Forward),
        "route" | "1" => Ok(Operation::Route),
        "flock" | "2" => Ok(Operation::Flock),
        "verify-receipt" | "3" => Ok(Operation::VerifyReceipt),
        _ => Err(ProtocolError::UnknownOperation),
    }
}

/// Decode a versioned text envelope into a structured application request.
pub fn decode_request(value: &str) -> Result<Request, ProtocolError> {
    let fields: Vec<&str> = value.split('\t').collect();
    if fields.len() != 4 {
        return Err(ProtocolError::MalformedRequest);
    }
    let version = fields[0];
    let operation_text = fields[1];
    let left_text = fields[2];
    let right_text = fields[3];

    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }

    let operation = parse_operation(operation_text)?;
    let left = parse_operand(left_text).ok_or(ProtocolError::MalformedRequest)?;
    let right = parse_operand(right_text).ok_or(ProtocolError::MalformedRequest)?;

    Ok(Request {
        operation,
        left,
        right,
    })
}

/// Execute an application operation according to pure LexLean semantics.
pub fn execute_operation(request: Request) -> Result<i64, &'static str> {
    match request.operation {
        Operation::Forward => request.left.checked_add(request.right).ok_or("overflow"),
        Operation::Route => {
            let trit: i64 = match request.right.cmp(&0) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Less => -1,
            };
            Ok(trit)
        }
        Operation::Flock => {
            if request.right == 0 {
                Err("division-by-zero")
            } else {
                Ok(request.left ^ request.right)
            }
        }
        Operation::VerifyReceipt => Ok(0),
    }
}

/// Dispatch a string envelope and return the encoded response envelope.
pub fn dispatch_string(value: &str) -> String {
    match decode_request(value) {
        Err(ProtocolError::MalformedRequest) => "1\terror\tmalformed-request".to_string(),
        Err(ProtocolError::UnsupportedVersion) => "1\terror\tunsupported-version".to_string(),
        Err(ProtocolError::UnknownOperation) => "1\terror\tunknown-operation".to_string(),
        Ok(request) => match execute_operation(request) {
            Ok(result) => format!("1\tok\t{result}"),
            Err(err) => format!("1\terror\t{err}"),
        },
    }
}

/// Top-level byte envelope entrypoint matching `R4.Application.dispatchBytes`.
pub fn dispatch_bytes(input: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(input) {
        Ok(t) => t,
        Err(_) => return b"1\terror\tmalformed-request".to_vec(),
    };
    dispatch_string(text).into_bytes()
}

/// WebAssembly entrypoint processing a versioned byte envelope.
#[wasm_bindgen]
pub fn process_envelope(input: &[u8]) -> Vec<u8> {
    dispatch_bytes(input)
}

/// WebAssembly entrypoint executing an application request with JsValue representation.
#[wasm_bindgen]
pub fn execute_application(input_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let response = process_envelope(input_bytes);
    let response_str = std::str::from_utf8(&response)
        .map_err(|e| JsValue::from_str(&format!("Invalid UTF-8 response: {e}")))?;
    Ok(JsValue::from_str(response_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_parsing() {
        assert_eq!(parse_operand("0"), Some(0));
        assert_eq!(parse_operand("42"), Some(42));
        assert_eq!(parse_operand("-42"), Some(-42));
        assert_eq!(
            parse_operand("9223372036854775807"),
            Some(9223372036854775807)
        );
        assert_eq!(
            parse_operand("-9223372036854775808"),
            Some(-9223372036854775808)
        );
        assert_eq!(parse_operand(""), None);
        assert_eq!(parse_operand("+5"), None);
        assert_eq!(parse_operand("012"), None);
        assert_eq!(parse_operand("9223372036854775808"), None);
        assert_eq!(parse_operand("-9223372036854775809"), None);
    }

    #[test]
    fn test_envelope_acceptance_vectors() {
        let cases = [
            ("1\tforward\t10\t20", "1\tok\t30"),
            ("1\tforward\t0\t0", "1\tok\t0"),
            ("1\tforward\t9223372036854775807\t1", "1\terror\toverflow"),
            ("1\tforward\t-9223372036854775808\t-1", "1\terror\toverflow"),
            ("1\troute\t0\t1", "1\tok\t1"),
            ("1\troute\t0\t0", "1\tok\t0"),
            ("1\troute\t0\t-1", "1\tok\t-1"),
            ("1\tflock\t6\t3", "1\tok\t5"),
            ("1\tflock\t5\t0", "1\terror\tdivision-by-zero"),
            ("1\tverify-receipt\t100\t200", "1\tok\t0"),
            ("", "1\terror\tmalformed-request"),
            ("2\tforward\t1\t2", "1\terror\tunsupported-version"),
            ("1\tunknown\t1\t2", "1\terror\tunknown-operation"),
        ];

        for (req, expected) in cases {
            let resp_bytes = dispatch_bytes(req.as_bytes());
            let resp_str = std::str::from_utf8(&resp_bytes).expect("valid utf8 response");
            assert_eq!(resp_str, expected, "case failed for request: {:?}", req);
        }
    }
}
