package ai.trustlayer.tlpx.policy;

import ai.trustlayer.tlpx.policy.PolicyEngine.Outcome;
import ai.trustlayer.tlpx.policy.PolicyEngine.Policy;
import org.junit.jupiter.api.Test;

import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

class PolicyEngineTest {
  private static Path policyPath() {
    // java/src/test/java/... -> implementations/javascript/config/policy.yaml
    return Path.of("").toAbsolutePath()
        .resolve("../javascript/config/policy.yaml")
        .normalize();
  }

  @Test
  void safeAllow() throws Exception {
    Policy p = PolicyEngine.load(policyPath());
    Outcome o = PolicyEngine.evaluate(Map.of(
        "actor", "agent.docs.summarizer",
        "action", "summarize_report",
        "risk", "low",
        "data_classes", List.of(),
        "low_credibility", false
    ), p);
    assertEquals("ALLOW", o.decision());
    assertNull(o.policyId());
  }

  @Test
  void piiEmailEscalates() throws Exception {
    Policy p = PolicyEngine.load(policyPath());
    Outcome o = PolicyEngine.evaluate(Map.of(
        "actor", "agent.support.mailer",
        "action", "send_email",
        "risk", "medium",
        "data_classes", List.of("PII"),
        "low_credibility", false
    ), p);
    assertEquals("REQUIRE_APPROVAL", o.decision());
    assertEquals("rule_pii_email", o.policyId());
  }
}
