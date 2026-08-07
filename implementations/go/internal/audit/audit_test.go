package audit

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSealAndVerify(t *testing.T) {
	dir := t.TempDir()
	log := filepath.Join(dir, "a.jsonl")
	_, err := Append(log, map[string]any{
		"record_type": "tlpx.decision",
		"receipt_id":  "r1",
		"decision":    "ALLOW",
	})
	if err != nil {
		t.Fatal(err)
	}
	v := Verify(log)
	if !v.OK {
		t.Fatalf("verify: %v", v.Errors)
	}
	// raw forge
	f, _ := os.OpenFile(log, os.O_APPEND|os.O_WRONLY, 0o600)
	_, _ = f.WriteString(`{"record_type":"tlpx.decision","receipt_id":"evil","decision":"ALLOW"}` + "\n")
	_ = f.Close()
	v2 := Verify(log)
	if v2.OK {
		t.Fatal("expected verify fail on unsealed line")
	}
}
