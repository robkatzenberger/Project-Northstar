package ai.trustlayer.tlpx.gate;

import ai.trustlayer.tlpx.policy.PolicyEngine;
import ai.trustlayer.tlpx.policy.PolicyEngine.Outcome;
import ai.trustlayer.tlpx.policy.PolicyEngine.Policy;
import ai.trustlayer.tlpx.switchboard.Switchboard;
import ai.trustlayer.tlpx.switchboard.Switchboard.Context;

import java.security.SecureRandom;
import java.time.Instant;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class Gate {
  public static final String STANDARD_ID = "TL-PX";
  public static final String STANDARD_VERSION = "0.1.0";
  public static final String CONTROL_MODE = "ALLOW_OR_ESCALATE";

  private static final SecureRandom RNG = new SecureRandom();

  private Gate() {}

  public static Map<String, Object> evaluateIntent(Map<String, Object> intent, Policy policy) {
    return evaluateIntent(intent, policy, null);
  }

  public static Map<String, Object> evaluateIntent(
      Map<String, Object> intent, Policy policy, Switchboard switchboard
  ) {
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

    String action = str(intent.get("action"));
    String actorType = str(intent.getOrDefault("actor_type", "machine"));
    Context sbCtx = null;
    if (switchboard != null) {
      sbCtx = switchboard.route(actor, action, actorType);
      if (sbCtx.actorTypeNormalized() != null) {
        actorType = sbCtx.actorTypeNormalized();
        intent.put("actor_type", actorType);
      }
      if (sbCtx.credibility() != null) {
        intent.put("credibility", sbCtx.credibility());
        intent.put("low_credibility", "low".equals(sbCtx.credibilityBand()));
        intent.put("high_trust", "high".equals(sbCtx.credibilityBand()));
        intent.put("whitelisted", sbCtx.whitelisted());
      }
    }

    Outcome out;
    if (sbCtx != null && sbCtx.gate() != null) {
      out = new Outcome(sbCtx.gate().decision(), sbCtx.gate().reason(), sbCtx.gate().policyId());
    } else {
      out = PolicyEngine.evaluate(intent, policy);
    }

    String auth = "AUTHORIZED";
    boolean blocking = false;
    String reward = "AUTO_ALLOW";
    if ("REQUIRE_APPROVAL".equals(out.decision())) {
      auth = "PENDING_HUMAN_APPROVAL";
      blocking = true;
      reward = "TRANSPARENCY_REWARDED";
    } else if ("DENY".equals(out.decision())) {
      auth = "DENIED";
      blocking = true;
      reward = "SWITCHBOARD_DENIED";
    }

    Map<String, Object> parties = new LinkedHashMap<>();
    Map<String, Object> declarer = new LinkedHashMap<>();
    declarer.put("id", actor);
    declarer.put("type", actorType);
    if (sbCtx != null && sbCtx.credibility() != null) {
      declarer.put("credibility", sbCtx.credibility());
      declarer.put("whitelisted", sbCtx.whitelisted());
    }
    parties.put("declarer", declarer);
    parties.put("evaluator", Map.of("id", "tlpx-java", "type", "machine"));
    parties.put("authorizer", null);
    if (sbCtx != null) {
      parties.put("router", Map.of("id", sbCtx.switchboardId(), "type", "machine"));
    }

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
    d.put("reward_signal", reward);
    d.put("authorization_status", auth);
    d.put("parties", parties);
    d.put("approval_route", sbCtx != null ? sbCtx.approvalRoute() : List.of());
    if (sbCtx != null) {
      Map<String, Object> sb = new LinkedHashMap<>();
      sb.put("switchboard_id", sbCtx.switchboardId());
      sb.put("lookup", sbCtx.lookup());
      sb.put("whitelisted", sbCtx.whitelisted());
      sb.put("credibility", sbCtx.credibility());
      sb.put("credibility_band", sbCtx.credibilityBand());
      sb.put("flags", sbCtx.flags());
      sb.put("approval_route", sbCtx.approvalRoute());
      d.put("switchboard", sb);
    } else {
      d.put("switchboard", null);
    }
    d.put("original_intent", intent);
    d.put("implementation", "java-0.2");
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
