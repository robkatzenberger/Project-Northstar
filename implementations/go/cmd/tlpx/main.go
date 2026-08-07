// tlpx — Go skeleton for TL-PX evaluate (+ optional Switchboard).
package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/gate"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/switchboard"
)

func main() {
	if len(os.Args) < 2 {
		usage()
		os.Exit(1)
	}
	switch os.Args[1] {
	case "evaluate":
		// tlpx evaluate <intent.json> <policy.yaml> [switchboard.json]
		if len(os.Args) < 4 || len(os.Args) > 5 {
			fmt.Fprintln(os.Stderr, "usage: tlpx evaluate <intent.json> <policy.yaml> [switchboard.json]")
			os.Exit(1)
		}
		var sbPath string
		if len(os.Args) == 5 {
			sbPath = os.Args[4]
		}
		if err := cmdEvaluate(os.Args[2], os.Args[3], sbPath); err != nil {
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

  evaluate <intent.json> <policy.yaml> [switchboard.json]

Switchboard-first DENY when switchboard.json is provided.
Shared fixtures: ../javascript/examples/
`)
}

func cmdEvaluate(intentPath, policyPath, sbPath string) error {
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
	opts := gate.EvaluateOptions{}
	if sbPath != "" {
		sb, err := switchboard.LoadFile(sbPath)
		if err != nil {
			return err
		}
		opts.Switchboard = sb
	}
	d, err := gate.EvaluateIntent(intent, p, opts)
	if err != nil {
		return err
	}
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(d)
}

func normalizeIntent(flat map[string]any) policy.Intent {
	intent := policy.Intent{}
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
