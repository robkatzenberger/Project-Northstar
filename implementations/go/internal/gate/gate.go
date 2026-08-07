package gate

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"time"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
)

const (
	StandardID      = "TL-PX"
	StandardVersion = "0.1.0"
	ControlMode     = "ALLOW_OR_ESCALATE"
)

// Decision is a minimal tlpx.decision-shaped map serializable to JSON.
type Decision map[string]any

// EvaluateIntent runs policy (no switchboard in this skeleton).
func EvaluateIntent(intent policy.Intent, p *policy.Policy) (Decision, error) {
	actor, _ := intent["actor"].(string)
	if actor == "" {
		return nil, fmt.Errorf("actor required")
	}
	declared, _ := intent["declared_intent"].(string)
	if declared == "" {
		if s, ok := intent["intent_summary"].(string); ok {
			declared = s
		}
	}
	if declared == "" {
		return nil, fmt.Errorf("declared_intent required")
	}

	out := policy.Evaluate(intent, p)
	rid := receiptID(fmt.Sprint(intent["intent_id"]))
	now := time.Now().UTC().Format(time.RFC3339Nano)

	auth := "AUTHORIZED"
	blocking := false
	reward := "AUTO_ALLOW"
	if out.Decision == "REQUIRE_APPROVAL" {
		auth = "PENDING_HUMAN_APPROVAL"
		blocking = true
		reward = "TRANSPARENCY_REWARDED"
	}

	actorType, _ := intent["actor_type"].(string)
	if actorType == "" {
		actorType = "machine"
	}

	d := Decision{
		"record_type":          "tlpx.decision",
		"standard":             StandardID,
		"standard_version":     StandardVersion,
		"control_mode":         ControlMode,
		"blocking":             blocking,
		"receipt_id":           rid,
		"evaluated_at":         now,
		"decision":             out.Decision,
		"reason":               out.Reason,
		"policy_id":            out.PolicyID,
		"policy_pack_id":       p.PackID,
		"reward_signal":        reward,
		"authorization_status": auth,
		"parties": map[string]any{
			"declarer":  map[string]any{"id": actor, "type": actorType},
			"evaluator": map[string]any{"id": "tlpx-go", "type": "machine"},
			"authorizer": nil,
		},
		"original_intent": intent,
		"implementation":  "go-skeleton-0.1",
	}
	return d, nil
}

func receiptID(intentID string) string {
	if intentID == "" || intentID == "<nil>" {
		intentID = "unknown"
	}
	b := make([]byte, 3)
	_, _ = rand.Read(b)
	stamp := time.Now().UTC().Format("20060102T150405Z")
	return fmt.Sprintf("rcpt_%s_%s_%s", sanitize(intentID), stamp, hex.EncodeToString(b))
}

func sanitize(s string) string {
	out := make([]rune, 0, len(s))
	for _, r := range s {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '_' || r == '-' {
			out = append(out, r)
		} else {
			out = append(out, '_')
		}
	}
	if len(out) > 40 {
		out = out[:40]
	}
	return string(out)
}
