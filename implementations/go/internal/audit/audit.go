package audit

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

const GenesisHash = "0000000000000000000000000000000000000000000000000000000000000000"

// CanonicalJSON matches the JS reference: sorted keys, drop audit_hash + seal.
func CanonicalJSON(v any) (string, error) {
	return canonical(v)
}

func canonical(v any) (string, error) {
	switch t := v.(type) {
	case nil:
		return "null", nil
	case bool:
		if t {
			return "true", nil
		}
		return "false", nil
	case float64:
		// JSON numbers
		b, err := json.Marshal(t)
		return string(b), err
	case int:
		return fmt.Sprintf("%d", t), nil
	case string:
		b, err := json.Marshal(t)
		return string(b), err
	case []any:
		parts := make([]string, len(t))
		for i, x := range t {
			s, err := canonical(x)
			if err != nil {
				return "", err
			}
			parts[i] = s
		}
		return "[" + strings.Join(parts, ",") + "]", nil
	case map[string]any:
		keys := make([]string, 0, len(t))
		for k := range t {
			if k == "audit_hash" || k == "seal" {
				continue
			}
			keys = append(keys, k)
		}
		sort.Strings(keys)
		parts := make([]string, 0, len(keys))
		for _, k := range keys {
			vs, err := canonical(t[k])
			if err != nil {
				return "", err
			}
			kb, _ := json.Marshal(k)
			parts = append(parts, string(kb)+":"+vs)
		}
		return "{" + strings.Join(parts, ",") + "}", nil
	default:
		// Fallback marshal
		b, err := json.Marshal(t)
		return string(b), err
	}
}

func sha256Hex(s string) string {
	sum := sha256.Sum256([]byte(s))
	return hex.EncodeToString(sum[:])
}

func hmacHex(key []byte, data string) string {
	m := hmac.New(sha256.New, key)
	m.Write([]byte(data))
	return hex.EncodeToString(m.Sum(nil))
}

func SealKeyPath(logPath string) string {
	dir := filepath.Dir(logPath)
	base := filepath.Base(logPath)
	return filepath.Join(dir, "."+base+".seal")
}

func ResolveSealKey(logPath string, createIfMissing bool) ([]byte, error) {
	if env := os.Getenv("TLPX_AUDIT_SEAL"); env != "" {
		return []byte(env), nil
	}
	kp := SealKeyPath(logPath)
	if b, err := os.ReadFile(kp); err == nil {
		return b, nil
	}
	if !createIfMissing {
		return nil, fmt.Errorf("missing seal key file: %s", kp)
	}
	if err := os.MkdirAll(filepath.Dir(kp), 0o700); err != nil {
		return nil, err
	}
	key := make([]byte, 32)
	if _, err := rand.Read(key); err != nil {
		return nil, err
	}
	if err := os.WriteFile(kp, key, 0o600); err != nil {
		return nil, err
	}
	return key, nil
}

func lastHash(logPath string) string {
	b, err := os.ReadFile(logPath)
	if err != nil || len(strings.TrimSpace(string(b))) == 0 {
		return GenesisHash
	}
	lines := strings.Split(strings.TrimSpace(string(b)), "\n")
	var last map[string]any
	if err := json.Unmarshal([]byte(lines[len(lines)-1]), &last); err != nil {
		return GenesisHash
	}
	if h, ok := last["audit_hash"].(string); ok && h != "" {
		return h
	}
	return GenesisHash
}

// Append seals and writes one JSONL record.
func Append(logPath string, record map[string]any) (map[string]any, error) {
	if err := os.MkdirAll(filepath.Dir(logPath), 0o755); err != nil {
		return nil, err
	}
	key, err := ResolveSealKey(logPath, true)
	if err != nil {
		return nil, err
	}
	prev := lastHash(logPath)
	rec := cloneMap(record)
	rec["prev_hash"] = prev
	canon, err := canonical(rec)
	if err != nil {
		return nil, err
	}
	ah := sha256Hex(canon)
	rec["audit_hash"] = ah
	rec["seal"] = hmacHex(key, ah)
	line, err := json.Marshal(rec)
	if err != nil {
		return nil, err
	}
	f, err := os.OpenFile(logPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	if _, err := f.Write(append(line, '\n')); err != nil {
		return nil, err
	}
	return rec, nil
}

type VerifyResult struct {
	OK     bool     `json:"ok"`
	Lines  int      `json:"lines"`
	Errors []string `json:"errors"`
}

func Verify(logPath string) VerifyResult {
	b, err := os.ReadFile(logPath)
	if err != nil {
		if os.IsNotExist(err) {
			return VerifyResult{OK: true, Lines: 0}
		}
		return VerifyResult{OK: false, Errors: []string{err.Error()}}
	}
	text := strings.TrimSpace(string(b))
	if text == "" {
		return VerifyResult{OK: true, Lines: 0}
	}
	key, err := ResolveSealKey(logPath, false)
	if err != nil {
		return VerifyResult{OK: false, Errors: []string{err.Error()}}
	}
	lines := strings.Split(text, "\n")
	expectedPrev := GenesisHash
	var errors []string
	for i, line := range lines {
		var rec map[string]any
		if err := json.Unmarshal([]byte(line), &rec); err != nil {
			errors = append(errors, fmt.Sprintf("line %d: corrupt JSON", i+1))
			break
		}
		ah, _ := rec["audit_hash"].(string)
		se, _ := rec["seal"].(string)
		ph, _ := rec["prev_hash"].(string)
		if ah == "" || se == "" || ph == "" {
			errors = append(errors, fmt.Sprintf("line %d: missing integrity fields", i+1))
			continue
		}
		if ph != expectedPrev {
			errors = append(errors, fmt.Sprintf("line %d: prev_hash mismatch", i+1))
		}
		body := cloneMap(rec)
		delete(body, "audit_hash")
		delete(body, "seal")
		canon, err := canonical(body)
		if err != nil {
			errors = append(errors, fmt.Sprintf("line %d: %v", i+1, err))
			continue
		}
		if sha256Hex(canon) != ah {
			errors = append(errors, fmt.Sprintf("line %d: audit_hash mismatch", i+1))
		}
		if hmacHex(key, ah) != se {
			errors = append(errors, fmt.Sprintf("line %d: seal mismatch", i+1))
		}
		expectedPrev = ah
	}
	return VerifyResult{OK: len(errors) == 0, Lines: len(lines), Errors: errors}
}

func Read(logPath string) ([]map[string]any, error) {
	v := Verify(logPath)
	if !v.OK {
		return nil, fmt.Errorf("audit integrity failed: %s", strings.Join(v.Errors, "; "))
	}
	b, err := os.ReadFile(logPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	text := strings.TrimSpace(string(b))
	if text == "" {
		return nil, nil
	}
	var out []map[string]any
	for i, line := range strings.Split(text, "\n") {
		var rec map[string]any
		if err := json.Unmarshal([]byte(line), &rec); err != nil {
			return nil, fmt.Errorf("line %d: %w", i+1, err)
		}
		out = append(out, rec)
	}
	return out, nil
}

func FindDecision(records []map[string]any, receiptID string) map[string]any {
	for _, r := range records {
		rt, _ := r["record_type"].(string)
		if (rt == "tlpx.decision" || rt == "glass.decision") && r["receipt_id"] == receiptID {
			return r
		}
	}
	return nil
}

func FindOperatorActions(records []map[string]any, receiptID string) []map[string]any {
	var out []map[string]any
	for _, r := range records {
		rt, _ := r["record_type"].(string)
		if rt != "tlpx.operator_action" && rt != "glass.operator_action" {
			continue
		}
		if r["receipt_id"] == receiptID || r["linked_receipt_id"] == receiptID {
			out = append(out, r)
		}
	}
	return out
}

// ResolveAuth derives authorization from sealed audit (JS parity).
func ResolveAuth(logPath, receiptID string) (map[string]any, error) {
	recs, err := Read(logPath)
	if err != nil {
		return nil, err
	}
	d := FindDecision(recs, receiptID)
	if d == nil {
		return nil, fmt.Errorf("no decision for receipt_id=%s", receiptID)
	}
	ops := FindOperatorActions(recs, receiptID)
	if len(ops) > 1 {
		return nil, fmt.Errorf("corrupt audit: multiple operator actions")
	}
	var op map[string]any
	if len(ops) == 1 {
		op = ops[0]
	}
	dec, _ := d["decision"].(string)
	switch dec {
	case "ALLOW":
		return map[string]any{
			"authorization_status": "AUTHORIZED",
			"decision":             d,
			"operator_action":      nil,
			"source":               "policy_allow",
			"terminal":             true,
		}, nil
	case "DENY":
		return map[string]any{
			"authorization_status": "DENIED",
			"decision":             d,
			"operator_action":      nil,
			"source":               "gate_deny",
			"terminal":             true,
		}, nil
	case "REQUIRE_APPROVAL":
		if op == nil {
			return map[string]any{
				"authorization_status": "PENDING_HUMAN_APPROVAL",
				"decision":             d,
				"operator_action":      nil,
				"source":               "awaiting_operator",
				"terminal":             false,
			}, nil
		}
		outcome, _ := op["outcome"].(string)
		if outcome == "APPROVE" {
			return map[string]any{
				"authorization_status": "AUTHORIZED",
				"decision":             d,
				"operator_action":      op,
				"source":               "human_approve",
				"terminal":             true,
			}, nil
		}
		return map[string]any{
			"authorization_status": "DENIED",
			"decision":             d,
			"operator_action":      op,
			"source":               "human_reject",
			"terminal":             true,
		}, nil
	default:
		return nil, fmt.Errorf("unknown decision %s", dec)
	}
}

func cloneMap(m map[string]any) map[string]any {
	out := make(map[string]any, len(m))
	for k, v := range m {
		out[k] = v
	}
	return out
}
