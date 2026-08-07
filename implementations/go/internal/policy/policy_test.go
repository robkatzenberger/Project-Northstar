package policy

import (
	"path/filepath"
	"runtime"
	"testing"
)

func jsPolicyPath(t *testing.T) string {
	t.Helper()
	_, file, _, _ := runtime.Caller(0)
	// internal/policy -> go -> implementations -> javascript/config
	root := filepath.Join(filepath.Dir(file), "..", "..", "..", "javascript", "config", "policy.yaml")
	return root
}

func TestEvaluateSafeAllow(t *testing.T) {
	p, err := LoadFile(jsPolicyPath(t))
	if err != nil {
		t.Fatal(err)
	}
	out := Evaluate(Intent{
		"actor":            "agent.docs.summarizer",
		"actor_type":       "machine",
		"declared_intent":  "sum",
		"action":           "summarize_report",
		"risk":             "low",
		"data_classes":     []string{},
		"low_credibility":  false,
	}, p)
	if out.Decision != "ALLOW" {
		t.Fatalf("got %s want ALLOW", out.Decision)
	}
}

func TestEvaluatePIIEmail(t *testing.T) {
	p, err := LoadFile(jsPolicyPath(t))
	if err != nil {
		t.Fatal(err)
	}
	out := Evaluate(Intent{
		"actor":           "agent.support.mailer",
		"action":          "send_email",
		"risk":            "medium",
		"data_classes":    []string{"PII"},
		"low_credibility": false,
	}, p)
	if out.Decision != "REQUIRE_APPROVAL" {
		t.Fatalf("got %s want REQUIRE_APPROVAL", out.Decision)
	}
	if out.PolicyID == nil || *out.PolicyID != "rule_pii_email" {
		t.Fatalf("policy_id=%v", out.PolicyID)
	}
}
