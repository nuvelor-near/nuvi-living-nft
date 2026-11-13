import { describe, expect, it } from "vitest";
import {
  actionAmountAtomic,
  actionReady,
  buildLivingPaymentMessage,
  formatNuve,
  parseNuve,
  parseTokenExtra,
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



  it("parses four-decimal NUVE to atomic units", () => {
    expect(parseNuve("1")).toBe("10000");
    expect(parseNuve("1.0001")).toBe("10001");
    expect(() => parseNuve("0")).toThrow(/positive/);
    expect(() => parseNuve("1.00001")).toThrow(/4 decimal/);
  });

  it("reads NUVI Account proof fields from token metadata extra", () => {
    expect(
      parseTokenExtra({
        metadata: {
          extra: JSON.stringify({
            nuvi_account_status: "active",
            nuvi_account_id: "nuvi-7.demo.testnet",
          }),
        },
      }),
    ).toEqual({
      nuvi_account_status: "active",
      nuvi_account_id: "nuvi-7.demo.testnet",
    });
    expect(parseTokenExtra({ metadata: { extra: "not-json" } })).toEqual({});
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
