package gate

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"time"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/switchboard"
)

const (
	StandardID      = "TL-PX"
	StandardVersion = "0.1.0"
	ControlMode     = "ALLOW_OR_ESCALATE"
)

// Decision is a minimal tlpx.decision-shaped map serializable to JSON.
type Decision map[string]any

// EvaluateOptions optional Switchboard.
type EvaluateOptions struct {
	Switchboard *switchboard.Config
}

// EvaluateIntent runs Switchboard (optional) then policy.
func EvaluateIntent(intent policy.Intent, p *policy.Policy, opts EvaluateOptions) (Decision, error) {
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

	action, _ := intent["action"].(string)
	actorType, _ := intent["actor_type"].(string)
	if actorType == "" {
		actorType = "machine"
	}

	var sbCtx *switchboard.Context
	if opts.Switchboard != nil {
		ctx := opts.Switchboard.Route(actor, action, actorType)
		sbCtx = &ctx
		if ctx.ActorTypeNormalized != "" {
			actorType = ctx.ActorTypeNormalized
			intent["actor_type"] = actorType
		}
		if ctx.Credibility != nil {
			intent["credibility"] = *ctx.Credibility
			intent["low_credibility"] = ctx.CredibilityBand == "low"
			intent["high_trust"] = ctx.CredibilityBand == "high"
			intent["whitelisted"] = ctx.Whitelisted
		}
	}

	var out policy.Outcome
	if sbCtx != nil && sbCtx.Gate != nil {
		pid := sbCtx.Gate.PolicyID
		out = policy.Outcome{
			Decision: sbCtx.Gate.Decision,
			Reason:   sbCtx.Gate.Reason,
			PolicyID: &pid,
		}
	} else {
		out = policy.Evaluate(intent, p)
	}

	rid := receiptID(fmt.Sprint(intent["intent_id"]))
	now := time.Now().UTC().Format(time.RFC3339Nano)

	auth := "AUTHORIZED"
	blocking := false
	reward := "AUTO_ALLOW"
	switch out.Decision {
	case "REQUIRE_APPROVAL":
		auth = "PENDING_HUMAN_APPROVAL"
		blocking = true
		reward = "TRANSPARENCY_REWARDED"
	case "DENY":
		auth = "DENIED"
		blocking = true
		reward = "SWITCHBOARD_DENIED"
	}

	parties := map[string]any{
		"declarer": map[string]any{
			"id":   actor,
			"type": actorType,
		},
		"evaluator":  map[string]any{"id": "tlpx-go", "type": "machine"},
		"authorizer": nil,
	}
	var approvalRoute []string
	var sbSummary any
	if sbCtx != nil {
		parties["router"] = map[string]any{"id": sbCtx.SwitchboardID, "type": "machine"}
		approvalRoute = sbCtx.ApprovalRoute
		var cred any
		if sbCtx.Credibility != nil {
			cred = *sbCtx.Credibility
		}
		sbSummary = map[string]any{
			"switchboard_id":    sbCtx.SwitchboardID,
			"lookup":            sbCtx.Lookup,
			"whitelisted":       sbCtx.Whitelisted,
			"credibility":       cred,
			"credibility_band":  sbCtx.CredibilityBand,
			"flags":             sbCtx.Flags,
			"approval_route":    sbCtx.ApprovalRoute,
		}
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
		"parties":              parties,
		"approval_route":       approvalRoute,
		"switchboard":          sbSummary,
		"original_intent":      intent,
		"implementation":       "go-skeleton-0.2",
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
