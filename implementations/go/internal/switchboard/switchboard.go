package switchboard

import (
	"encoding/json"
	"fmt"
	"os"
)

const (
	CredMin = 0.0
	CredMax = 0.99
)

type Principal struct {
	ID             string   `json:"id"`
	Type           string   `json:"type"`
	Label          string   `json:"label"`
	Whitelisted    bool     `json:"whitelisted"`
	Credibility    float64  `json:"credibility"`
	AllowedActions []string `json:"allowed_actions"`
	ApprovalRoute  []string `json:"approval_route"`
}

type Operators struct {
	Enforce   bool     `json:"enforce"`
	Allowlist []string `json:"allowlist"`
}

type Config struct {
	SwitchboardID       string             `json:"switchboard_id"`
	UnknownAgentPolicy  string             `json:"unknown_agent_policy"`
	Thresholds          Thresholds         `json:"thresholds"`
	Defaults            Defaults           `json:"defaults"`
	Operators           *Operators         `json:"operators"`
	Principals          []Principal        `json:"principals"`
	byID                map[string]Principal
}

type Thresholds struct {
	ForceEscalateBelow  float64 `json:"force_escalate_below"`
	HighTrustAtOrAbove  float64 `json:"high_trust_at_or_above"`
	MaxCredibility      float64 `json:"max_credibility"`
}

type Defaults struct {
	ApprovalRoute []string `json:"approval_route"`
}

type Gate struct {
	Decision string
	Reason   string
	PolicyID string
}

type Context struct {
	SwitchboardID         string
	Lookup                string
	PrincipalID           string
	Whitelisted           bool
	Credibility           *float64
	CredibilityBand       string
	ApprovalRoute         []string
	Flags                 []string
	ActorTypeNormalized   string
	Gate                  *Gate
}

func LoadFile(path string) (*Config, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var raw Config
	if err := json.Unmarshal(b, &raw); err != nil {
		return nil, err
	}
	if raw.SwitchboardID == "" {
		raw.SwitchboardID = "switchboard"
	}
	if raw.UnknownAgentPolicy != "REQUIRE_APPROVAL" {
		raw.UnknownAgentPolicy = "DENY"
	}
	if raw.Thresholds.ForceEscalateBelow == 0 {
		raw.Thresholds.ForceEscalateBelow = 0.4
	}
	if raw.Thresholds.HighTrustAtOrAbove == 0 {
		raw.Thresholds.HighTrustAtOrAbove = 0.85
	}
	if raw.Thresholds.MaxCredibility == 0 {
		raw.Thresholds.MaxCredibility = CredMax
	}
	raw.byID = make(map[string]Principal)
	for _, p := range raw.Principals {
		if p.Credibility < CredMin || p.Credibility > raw.Thresholds.MaxCredibility {
			return nil, fmt.Errorf("principal %s credibility out of range", p.ID)
		}
		raw.byID[p.ID] = p
	}
	return &raw, nil
}

func (c *Config) Lookup(actorID string) (Principal, bool) {
	p, ok := c.byID[actorID]
	return p, ok
}

// Route applies Switchboard-first hard gates and enrichment flags.
func (c *Config) Route(actorID, action, actorType string) Context {
	ctx := Context{
		SwitchboardID: c.SwitchboardID,
		PrincipalID:   actorID,
		Flags:         []string{},
		ApprovalRoute: append([]string{}, c.Defaults.ApprovalRoute...),
	}

	p, ok := c.Lookup(actorID)
	if !ok {
		ctx.Lookup = "unknown"
		ctx.CredibilityBand = "unknown"
		ctx.Flags = append(ctx.Flags, "UNKNOWN_PRINCIPAL")
		if c.UnknownAgentPolicy == "REQUIRE_APPROVAL" {
			ctx.Gate = &Gate{
				Decision: "REQUIRE_APPROVAL",
				Reason:   "Switchboard: unknown principal requires human approval",
				PolicyID: "switchboard.unknown_escalate",
			}
		} else {
			ctx.Gate = &Gate{
				Decision: "DENY",
				Reason:   "Switchboard: principal not registered",
				PolicyID: "switchboard.unknown_deny",
			}
		}
		return ctx
	}

	ctx.Lookup = "known"
	ctx.Whitelisted = p.Whitelisted
	cred := p.Credibility
	ctx.Credibility = &cred
	ctx.CredibilityBand = band(cred, c.Thresholds)
	if p.Type == "human" {
		ctx.ActorTypeNormalized = "human"
	} else {
		ctx.ActorTypeNormalized = "machine"
	}
	if len(p.ApprovalRoute) > 0 {
		ctx.ApprovalRoute = append([]string{}, p.ApprovalRoute...)
	}

	if !p.Whitelisted {
		ctx.Flags = append(ctx.Flags, "NOT_WHITELISTED")
		ctx.Gate = &Gate{
			Decision: "DENY",
			Reason:   "Switchboard: principal not on whitelist",
			PolicyID: "switchboard.not_whitelisted",
		}
		return ctx
	}

	if p.AllowedActions != nil && action != "" && !contains(p.AllowedActions, action) {
		ctx.Flags = append(ctx.Flags, "ACTION_NOT_PERMITTED")
		ctx.Gate = &Gate{
			Decision: "DENY",
			Reason:   fmt.Sprintf("Switchboard: action '%s' not permitted for %s", action, p.ID),
			PolicyID: "switchboard.action_denied",
		}
		return ctx
	}

	if cred < c.Thresholds.ForceEscalateBelow {
		ctx.Flags = append(ctx.Flags, "LOW_CREDIBILITY")
	}
	if cred >= c.Thresholds.HighTrustAtOrAbove {
		ctx.Flags = append(ctx.Flags, "HIGH_TRUST")
	}
	return ctx
}

func band(score float64, t Thresholds) string {
	if score < t.ForceEscalateBelow {
		return "low"
	}
	if score >= t.HighTrustAtOrAbove {
		return "high"
	}
	return "medium"
}

func contains(arr []string, item string) bool {
	for _, a := range arr {
		if a == item {
			return true
		}
	}
	return false
}
