import { describe, expect, it } from "vitest";

import { shouldPrefillInlinePromptOnSourceSwitch } from "./mission-automations-dialog";

describe("shouldPrefillInlinePromptOnSourceSwitch", () => {
  it("prefills only for library -> inline when inline prompt is empty", () => {
    expect(shouldPrefillInlinePromptOnSourceSwitch("library", "inline", "")).toBe(true);
    expect(shouldPrefillInlinePromptOnSourceSwitch("library", "inline", "   ")).toBe(true);

    expect(shouldPrefillInlinePromptOnSourceSwitch("inline", "library", "")).toBe(false);
    expect(shouldPrefillInlinePromptOnSourceSwitch("library", "inline", "keep this")).toBe(
      false,
    );
  });

  it("supports repeated back/forth switching without overwrite", () => {
    const firstSwitch = shouldPrefillInlinePromptOnSourceSwitch("library", "inline", "");
    const withExistingText = shouldPrefillInlinePromptOnSourceSwitch(
      "library",
      "inline",
      "My custom inline prompt",
    );
    const afterClearing = shouldPrefillInlinePromptOnSourceSwitch("library", "inline", " ");

    expect(firstSwitch).toBe(true);
    expect(withExistingText).toBe(false);
    expect(afterClearing).toBe(true);
  });
});
