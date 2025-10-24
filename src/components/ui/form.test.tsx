import { useEffect } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useForm } from "react-hook-form";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "./form";

describe("ui/form helpers", () => {
  it("renders description without message while field is valid", () => {
    function ValidForm() {
      const form = useForm<{ name: string }>({ defaultValues: { name: "" } });
      return (
        <Form {...form}>
          <form>
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Label</FormLabel>
                  <FormControl>
                    <input aria-label="name" {...field} />
                  </FormControl>
                  <FormDescription>Description</FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />
          </form>
        </Form>
      );
    }

    render(<ValidForm />);
    expect(screen.getByText("Description")).toBeInTheDocument();
    expect(screen.queryByText("Required")).not.toBeInTheDocument();
  });

  it("shows validation message when field has error", async () => {
    function InvalidForm() {
      const form = useForm<{ name: string }>({ defaultValues: { name: "" } });

      useEffect(() => {
        form.setError("name", { message: "Required" });
      }, [form]);

      return (
        <Form {...form}>
          <form>
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Label</FormLabel>
                  <FormControl>
                    <input aria-label="name" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
          </form>
        </Form>
      );
    }

    render(<InvalidForm />);
    expect(await screen.findByText("Required")).toBeInTheDocument();
  });
});
