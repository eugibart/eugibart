//! Safety interlocks.
//!
//! Policy (see docs/SAFETY.md):
//! - Sessions start read-only; anything that changes ECU state requires
//!   service mode plus a per-operation confirmation from the user.
//! - A routine's preconditions (from the ECU definition file) are evaluated
//!   against live measurements *before* any bytes are sent.

use motodiag_ecu_defs::schema::Preconditions;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SafetyViolation {
    #[error("service mode is not enabled; this session is read-only")]
    ServiceModeDisabled,
    #[error("operation requires explicit confirmation")]
    ConfirmationRequired,
    #[error("engine must be off (measured {rpm:.0} rpm)")]
    EngineRunning { rpm: f64 },
    #[error("engine state unknown: no RPM channel available to verify engine is off")]
    EngineStateUnknown,
    #[error("battery voltage {measured:.1} V outside allowed range {min:.1}–{max:.1} V")]
    BatteryOutOfRange { measured: f64, min: f64, max: f64 },
    #[error("battery voltage unknown: no battery channel available to verify")]
    BatteryUnknown,
}

/// Evaluate routine preconditions against current measurements.
///
/// `rpm` / `battery_v` are `None` when the ECU definition has no channel to
/// measure them — which *fails closed*: if a precondition needs a measurement
/// we don't have, the routine is blocked.
pub fn check_preconditions(
    pre: &Preconditions,
    rpm: Option<f64>,
    battery_v: Option<f64>,
) -> Result<(), SafetyViolation> {
    if pre.engine_must_be_off {
        match rpm {
            Some(rpm) if rpm > 0.0 => return Err(SafetyViolation::EngineRunning { rpm }),
            Some(_) => {}
            None => return Err(SafetyViolation::EngineStateUnknown),
        }
    }

    if pre.min_battery_v.is_some() || pre.max_battery_v.is_some() {
        let measured = battery_v.ok_or(SafetyViolation::BatteryUnknown)?;
        let min = pre.min_battery_v.unwrap_or(f64::NEG_INFINITY);
        let max = pre.max_battery_v.unwrap_or(f64::INFINITY);
        if measured < min || measured > max {
            return Err(SafetyViolation::BatteryOutOfRange { measured, min, max });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pre(engine_off: bool, min_v: Option<f64>, max_v: Option<f64>) -> Preconditions {
        Preconditions {
            engine_must_be_off: engine_off,
            min_battery_v: min_v,
            max_battery_v: max_v,
            notes: None,
        }
    }

    #[test]
    fn engine_running_blocks() {
        let err = check_preconditions(&pre(true, None, None), Some(1250.0), None).unwrap_err();
        assert!(matches!(err, SafetyViolation::EngineRunning { .. }));
    }

    #[test]
    fn missing_measurement_fails_closed() {
        assert_eq!(
            check_preconditions(&pre(true, None, None), None, None),
            Err(SafetyViolation::EngineStateUnknown)
        );
        assert_eq!(
            check_preconditions(&pre(false, Some(12.0), None), None, None),
            Err(SafetyViolation::BatteryUnknown)
        );
    }

    #[test]
    fn battery_window_enforced() {
        let p = pre(false, Some(12.0), Some(14.8));
        assert!(check_preconditions(&p, None, Some(12.8)).is_ok());
        assert!(matches!(
            check_preconditions(&p, None, Some(11.4)),
            Err(SafetyViolation::BatteryOutOfRange { .. })
        ));
        assert!(matches!(
            check_preconditions(&p, None, Some(15.2)),
            Err(SafetyViolation::BatteryOutOfRange { .. })
        ));
    }

    #[test]
    fn no_preconditions_passes_with_no_data() {
        assert!(check_preconditions(&pre(false, None, None), None, None).is_ok());
    }
}
