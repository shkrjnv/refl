// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ApiClient } from "../api/client";
import { I18nProvider } from "../i18n";
import { AdminAccessSection } from "./AdminAccessSection";

afterEach(cleanup);

type AdminApi = Pick<
  ApiClient,
  "createInvite" | "pendingRequests" | "approve" | "reject"
>;

describe("admin invite actions", () => {
  it("shows a checkmark when the invite link is copied", async () => {
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const api: AdminApi = {
      createInvite: async () => ({
        id: "invite-1",
        invite_code: "ABC123",
        max_uses: 1,
        expires_at: null,
      }),
      pendingRequests: async () => [],
      approve: async (userId) => ({ user_id: userId, access_status: "approved" }),
      reject: async (userId) => ({ user_id: userId, access_status: "rejected" }),
    };

    render(
      <I18nProvider initialLocale="en">
        <AdminAccessSection api={api} />
      </I18nProvider>,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Create invite link" }),
    );
    const copy = await screen.findByRole("button", { name: "Copy link" });
    fireEvent.click(copy);

    const copied = await screen.findByRole("button", { name: "Copied" });
    expect(writeText).toHaveBeenCalledWith(
      `${window.location.origin}/?invite=ABC123`,
    );
    const indicator = copied.querySelector(".crossfade-icon");
    expect(indicator?.getAttribute("data-active")).toBe("true");
    expect(
      indicator?.querySelector(
        '.crossfade-icon-state-to path[d="m4.5 12.5 5 5L20 6.8"]',
      ),
    ).not.toBeNull();
    expect(document.querySelector(".pulse-confirm")).toBeNull();
  });
});
