package switchboard

import (
	"path/filepath"
	"runtime"
	"testing"
)

func jsSwitchboard(t *testing.T) string {
	t.Helper()
	_, file, _, _ := runtime.Caller(0)
	return filepath.Join(filepath.Dir(file), "..", "..", "..", "javascript", "config", "switchboard.json")
}

func TestUnknownDeny(t *testing.T) {
	c, err := LoadFile(jsSwitchboard(t))
	if err != nil {
		t.Fatal(err)
	}
	ctx := c.Route("agent.rogue.unregistered", "summarize_report", "machine")
	if ctx.Gate == nil || ctx.Gate.Decision != "DENY" {
		t.Fatalf("expected DENY, got %+v", ctx.Gate)
	}
}

func TestNotWhitelisted(t *testing.T) {
	c, err := LoadFile(jsSwitchboard(t))
	if err != nil {
		t.Fatal(err)
	}
	ctx := c.Route("agent.shadow.unknown", "send_email", "machine")
	if ctx.Gate == nil || ctx.Gate.PolicyID != "switchboard.not_whitelisted" {
		t.Fatalf("got %+v", ctx.Gate)
	}
}

func TestHighTrustNoGate(t *testing.T) {
	c, err := LoadFile(jsSwitchboard(t))
	if err != nil {
		t.Fatal(err)
	}
	ctx := c.Route("agent.docs.summarizer", "summarize_report", "machine")
	if ctx.Gate != nil {
		t.Fatalf("expected no hard gate, got %+v", ctx.Gate)
	}
	if ctx.CredibilityBand != "high" {
		t.Fatalf("band=%s", ctx.CredibilityBand)
	}
}
