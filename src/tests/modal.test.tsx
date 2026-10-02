// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";

import { Modal } from "../shared/Modal";

function Harness() {
  const [open, setOpen] = useState(false);

  return (
    <>
      <button onClick={() => setOpen(true)}>Open settings</button>
      {open && (
        <Modal title="Pengaturan" onClose={() => setOpen(false)}>
          <button>Inside action</button>
        </Modal>
      )}
    </>
  );
}

beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: vi.fn(),
  });
  Object.defineProperty(HTMLDialogElement.prototype, "close", {
    configurable: true,
    value: vi.fn(),
  });
});

describe("modal accessibility", () => {
  it("uses the visible heading as its accessible name and restores trigger focus", () => {
    render(<Harness />);

    const trigger = screen.getByRole("button", { name: "Open settings" });
    trigger.focus();
    fireEvent.click(trigger);

    expect(screen.getByRole("dialog", { name: "Pengaturan" })).not.toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Close / Tutup" }));
    expect(document.activeElement).toBe(trigger);
  });
});
