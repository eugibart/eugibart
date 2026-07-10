//! Expose the simulated ECU on a pseudo-terminal so any serial client —
//! including the MotoDiag desktop app pointed at the printed device path —
//! can talk to it like real hardware.
//!
//! Limitations vs. the in-memory link: break pulses don't propagate through a
//! pty, so fast-init wake-ups are invisible here. The simulator still accepts
//! StartCommunication directly, which is what matters for protocol testing.

#[cfg(unix)]
fn main() {
    use std::io::{Read, Write};
    use std::time::Duration;

    use motodiag_ecu_sim::{SimConfig, Simulator};
    use motodiag_kwp2000::framing::{parse_buffer, FrameCodec, ParseStatus};
    use serialport::SerialPort;

    let (mut master, slave) = serialport::TTYPort::pair().expect("failed to create pty pair");
    master
        .set_timeout(Duration::from_millis(100))
        .expect("failed to set pty timeout");

    let config = SimConfig::default();
    let response_codec = FrameCodec::physical(config.tester_address, config.ecu_address);
    let mut sim = Simulator::new(config);

    println!(
        "Simulated ECU listening on: {}",
        slave.name().unwrap_or_else(|| "<unknown pty>".to_string())
    );
    println!("Point the MotoDiag app (or any KWP2000 tester) at that port. Ctrl-C to stop.");

    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 64];
    loop {
        match master.read(&mut chunk) {
            Ok(0) => {}
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                loop {
                    match parse_buffer(&buf) {
                        ParseStatus::Incomplete => break,
                        ParseStatus::Invalid(reason) => {
                            eprintln!("resync: {reason}");
                            buf.clear();
                            break;
                        }
                        ParseStatus::Complete { frame, consumed } => {
                            buf.drain(..consumed);
                            if let Some(response) = sim.handle_request(&frame.payload) {
                                let encoded = response_codec.encode(&response);
                                if master.write_all(&encoded).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                eprintln!("pty error: {e}");
                return;
            }
        }
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("The pty simulator is only available on Unix platforms.");
    std::process::exit(1);
}
