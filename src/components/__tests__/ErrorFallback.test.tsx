import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";
import { ErrorFallback } from "../ErrorFallback";

describe("ErrorFallback", () => {
  it("renders default message and error details", () => {
    const testError = new Error("Boom!");

    render(
      <ErrorFallback error={testError} resetErrorBoundary={vi.fn()} />,
    );

    expect(screen.getByText("应用运行出现异常")).toBeInTheDocument();
    expect(
      screen.getByText("我们已记录错误，你可以重试或刷新页面。"),
    ).toBeInTheDocument();
    expect(screen.getByText("Boom!")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "重试" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "刷新页面" })).toBeInTheDocument();
  });

  it("invokes resetErrorBoundary when retry is clicked", async () => {
    const reset = vi.fn();
    const user = userEvent.setup();

    render(<ErrorFallback error={new Error("Oops")} resetErrorBoundary={reset} />);

    await user.click(screen.getByRole("button", { name: "重试" }));

    expect(reset).toHaveBeenCalledTimes(1);
  });

  it("hides error details when message is empty", () => {
    render(
      <ErrorFallback error={new Error("")} resetErrorBoundary={vi.fn()} />,
    );

    expect(document.querySelector("pre")).toBeNull();
  });

  it("reloads the page when refresh button is clicked", async () => {
    const reloadSpy = vi.spyOn(window.location, "reload").mockImplementation(
      () => {},
    );
    const user = userEvent.setup();

    render(<ErrorFallback error={new Error("Refresh")} resetErrorBoundary={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "刷新页面" }));

    expect(reloadSpy).toHaveBeenCalledTimes(1);
    reloadSpy.mockRestore();
  });
});
