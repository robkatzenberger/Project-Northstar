package ai.trustlayer.tlpx.switchboard;

import ai.trustlayer.tlpx.switchboard.Switchboard.Context;
import org.junit.jupiter.api.Test;

import java.nio.file.Path;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

class SwitchboardTest {
  private static Path sbPath() {
    return Path.of("").toAbsolutePath().resolve("../javascript/config/switchboard.json").normalize();
  }

  @Test
  void unknownDeny() throws Exception {
    Switchboard sb = Switchboard.load(sbPath());
    Context ctx = sb.route("agent.rogue.unregistered", "summarize_report", "machine");
    assertNotNull(ctx.gate());
    assertEquals("DENY", ctx.gate().decision());
  }

  @Test
  void highTrustNoGate() throws Exception {
    Switchboard sb = Switchboard.load(sbPath());
    Context ctx = sb.route("agent.docs.summarizer", "summarize_report", "machine");
    assertNull(ctx.gate());
    assertEquals("high", ctx.credibilityBand());
  }
}
