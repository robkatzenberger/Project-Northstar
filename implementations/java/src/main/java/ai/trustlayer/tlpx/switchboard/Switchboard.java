package ai.trustlayer.tlpx.switchboard;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

/** Switchboard-first identity router (JS parity for hard gates). */
public final class Switchboard {
  public record Principal(
      String id,
      String type,
      boolean whitelisted,
      double credibility,
      List<String> allowedActions,
      List<String> approvalRoute
  ) {}

  public record Thresholds(double forceEscalateBelow, double highTrustAtOrAbove, double maxCredibility) {}

  public record Operators(boolean enforce, List<String> allowlist) {}

  public record Gate(String decision, String reason, String policyId) {}

  public record Context(
      String switchboardId,
      String lookup,
      String principalId,
      boolean whitelisted,
      Double credibility,
      String credibilityBand,
      List<String> approvalRoute,
      List<String> flags,
      String actorTypeNormalized,
      Gate gate
  ) {}

  private final String switchboardId;
  private final String unknownPolicy;
  private final Thresholds thresholds;
  private final List<String> defaultRoute;
  private final Operators operators;
  private final Map<String, Principal> byId = new HashMap<>();

  private Switchboard(
      String switchboardId,
      String unknownPolicy,
      Thresholds thresholds,
      List<String> defaultRoute,
      Operators operators,
      List<Principal> principals
  ) {
    this.switchboardId = switchboardId;
    this.unknownPolicy = unknownPolicy;
    this.thresholds = thresholds;
    this.defaultRoute = defaultRoute;
    this.operators = operators;
    for (Principal p : principals) {
      byId.put(p.id(), p);
    }
  }

  public Operators operators() {
    return operators;
  }

  public static Switchboard load(Path path) throws IOException {
    ObjectMapper m = new ObjectMapper();
    JsonNode root = m.readTree(path.toFile());
    String id = text(root, "switchboard_id", "switchboard");
    String unk = text(root, "unknown_agent_policy", "DENY");
    JsonNode th = root.path("thresholds");
    Thresholds thresholds = new Thresholds(
        th.path("force_escalate_below").asDouble(0.4),
        th.path("high_trust_at_or_above").asDouble(0.85),
        th.path("max_credibility").asDouble(0.99)
    );
    List<String> defaults = stringList(root.path("defaults").path("approval_route"));
    Operators ops = null;
    if (root.has("operators")) {
      JsonNode o = root.get("operators");
      ops = new Operators(o.path("enforce").asBoolean(true), stringList(o.path("allowlist")));
    }
    List<Principal> principals = new ArrayList<>();
    for (JsonNode p : root.path("principals")) {
      principals.add(new Principal(
          p.path("id").asText(),
          p.path("type").asText("machine"),
          p.path("whitelisted").asBoolean(false),
          p.path("credibility").asDouble(0),
          stringList(p.path("allowed_actions")),
          stringList(p.path("approval_route"))
      ));
    }
    return new Switchboard(id, unk, thresholds, defaults, ops, principals);
  }

  public Context route(String actorId, String action, String actorType) {
    List<String> flags = new ArrayList<>();
    List<String> route = new ArrayList<>(defaultRoute);
    Principal p = byId.get(actorId);
    if (p == null) {
      flags.add("UNKNOWN_PRINCIPAL");
      Gate g = "REQUIRE_APPROVAL".equals(unknownPolicy)
          ? new Gate("REQUIRE_APPROVAL", "Switchboard: unknown principal requires human approval", "switchboard.unknown_escalate")
          : new Gate("DENY", "Switchboard: principal not registered", "switchboard.unknown_deny");
      return new Context(switchboardId, "unknown", actorId, false, null, "unknown", route, flags, actorType, g);
    }
    if (p.approvalRoute() != null && !p.approvalRoute().isEmpty()) {
      route = new ArrayList<>(p.approvalRoute());
    }
    String band = band(p.credibility());
    String normType = "human".equals(p.type()) ? "human" : "machine";
    if (!p.whitelisted()) {
      flags.add("NOT_WHITELISTED");
      return new Context(switchboardId, "known", p.id(), false, p.credibility(), band, route, flags, normType,
          new Gate("DENY", "Switchboard: principal not on whitelist", "switchboard.not_whitelisted"));
    }
    if (p.allowedActions() != null && action != null && !action.isBlank()
        && !p.allowedActions().contains(action)) {
      flags.add("ACTION_NOT_PERMITTED");
      return new Context(switchboardId, "known", p.id(), true, p.credibility(), band, route, flags, normType,
          new Gate("DENY", "Switchboard: action '" + action + "' not permitted for " + p.id(),
              "switchboard.action_denied"));
    }
    if (p.credibility() < thresholds.forceEscalateBelow()) flags.add("LOW_CREDIBILITY");
    if (p.credibility() >= thresholds.highTrustAtOrAbove()) flags.add("HIGH_TRUST");
    return new Context(switchboardId, "known", p.id(), true, p.credibility(), band, route, flags, normType, null);
  }

  private String band(double score) {
    if (score < thresholds.forceEscalateBelow()) return "low";
    if (score >= thresholds.highTrustAtOrAbove()) return "high";
    return "medium";
  }

  private static String text(JsonNode n, String field, String def) {
    JsonNode v = n.get(field);
    return v == null || v.isNull() ? def : v.asText(def);
  }

  private static List<String> stringList(JsonNode n) {
    List<String> out = new ArrayList<>();
    if (n != null && n.isArray()) {
      for (Iterator<JsonNode> it = n.elements(); it.hasNext(); ) {
        out.add(it.next().asText());
      }
    }
    return out;
  }
}
