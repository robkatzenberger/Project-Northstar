// tlpx — Go reference CLI: evaluate, approve, execute, verify (+ optional Switchboard / sealed audit).
package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/audit"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/gate"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/ops"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/switchboard"
)

func main() {
	if len(os.Args) < 2 {
		usage()
		os.Exit(1)
	}
	var err error
	switch os.Args[1] {
	case "evaluate":
		err = cmdEvaluate(os.Args[2:])
	case "approve":
		err = cmdResolve(os.Args[2:], "APPROVE")
	case "reject":
		err = cmdResolve(os.Args[2:], "REJECT")
	case "execute":
		err = cmdExecute(os.Args[2:])
	case "auth":
		err = cmdAuth(os.Args[2:])
	case "verify":
		err = cmdVerify(os.Args[2:])
	case "help", "--help", "-h":
		usage()
	default:
		fmt.Fprintln(os.Stderr, "unknown command:", os.Args[1])
		usage()
		os.Exit(1)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func usage() {
	fmt.Println(`tlpx (Go)

  evaluate <intent.json> <policy.yaml> [--switchboard PATH] [--log PATH]
  approve  <receipt_id> --operator ID --log PATH [--switchboard PATH]
  reject   <receipt_id> --operator ID --log PATH [--switchboard PATH]
  execute  <receipt_id> --executor ID --status STATUS --log PATH
  auth     <receipt_id> --log PATH
  verify   --log PATH

When --log is set, decisions/actions are sealed into the audit JSONL.
`)
}

func cmdEvaluate(args []string) error {
	f := parseFlags(args)
	if len(f.pos) < 2 {
		return fmt.Errorf("usage: tlpx evaluate <intent.json> <policy.yaml> [--switchboard PATH] [--log PATH]")
	}
	intent, err := loadIntent(f.pos[0])
	if err != nil {
		return err
	}
	p, err := policy.LoadFile(f.pos[1])
	if err != nil {
		return err
	}
	var sb *switchboard.Config
	if f.switchboard != "" {
		sb, err = switchboard.LoadFile(f.switchboard)
		if err != nil {
			return err
		}
	}
	if f.log != "" {
		d, err := ops.EvaluateAndAppend(intent, p, sb, f.log)
		if err != nil {
			return err
		}
		return printJSON(d)
	}
	d, err := gate.EvaluateIntent(intent, p, gate.EvaluateOptions{Switchboard: sb})
	if err != nil {
		return err
	}
	return printJSON(d)
}

func cmdResolve(args []string, outcome string) error {
	f := parseFlags(args)
	if len(f.pos) < 1 || f.operator == "" || f.log == "" {
		return fmt.Errorf("usage: tlpx approve|reject <receipt_id> --operator ID --log PATH")
	}
	var sb *switchboard.Config
	var err error
	if f.switchboard != "" {
		sb, err = switchboard.LoadFile(f.switchboard)
		if err != nil {
			return err
		}
	}
	rec, err := ops.ResolveEscalation(f.log, f.pos[0], f.operator, outcome, f.note, sb)
	if err != nil {
		return err
	}
	return printJSON(rec)
}

func cmdExecute(args []string) error {
	f := parseFlags(args)
	if len(f.pos) < 1 || f.executor == "" || f.status == "" || f.log == "" {
		return fmt.Errorf("usage: tlpx execute <receipt_id> --executor ID --status STATUS --log PATH")
	}
	rec, err := ops.RecordExecution(f.log, f.pos[0], f.executor, f.status, f.summary)
	if err != nil {
		return err
	}
	return printJSON(rec)
}

func cmdAuth(args []string) error {
	f := parseFlags(args)
	if len(f.pos) < 1 || f.log == "" {
		return fmt.Errorf("usage: tlpx auth <receipt_id> --log PATH")
	}
	auth, err := audit.ResolveAuth(f.log, f.pos[0])
	if err != nil {
		return err
	}
	return printJSON(auth)
}

func cmdVerify(args []string) error {
	f := parseFlags(args)
	if f.log == "" {
		return fmt.Errorf("usage: tlpx verify --log PATH")
	}
	v := audit.Verify(f.log)
	if err := printJSON(v); err != nil {
		return err
	}
	if !v.OK {
		os.Exit(2)
	}
	return nil
}

type flags struct {
	pos         []string
	log         string
	switchboard string
	operator    string
	executor    string
	status      string
	note        string
	summary     string
}

func parseFlags(args []string) flags {
	var f flags
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch a {
		case "--log":
			i++
			if i < len(args) {
				f.log = args[i]
			}
		case "--switchboard":
			i++
			if i < len(args) {
				f.switchboard = args[i]
			}
		case "--operator":
			i++
			if i < len(args) {
				f.operator = args[i]
			}
		case "--executor":
			i++
			if i < len(args) {
				f.executor = args[i]
			}
		case "--status":
			i++
			if i < len(args) {
				f.status = args[i]
			}
		case "--note":
			i++
			if i < len(args) {
				f.note = args[i]
			}
		case "--summary":
			i++
			if i < len(args) {
				f.summary = args[i]
			}
		default:
			if len(a) > 0 && a[0] != '-' {
				f.pos = append(f.pos, a)
			}
		}
	}
	return f
}

func loadIntent(path string) (policy.Intent, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var flat map[string]any
	if err := json.Unmarshal(raw, &flat); err != nil {
		return nil, err
	}
	return normalizeIntent(flat), nil
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

func printJSON(v any) error {
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(v)
}
