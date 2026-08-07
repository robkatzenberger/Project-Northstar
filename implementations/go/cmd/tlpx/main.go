// tlpx — minimal Go skeleton for TL-PX evaluate (policy only; no Switchboard yet).
package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/gate"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
)

func main() {
	if len(os.Args) < 2 {
		usage()
		os.Exit(1)
	}
	switch os.Args[1] {
	case "evaluate":
		if len(os.Args) != 4 {
			fmt.Fprintln(os.Stderr, "usage: tlpx evaluate <intent.json> <policy.yaml>")
			os.Exit(1)
		}
		if err := cmdEvaluate(os.Args[2], os.Args[3]); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	case "help", "--help", "-h":
		usage()
	default:
		fmt.Fprintln(os.Stderr, "unknown command:", os.Args[1])
		usage()
		os.Exit(1)
	}
}

func usage() {
	fmt.Println(`tlpx (Go skeleton)

  evaluate <intent.json> <policy.yaml>

Minimal TL-PX-shaped evaluate. No Switchboard, no sealed audit yet.
Shared fixtures: ../../javascript/examples/
Policy: ../../javascript/config/policy.yaml
`)
}

func cmdEvaluate(intentPath, policyPath string) error {
	raw, err := os.ReadFile(intentPath)
	if err != nil {
		return err
	}
	var flat map[string]any
	if err := json.Unmarshal(raw, &flat); err != nil {
		return err
	}
	intent := normalizeIntent(flat)
	p, err := policy.LoadFile(policyPath)
	if err != nil {
		return err
	}
	d, err := gate.EvaluateIntent(intent, p)
	if err != nil {
		return err
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(d)
}

func normalizeIntent(flat map[string]any) policy.Intent {
	intent := policy.Intent{}
	// support example-style { agent, intent_summary, ... }
	if agent, ok := flat["agent"].(string); ok {
		intent["actor"] = agent
	}
	if actor, ok := flat["actor"].(string); ok {
		intent["actor"] = actor
	}
	if s, ok := flat["intent_summary"].(string); ok {
		intent["declared_intent"] = s
		intent["intent_summary"] = s
	}
	if s, ok := flat["declared_intent"].(string); ok {
		intent["declared_intent"] = s
	}
	if s, ok := flat["actor_type"].(string); ok {
		intent["actor_type"] = s
	} else {
		intent["actor_type"] = "machine"
	}
	for _, k := range []string{"action", "target", "risk"} {
		if v, ok := flat[k]; ok {
			intent[k] = v
		}
	}
	if dc, ok := flat["data_classes"].([]any); ok {
		var ss []string
		for _, x := range dc {
			if s, ok := x.(string); ok {
				ss = append(ss, s)
			}
		}
		intent["data_classes"] = ss
	} else {
		intent["data_classes"] = []string{}
	}
	if id, ok := flat["intent_id"].(string); ok {
		intent["intent_id"] = id
	} else if id, ok := flat["prism_id"].(string); ok {
		intent["intent_id"] = id
	} else {
		intent["intent_id"] = fmt.Sprintf("go_%d", os.Getpid())
	}
	if risk, ok := intent["risk"].(string); !ok || risk == "" {
		intent["risk"] = "low"
	}
	return intent
}
