import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { NotificationInbox } from "./NotificationInbox";
import { useSystemMessages } from "@/hooks/useSystemMessages";

vi.mock("@/hooks/useSystemMessages", () => ({
  useSystemMessages: vi.fn(),
}));

vi.mock("lucide-react", () => ({
  Bell: () => <svg />,
  CheckCheck: () => <svg />,
  X: () => <svg />,
}));

const mockUseSystemMessages = vi.mocked(useSystemMessages);

describe("NotificationInbox", () => {
  beforeEach(() => {
    mockUseSystemMessages.mockReset();
  });

  it("shows the empty inbox state", async () => {
    const user = userEvent.setup();
    mockUseSystemMessages.mockReturnValue({
      messages: [],
      unreadCount: 0,
      loading: false,
      error: null,
      markRead: vi.fn(),
      dismiss: vi.fn(),
      dismissAll: vi.fn(),
    });

    render(<NotificationInbox />);
    await user.click(screen.getByRole("button", { name: "Notifications" }));

    expect(screen.getByRole("dialog", { name: "System notifications" })).toBeInTheDocument();
    expect(screen.getByText("No notifications")).toBeInTheDocument();
  });

  it("renders one notification and marks it read when opened", async () => {
    const user = userEvent.setup();
    const markRead = vi.fn();
    mockUseSystemMessages.mockReturnValue({
      messages: [
        {
          id: "maintenance-1",
          title: "Scheduled maintenance",
          body: "Quotes may refresh more slowly.",
          severity: "maintenance",
          created_at: "2026-08-25T00:00:00.000Z",
        },
      ],
      unreadCount: 1,
      loading: false,
      error: null,
      markRead,
      dismiss: vi.fn(),
      dismissAll: vi.fn(),
    });

    render(<NotificationInbox />);
    await user.click(
      screen.getByRole("button", { name: "Notifications, 1 unread" }),
    );

    expect(screen.getByText("Scheduled maintenance")).toBeInTheDocument();
    expect(screen.getByText("Quotes may refresh more slowly.")).toBeInTheDocument();
    expect(screen.getByText("maintenance")).toBeInTheDocument();
    expect(markRead).toHaveBeenCalledWith("maintenance-1");
  });
});
