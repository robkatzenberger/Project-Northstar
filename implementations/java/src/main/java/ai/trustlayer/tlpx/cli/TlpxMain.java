package ai.trustlayer.tlpx.cli;

import ai.trustlayer.tlpx.gate.Gate;
import ai.trustlayer.tlpx.policy.PolicyEngine;
import ai.trustlayer.tlpx.policy.PolicyEngine.Policy;
import ai.trustlayer.tlpx.switchboard.Switchboard;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class TlpxMain {
  private static final ObjectMapper M = new ObjectMapper().enable(SerializationFeature.INDENT_OUTPUT);

  public static void main(String[] args) throws Exception {
    if (args.length == 0 || args[0].equals("help") || args[0].equals("--help")) {
      System.out.println("""
          tlpx (Java)

            evaluate <intent.json> <policy.yaml> [switchboard.json]

          Switchboard-first DENY when switchboard.json is provided.
          """);
      return;
    }
    if (!args[0].equals("evaluate") || args.length < 3 || args.length > 4) {
      System.err.println("usage: tlpx evaluate <intent.json> <policy.yaml> [switchboard.json]");
      System.exit(1);
    }
    @SuppressWarnings("unchecked")
    Map<String, Object> raw = M.readValue(Path.of(args[1]).toFile(), Map.class);
    Map<String, Object> intent = normalize(raw);
    Policy policy = PolicyEngine.load(Path.of(args[2]));
    Switchboard sb = null;
    if (args.length == 4) {
      sb = Switchboard.load(Path.of(args[3]));
    }
    Map<String, Object> decision = Gate.evaluateIntent(intent, policy, sb);
    M.writeValue(System.out, decision);
    System.out.println();
  }

  @SuppressWarnings("unchecked")
  static Map<String, Object> normalize(Map<String, Object> flat) {
    Map<String, Object> intent = new LinkedHashMap<>();
    if (flat.get("agent") instanceof String a) intent.put("actor", a);
    if (flat.get("actor") instanceof String a) intent.put("actor", a);
    if (flat.get("intent_summary") instanceof String s) {
      intent.put("declared_intent", s);
      intent.put("intent_summary", s);
    }
    if (flat.get("declared_intent") instanceof String s) intent.put("declared_intent", s);
    intent.put("actor_type", flat.getOrDefault("actor_type", "machine"));
    for (String k : List.of("action", "target", "risk")) {
      if (flat.containsKey(k)) intent.put(k, flat.get(k));
    }
    if (flat.get("data_classes") instanceof List<?> list) {
      List<String> ss = new ArrayList<>();
      for (Object x : list) if (x instanceof String s) ss.add(s);
      intent.put("data_classes", ss);
    } else {
      intent.put("data_classes", List.of());
    }
    if (flat.get("intent_id") instanceof String id) intent.put("intent_id", id);
    else if (flat.get("prism_id") instanceof String id) intent.put("intent_id", id);
    else intent.put("intent_id", "java_" + ProcessHandle.current().pid());
    if (!(intent.get("risk") instanceof String) || ((String) intent.get("risk")).isBlank()) {
      intent.put("risk", "low");
    }
    return intent;
  }
}
