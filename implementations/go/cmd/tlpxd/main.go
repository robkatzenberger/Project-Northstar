// tlpxd — historical cooperative 0.1-era HTTP reference.
//
// This service trusts caller-supplied actor identifiers. It is not the TL-PX
// 0.2 Rust authority, an authenticated adapter endpoint, or a PEP.
//
// Endpoints:
//   POST /v1/evaluate          { intent, policy_path?, switchboard_path? }
//   POST /v1/operator-action   { receipt_id, operator_id, outcome, note? }
//   POST /v1/execution         { receipt_id, executor_id, status, summary? }
//   GET  /v1/auth/{receipt_id}
//   GET  /v1/verify
//   GET  /healthz
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/audit"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/ops"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/policy"
	"github.com/Trust-Layer-AI/Project-Northstar/implementations/go/internal/switchboard"
)

func main() {
	addr := flag.String("addr", "127.0.0.1:8787", "listen address (default localhost only)")
	auditPath := flag.String("log", "", "audit JSONL path (required)")
	policyPath := flag.String("policy", "", "default policy.yaml")
	sbPath := flag.String("switchboard", "", "default switchboard.json")
	flag.Parse()
	if *auditPath == "" {
		fmt.Fprintln(os.Stderr, "--log is required")
		os.Exit(1)
	}
	if *policyPath == "" {
		// try relative to repo javascript config
		*policyPath = filepath.Join("..", "javascript", "config", "policy.yaml")
	}

	pol, err := policy.LoadFile(*policyPath)
	if err != nil {
		fmt.Fprintln(os.Stderr, "policy:", err)
		os.Exit(1)
	}
	var sb *switchboard.Config
	if *sbPath != "" {
		sb, err = switchboard.LoadFile(*sbPath)
		if err != nil {
			fmt.Fprintln(os.Stderr, "switchboard:", err)
			os.Exit(1)
		}
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, 200, map[string]any{"ok": true, "service": "tlpxd"})
	})
	mux.HandleFunc("/v1/verify", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", 405)
			return
		}
		v := audit.Verify(*auditPath)
		code := 200
		if !v.OK {
			code = 409
		}
		writeJSON(w, code, v)
	})
	mux.HandleFunc("/v1/auth/", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "method not allowed", 405)
			return
		}
		id := strings.TrimPrefix(r.URL.Path, "/v1/auth/")
		auth, err := audit.ResolveAuth(*auditPath, id)
		if err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		writeJSON(w, 200, auth)
	})
	mux.HandleFunc("/v1/evaluate", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", 405)
			return
		}
		var body struct {
			Intent           map[string]any `json:"intent"`
			PolicyPath       string         `json:"policy_path"`
			SwitchboardPath  string         `json:"switchboard_path"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		p := pol
		if body.PolicyPath != "" {
			p, err = policy.LoadFile(body.PolicyPath)
			if err != nil {
				http.Error(w, err.Error(), 400)
				return
			}
		}
		sbc := sb
		if body.SwitchboardPath != "" {
			sbc, err = switchboard.LoadFile(body.SwitchboardPath)
			if err != nil {
				http.Error(w, err.Error(), 400)
				return
			}
		}
		intent := normalizeHTTPIntent(body.Intent)
		d, err := ops.EvaluateAndAppend(intent, p, sbc, *auditPath)
		if err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		writeJSON(w, 200, d)
	})
	mux.HandleFunc("/v1/operator-action", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", 405)
			return
		}
		var body struct {
			ReceiptID  string `json:"receipt_id"`
			OperatorID string `json:"operator_id"`
			Outcome    string `json:"outcome"`
			Note       string `json:"note"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		rec, err := ops.ResolveEscalation(*auditPath, body.ReceiptID, body.OperatorID, body.Outcome, body.Note, sb)
		if err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		writeJSON(w, 200, rec)
	})
	mux.HandleFunc("/v1/execution", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", 405)
			return
		}
		var body struct {
			ReceiptID  string `json:"receipt_id"`
			ExecutorID string `json:"executor_id"`
			Status     string `json:"status"`
			Summary    string `json:"summary"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		rec, err := ops.RecordExecution(*auditPath, body.ReceiptID, body.ExecutorID, body.Status, body.Summary)
		if err != nil {
			http.Error(w, err.Error(), 400)
			return
		}
		writeJSON(w, 200, rec)
	})

	fmt.Fprintf(os.Stderr, "tlpxd listening on http://%s  audit=%s\n", *addr, *auditPath)
	if err := http.ListenAndServe(*addr, mux); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	enc := json.NewEncoder(w)
	enc.SetIndent("", "  ")
	_ = enc.Encode(v)
}

func normalizeHTTPIntent(flat map[string]any) policy.Intent {
	intent := policy.Intent{}
	if flat == nil {
		return intent
	}
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
	for _, k := range []string{"action", "target", "risk", "intent_id"} {
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
	if _, ok := intent["risk"]; !ok {
		intent["risk"] = "low"
	}
	if _, ok := intent["intent_id"]; !ok {
		intent["intent_id"] = fmt.Sprintf("http_%d", os.Getpid())
	}
	return intent
}
