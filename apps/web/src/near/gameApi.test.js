import { describe, expect, it } from "vitest";
import {
  actionAmountAtomic,
  actionReady,
  buildLivingPaymentMessage,
  formatNuve,
} from "./gameApi";

describe("NUVI Hackathon gameplay client", () => {
  it("uses the on-chain demo economy atomic amounts", () => {
    expect(actionAmountAtomic("feed")).toBe("50000");
    expect(actionAmountAtomic("work_a")).toBe("100000");
    expect(actionAmountAtomic("work_b")).toBe("100000");
  });

  it("builds the exact Living NFT ft_transfer_call message", () => {
    expect(JSON.parse(buildLivingPaymentMessage("7", "work_a", "burn"))).toEqual({
      token_id: "7",
      action: "work_a",
      route: "burn",
    });
  });

  it("formats four-decimal NUVE without floating point", () => {
    expect(formatNuve("1000000")).toBe("100.0000");
    expect(formatNuve("50001")).toBe("5.0001");
    expect(formatNuve("0")).toBe("0.0000");
  });

  it("detects action readiness from nanosecond timestamps", () => {
    expect(
      actionReady(
        { active_action: "feed", ready_at_ns: "2000000000" },
        2_000,
      ),
    ).toBe(true);
    expect(
      actionReady(
        { active_action: "feed", ready_at_ns: "2001000000" },
        2_000,
      ),
    ).toBe(false);
  });
});
