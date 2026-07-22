/** TL-PX Minimum Standard identity constants */

export const STANDARD_ID = "TL-PX";
export const STANDARD_VERSION = "0.1.0";
export const CONTROL_MODE = "ALLOW_OR_ESCALATE";

/** Canonical record types + accepted aliases (Glass product names). */
export const RECORD_TYPES = {
  decision: ["tlpx.decision", "glass.decision"],
  operator_action: ["tlpx.operator_action", "glass.operator_action"],
  execution: ["tlpx.execution", "glass.execution"],
  accountability_report: [
    "tlpx.accountability_report",
    "glass.accountability_report"
  ],
  incident: ["tlpx.incident", "glass.incident"]
};

export function isRecordType(value, kind) {
  return RECORD_TYPES[kind]?.includes(value);
}

export function standardStamp() {
  return {
    standard: STANDARD_ID,
    standard_version: STANDARD_VERSION
  };
}
