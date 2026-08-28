//! Slice 3.9 restricted-agent local PEP and acceptance client.

use std::path::Path;
use tlpx::{
    assert_replacement_bind_denied, raw_restricted_pep_request, request_restricted_pep,
    serve_restricted_pep, verify_restricted_pep_evidence,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> tlpx::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("serve") => {
            let config = required(&mut args, "CONFIG")?;
            end(&mut args)?;
            serve_restricted_pep(config)
        }
        Some("request") => {
            let socket = required(&mut args, "SOCKET")?;
            let request_id = required(&mut args, "REQUEST_ID")?;
            let operation = required(&mut args, "OPERATION")?;
            end(&mut args)?;
            println!(
                "{}",
                request_restricted_pep(socket, &request_id, &operation)?
            );
            Ok(())
        }
        Some("raw") => {
            let socket = required(&mut args, "SOCKET")?;
            let request = required(&mut args, "REQUEST_LINE")?;
            end(&mut args)?;
            println!("{}", raw_restricted_pep_request(socket, &request)?);
            Ok(())
        }
        Some("verify") => {
            let config = required(&mut args, "CONFIG")?;
            end(&mut args)?;
            let (reconciliation, evidence) = verify_restricted_pep_evidence(config)?;
            eprintln!(
                "verified total={} pending={} exported={} last_chain_hash={}",
                reconciliation.total,
                reconciliation.pending,
                reconciliation.exported,
                reconciliation.last_chain_hash.as_deref().unwrap_or("none")
            );
            for row in evidence {
                println!("{}", row.record_json);
            }
            Ok(())
        }
        Some("probe-bind") => {
            let socket = required(&mut args, "SOCKET")?;
            end(&mut args)?;
            assert_replacement_bind_denied(Path::new(&socket))?;
            println!("DENY PEP_REPLACEMENT_BIND");
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn required(args: &mut impl Iterator<Item = String>, _name: &str) -> tlpx::Result<String> {
    args.next().ok_or_else(usage)
}

fn end(args: &mut impl Iterator<Item = String>) -> tlpx::Result<()> {
    if args.next().is_some() {
        Err(usage())
    } else {
        Ok(())
    }
}

fn usage() -> tlpx::Error {
    tlpx::Error::coded(
        "PEP_REQUEST_INVALID",
        "usage: tlpx-run serve CONFIG | request SOCKET REQUEST_ID OPERATION | raw SOCKET REQUEST_LINE | verify CONFIG | probe-bind SOCKET",
    )
}
