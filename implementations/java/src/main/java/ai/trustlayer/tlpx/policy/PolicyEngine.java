package ai.trustlayer.tlpx.policy;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Minimal first-match policy engine compatible with the JS reference YAML subset. */
public final class PolicyEngine {
  public record Rule(String id, String description, String ifExpr, String require) {}

  public record Policy(String packId, List<Rule> rules) {}

  public record Outcome(String decision, String reason, String policyId) {}

  private static final Pattern RULE = Pattern.compile("^\\s*-\\s+id:\\s+(.+)$");
  private static final Pattern FIELD = Pattern.compile("^\\s+([A-Za-z_]+):\\s+(.+)$");
  private static final Pattern EQ = Pattern.compile("^([A-Za-z_][A-Za-z0-9_]*)\\s*==\\s*\"(.*)\"$");
  private static final Pattern IN = Pattern.compile("^\"([^\"]+)\"\\s+in\\s+([A-Za-z_][A-Za-z0-9_]*)$");

  private PolicyEngine() {}

  public static Policy load(Path path) throws IOException {
    return parse(Files.readString(path));
  }

  public static Policy parse(String text) {
    List<Rule> rules = new ArrayList<>();
    String id = null, desc = null, ifExpr = null, require = null;
    for (String raw : text.split("\n")) {
      String line = raw.stripTrailing();
      String trim = line.trim();
      if (trim.isEmpty() || trim.startsWith("#") || trim.equals("rules:")) continue;
      Matcher rm = RULE.matcher(line);
      if (rm.find()) {
        if (id != null) {
          rules.add(new Rule(id, desc, ifExpr, require));
        }
        id = unquote(rm.group(1).trim());
        desc = null;
        ifExpr = null;
        require = null;
        continue;
      }
      Matcher fm = FIELD.matcher(line);
      if (fm.find() && id != null) {
        switch (fm.group(1)) {
          case "description" -> desc = unquote(fm.group(2).trim());
          case "if" -> ifExpr = unquote(fm.group(2).trim());
          case "require" -> require = unquote(fm.group(2).trim());
          default -> {}
        }
      }
    }
    if (id != null) {
      rules.add(new Rule(id, desc, ifExpr, require));
    }
    return new Policy("default", rules);
  }

  public static Outcome evaluate(Map<String, Object> intent, Policy policy) {
    for (Rule r : policy.rules()) {
      if (r.ifExpr() == null || r.ifExpr().isBlank()) continue;
      if (!evalCond(r.ifExpr(), intent)) continue;
      if (r.require() != null && !r.require().isBlank()) {
        String reason = r.description() != null ? r.description() : "Policy requires " + r.require();
        return new Outcome("REQUIRE_APPROVAL", reason, r.id());
      }
    }
    return new Outcome("ALLOW", "No approval rules matched", null);
  }

  private static boolean evalCond(String expr, Map<String, Object> intent) {
    String[] parts = expr.split("\\s+and\\s+");
    for (String part : parts) {
      part = part.trim();
      Matcher eq = EQ.matcher(part);
      if (eq.find()) {
        Object got = intent.get(eq.group(1));
        if (got == null || !eq.group(2).equals(String.valueOf(got))) return false;
        continue;
      }
      Matcher in = IN.matcher(part);
      if (in.find()) {
        Object arr = intent.get(in.group(2));
        if (!(arr instanceof List<?> list) || !list.contains(in.group(1))) return false;
        continue;
      }
      if (part.contains("==")) {
        String[] bits = part.split("==", 2);
        String field = bits[0].trim();
        String want = bits[1].trim();
        Object val = intent.get(field);
        if ("true".equals(want)) {
          if (!(Boolean.TRUE.equals(val))) return false;
          continue;
        }
        if ("false".equals(want)) {
          if (Boolean.TRUE.equals(val)) return false;
          continue;
        }
      }
      return false;
    }
    return true;
  }

  private static String unquote(String s) {
    if (s.length() >= 2 && s.startsWith("\"") && s.endsWith("\"")) {
      return s.substring(1, s.length() - 1);
    }
    return s;
  }
}
