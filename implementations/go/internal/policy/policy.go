package policy

import (
	"fmt"
	"os"
	"regexp"
	"strings"
)

// Rule is a minimal TL-PX policy rule (first match wins).
type Rule struct {
	ID          string
	Description string
	If          string
	Require     string
}

type Policy struct {
	PackID string
	Rules  []Rule
}

type Outcome struct {
	Decision string // ALLOW | REQUIRE_APPROVAL
	Reason   string
	PolicyID *string
}

// LoadFile loads the flat YAML-like policy format used by the JS reference.
func LoadFile(path string) (*Policy, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return Parse(string(b))
}

func Parse(text string) (*Policy, error) {
	p := &Policy{PackID: "default"}
	var cur *Rule
	lines := strings.Split(text, "\n")
	reRule := regexp.MustCompile(`^\s*-\s+id:\s+(.+)$`)
	reField := regexp.MustCompile(`^\s+([A-Za-z_]+):\s+(.+)$`)

	for _, raw := range lines {
		line := strings.TrimRight(raw, "\r")
		trim := strings.TrimSpace(line)
		if trim == "" || strings.HasPrefix(trim, "#") || trim == "rules:" {
			continue
		}
		if m := reRule.FindStringSubmatch(line); m != nil {
			r := Rule{ID: unquote(strings.TrimSpace(m[1]))}
			p.Rules = append(p.Rules, r)
			cur = &p.Rules[len(p.Rules)-1]
			continue
		}
		if cur != nil {
			if m := reField.FindStringSubmatch(line); m != nil {
				key, val := m[1], unquote(strings.TrimSpace(m[2]))
				switch key {
				case "description":
					cur.Description = val
				case "if":
					cur.If = val
				case "require":
					cur.Require = val
				}
			}
		}
	}
	return p, nil
}

func unquote(s string) string {
	if len(s) >= 2 && s[0] == '"' && s[len(s)-1] == '"' {
		return s[1 : len(s)-1]
	}
	return s
}

// Intent is a flat evaluation map (string fields + string slices).
type Intent map[string]any

// Evaluate applies first-match rules. Supports a tiny expression subset.
func Evaluate(intent Intent, p *Policy) Outcome {
	for _, r := range p.Rules {
		if r.If == "" {
			continue
		}
		ok, err := evalCond(r.If, intent)
		if err != nil || !ok {
			continue
		}
		if r.Require != "" {
			id := r.ID
			reason := r.Description
			if reason == "" {
				reason = fmt.Sprintf("Policy requires %s", r.Require)
			}
			return Outcome{Decision: "REQUIRE_APPROVAL", Reason: reason, PolicyID: &id}
		}
	}
	return Outcome{Decision: "ALLOW", Reason: "No approval rules matched", PolicyID: nil}
}

var (
	reEq  = regexp.MustCompile(`^([A-Za-z_][A-Za-z0-9_]*)\s*==\s*"(.*)"$`)
	reIn  = regexp.MustCompile(`^"([^"]+)"\s+in\s+([A-Za-z_][A-Za-z0-9_]*)$`)
	reAnd = regexp.MustCompile(`\s+and\s+`)
)

func evalCond(expr string, intent Intent) (bool, error) {
	// Only support: field == "x" | "x" in field | and combinations
	parts := reAnd.Split(expr, -1)
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if m := reEq.FindStringSubmatch(part); m != nil {
			got, _ := intent[m[1]].(string)
			if got != m[2] {
				return false, nil
			}
			continue
		}
		if m := reIn.FindStringSubmatch(part); m != nil {
			arr, _ := intent[m[2]].([]string)
			if !contains(arr, m[1]) {
				return false, nil
			}
			continue
		}
		// low_credibility == true style
		if strings.Contains(part, "==") {
			bits := strings.SplitN(part, "==", 2)
			if len(bits) == 2 {
				field := strings.TrimSpace(bits[0])
				want := strings.TrimSpace(bits[1])
				val := intent[field]
				if want == "true" {
					b, _ := val.(bool)
					if !b {
						return false, nil
					}
					continue
				}
				if want == "false" {
					b, _ := val.(bool)
					if b {
						return false, nil
					}
					continue
				}
			}
		}
		return false, fmt.Errorf("unsupported expression: %s", part)
	}
	return true, nil
}

func contains(arr []string, item string) bool {
	for _, a := range arr {
		if a == item {
			return true
		}
	}
	return false
}
