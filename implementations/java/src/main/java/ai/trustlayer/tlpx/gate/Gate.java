package ai.trustlayer.tlpx.gate;

import ai.trustlayer.tlpx.policy.PolicyEngine;
import ai.trustlayer.tlpx.policy.PolicyEngine.Outcome;
import ai.trustlayer.tlpx.policy.PolicyEngine.Policy;

import java.security.SecureRandom;
import java.time.Instant;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.Map;

public final class Gate {
  public static final String STANDARD_ID = "TL-PX";
  public static final String STANDARD_VERSION = "0.1.0";
  public static final String CONTROL_MODE = "ALLOW_OR_ESCALATE";

  private static final SecureRandom RNG = new SecureRandom();

  private Gate() {}

  public static Map<String, Object> evaluateIntent(Map<String, Object> intent, Policy policy) {
    String actor = str(intent.get("actor"));
    if (actor == null || actor.isBlank()) {
      throw new IllegalArgumentException("actor required");
    }
    String declared = str(intent.get("declared_intent"));
    if (declared == null || declared.isBlank()) {
      declared = str(intent.get("intent_summary"));
    }
    if (declared == null || declared.isBlank()) {
      throw new IllegalArgumentException("declared_intent required");
    }

    Outcome out = PolicyEngine.evaluate(intent, policy);
    String auth = "ALLOW".equals(out.decision()) ? "AUTHORIZED" : "PENDING_HUMAN_APPROVAL";
    boolean blocking = !"ALLOW".equals(out.decision());

    Map<String, Object> parties = new LinkedHashMap<>();
    parties.put("declarer", Map.of(
        "id", actor,
        "type", str(intent.getOrDefault("actor_type", "machine"))
    ));
    parties.put("evaluator", Map.of("id", "tlpx-java", "type", "machine"));
    parties.put("authorizer", null);

    Map<String, Object> d = new LinkedHashMap<>();
    d.put("record_type", "tlpx.decision");
    d.put("standard", STANDARD_ID);
    d.put("standard_version", STANDARD_VERSION);
    d.put("control_mode", CONTROL_MODE);
    d.put("blocking", blocking);
    d.put("receipt_id", receiptId(str(intent.get("intent_id"))));
    d.put("evaluated_at", Instant.now().toString());
    d.put("decision", out.decision());
    d.put("reason", out.reason());
    d.put("policy_id", out.policyId());
    d.put("policy_pack_id", policy.packId());
    d.put("reward_signal", blocking ? "TRANSPARENCY_REWARDED" : "AUTO_ALLOW");
    d.put("authorization_status", auth);
    d.put("parties", parties);
    d.put("original_intent", intent);
    d.put("implementation", "java-skeleton-0.1");
    return d;
  }

  private static String receiptId(String intentId) {
    if (intentId == null || intentId.isBlank()) intentId = "unknown";
    byte[] b = new byte[3];
    RNG.nextBytes(b);
    String stamp = Instant.now().toString().replace("-", "").replace(":", "").replace(".", "");
    return "rcpt_" + intentId.replaceAll("[^A-Za-z0-9_-]", "_") + "_" + stamp + "_" + HexFormat.of().formatHex(b);
  }

  private static String str(Object o) {
    return o == null ? null : String.valueOf(o);
  }
}
