import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, expect, describe, it } from "vitest";
import {
  useKeyboardShortcuts,
  getModifierSymbol,
} from "./use-keyboard-shortcuts";

function TestComponent({
  shortcut,
}: {
  shortcut: Parameters<typeof useKeyboardShortcuts>[0][number];
}) {
  useKeyboardShortcuts([shortcut]);
  return null;
}

describe("useKeyboardShortcuts", () => {
  it("invokes the callback when the shortcut matches", async () => {
    const callback = vi.fn();
    const user = userEvent.setup();

    render(<TestComponent shortcut={{ key: "k", ctrl: true, callback }} />);

    await user.keyboard("{Control>}k{/Control}");

    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("ignores non matching key combinations", async () => {
    const callback = vi.fn();
    const user = userEvent.setup();

    render(<TestComponent shortcut={{ key: "k", ctrl: true, callback }} />);

    await user.keyboard("k");

    expect(callback).not.toHaveBeenCalled();
  });

  it("cleans up the event listener on unmount", () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    const removeSpy = vi.spyOn(window, "removeEventListener");
    const shortcut = { key: "a", callback: vi.fn() };

    const { unmount } = render(<TestComponent shortcut={shortcut} />);

    expect(addSpy).toHaveBeenCalledWith("keydown", expect.any(Function));

    unmount();

    expect(
      removeSpy.mock.calls.some(
        ([eventName]) => eventName === "keydown",
      ),
    ).toBe(true);

    addSpy.mockRestore();
   removeSpy.mockRestore();
  });

  it("honours meta, alt and shift modifier requirements", () => {
    const callback = vi.fn();

    render(
      <TestComponent
        shortcut={{
          key: "k",
          meta: true,
          alt: false,
          shift: true,
          callback,
        }}
      />,
    );

    const event = new KeyboardEvent("keydown", {
      key: "k",
      metaKey: true,
      altKey: false,
      shiftKey: true,
    });

    window.dispatchEvent(event);
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it("uses mac symbol when platform is macOS", () => {
    const original = navigator.platform;
    Object.defineProperty(navigator, "platform", {
      configurable: true,
      value: "MacIntel",
    });

    expect(getModifierSymbol()).toBe("⌘");

    Object.defineProperty(navigator, "platform", {
      configurable: true,
      value: original,
    });
    expect(getModifierSymbol()).toBe("Ctrl");
  });
});
