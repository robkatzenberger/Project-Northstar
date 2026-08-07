package ops

import (
	"fmt"
	"time"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/audit"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/gate"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/switchboard"
)

// EvaluateAndAppend runs gate + optional switchboard and seals decision to audit.
func EvaluateAndAppend(intent policy.Intent, p *policy.Policy, sb *switchboard.Config, auditPath string) (gate.Decision, error) {
	d, err := gate.EvaluateIntent(intent, p, gate.EvaluateOptions{Switchboard: sb})
	if err != nil {
		return nil, err
	}
	sealed, err := audit.Append(auditPath, map[string]any(d))
	if err != nil {
		return nil, err
	}
	return gate.Decision(sealed), nil
}

func routeList(decision map[string]any) []string {
	switch rr := decision["approval_route"].(type) {
	case []string:
		return rr
	case []any:
		var out []string
		for _, x := range rr {
			if s, ok := x.(string); ok {
				out = append(out, s)
			}
		}
		return out
	default:
		return nil
	}
}

func AssertOperator(operatorID string, decision map[string]any, sb *switchboard.Config) error {
	if operatorID == "" {
		return fmt.Errorf("operator_id required")
	}
	route := routeList(decision)
	if len(route) > 0 {
		found := false
		for _, s := range route {
			if s == operatorID {
				found = true
				break
			}
		}
		if !found {
			return fmt.Errorf("operator '%s' not on approval_route", operatorID)
		}
	}
	if sb != nil && sb.Operators != nil && len(sb.Operators.Allowlist) > 0 && sb.Operators.Enforce {
		found := false
		for _, a := range sb.Operators.Allowlist {
			if a == operatorID {
				found = true
				break
			}
		}
		if !found {
			return fmt.Errorf("operator '%s' not in operators.allowlist", operatorID)
		}
	}
	return nil
}

func ResolveEscalation(auditPath, receiptID, operatorID, outcome, note string, sb *switchboard.Config) (map[string]any, error) {
	if outcome != "APPROVE" && outcome != "REJECT" {
		return nil, fmt.Errorf("outcome must be APPROVE or REJECT")
	}
	recs, err := audit.Read(auditPath)
	if err != nil {
		return nil, err
	}
	d := audit.FindDecision(recs, receiptID)
	if d == nil {
		return nil, fmt.Errorf("no decision for %s", receiptID)
	}
	if d["decision"] != "REQUIRE_APPROVAL" {
		return nil, fmt.Errorf("only REQUIRE_APPROVAL can be resolved")
	}
	if ops := audit.FindOperatorActions(recs, receiptID); len(ops) > 0 {
		return nil, fmt.Errorf("receipt already resolved")
	}
	if err := AssertOperator(operatorID, d, sb); err != nil {
		return nil, err
	}
	authStatus := "AUTHORIZED"
	if outcome == "REJECT" {
		authStatus = "DENIED"
	}
	rec := map[string]any{
		"record_type":          "tlpx.operator_action",
		"standard":             gate.StandardID,
		"standard_version":     gate.StandardVersion,
		"receipt_id":           receiptID,
		"linked_receipt_id":    receiptID,
		"acted_at":             time.Now().UTC().Format(time.RFC3339Nano),
		"operator":             map[string]any{"id": operatorID, "type": "human"},
		"outcome":              outcome,
		"note":                 note,
		"authorization_status": authStatus,
		"original_intent":      d["original_intent"],
		"parties": map[string]any{
			"declarer":  partyField(d, "declarer"),
			"evaluator": partyField(d, "evaluator"),
			"authorizer": map[string]any{
				"id": operatorID, "type": "human", "outcome": outcome,
			},
		},
		"implementation": "go-0.3",
	}
	return audit.Append(auditPath, rec)
}

func RecordExecution(auditPath, receiptID, executorID, status, summary string) (map[string]any, error) {
	auth, err := audit.ResolveAuth(auditPath, receiptID)
	if err != nil {
		return nil, err
	}
	authStatus, _ := auth["authorization_status"].(string)
	if status == "EXECUTED" && authStatus != "AUTHORIZED" {
		return nil, fmt.Errorf("cannot EXECUTED when authorization_status=%s", authStatus)
	}
	d, _ := auth["decision"].(map[string]any)
	var orig any
	if d != nil {
		orig = d["original_intent"]
	}
	rec := map[string]any{
		"record_type":          "tlpx.execution",
		"standard":             gate.StandardID,
		"standard_version":     gate.StandardVersion,
		"receipt_id":           receiptID,
		"linked_receipt_id":    receiptID,
		"executed_at":          time.Now().UTC().Format(time.RFC3339Nano),
		"status":               status,
		"result_summary":       summary,
		"executor":             map[string]any{"id": executorID, "type": "machine"},
		"authorization_status": authStatus,
		"authorization_source": auth["source"],
		"original_intent":      orig,
		"implementation":       "go-0.3",
	}
	return audit.Append(auditPath, rec)
}

// ExecuteAuthorized fail-closed: runs fn only if AUTHORIZED.
func ExecuteAuthorized(auditPath, receiptID, executorID string, fn func() (string, error)) (map[string]any, error) {
	auth, err := audit.ResolveAuth(auditPath, receiptID)
	if err != nil {
		return nil, err
	}
	if auth["authorization_status"] != "AUTHORIZED" {
		rec, rerr := RecordExecution(auditPath, receiptID, executorID, "BLOCKED",
			fmt.Sprintf("Blocked: %v", auth["authorization_status"]))
		out := map[string]any{"ok": false, "reason": auth["authorization_status"], "auth": auth}
		if rerr == nil {
			out["execution"] = rec
		}
		return out, nil
	}
	summary, err := fn()
	if err != nil {
		rec, _ := RecordExecution(auditPath, receiptID, executorID, "FAILED", err.Error())
		return map[string]any{"ok": false, "reason": err.Error(), "execution": rec, "auth": auth}, nil
	}
	rec, err := RecordExecution(auditPath, receiptID, executorID, "EXECUTED", summary)
	if err != nil {
		return nil, err
	}
	return map[string]any{"ok": true, "result": summary, "execution": rec, "auth": auth}, nil
}

func partyField(d map[string]any, name string) any {
	parties, ok := d["parties"].(map[string]any)
	if !ok {
		return nil
	}
	return parties[name]
}
