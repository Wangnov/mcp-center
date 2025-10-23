import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import * as z from "zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { addMcpServer, AddServerPayload } from "@/lib/api";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useState } from "react";

const formSchema = z
  .object({
    name: z.string().min(1, { message: "Server name is required." }),
    protocol: z.enum(["stdio", "sse", "http"]),
    command: z.string().optional(),
    endpoint: z
      .string()
      .url({ message: "Please enter a valid URL." })
      .optional(),
  })
  .refine(
    (data) => {
      if (data.protocol === "stdio") {
        return !!data.command && data.command.length > 0;
      }
      return true;
    },
    { message: "Command is required for stdio protocol.", path: ["command"] },
  )
  .refine(
    (data) => {
      if (data.protocol === "sse" || data.protocol === "http") {
        return !!data.endpoint && data.endpoint.length > 0;
      }
      return true;
    },
    {
      message: "Endpoint URL is required for sse/http protocols.",
      path: ["endpoint"],
    },
  );

export function AddServerDialog({
  children,
  open: externalOpen,
  onOpenChange: externalOnOpenChange,
}: {
  children: React.ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [internalOpen, setInternalOpen] = useState(false);

  // 使用外部状态或内部状态
  const open = externalOpen !== undefined ? externalOpen : internalOpen;
  const setOpen = externalOnOpenChange || setInternalOpen;

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: "",
      protocol: "stdio",
      command: "",
      endpoint: "",
    },
  });

  const protocol = form.watch("protocol");

  const { mutate: createServer, isPending } = useMutation({
    mutationFn: (values: AddServerPayload) => addMcpServer(values),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      setOpen(false); // Close dialog on success
      form.reset();
      toast.success(t("server_added_success") || "服务器添加成功");
    },
    onError: (error) => {
      console.error("Failed to add server:", error);
      toast.error(t("server_added_error") || "添加服务器失败", {
        description: error.message,
      });
    },
  });

  function onSubmit(values: z.infer<typeof formSchema>) {
    const payload: AddServerPayload = { ...values };
    // The backend CLI splits the command and args, so we can pass it as a single string.
    if (payload.protocol === "stdio" && payload.command) {
      const parts = payload.command.split(/\s+/);
      payload.command = parts.shift() || "";
      payload.args = parts.join(" ");
    }
    createServer(payload);
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <Form {...form}>
        <form onSubmit={form.handleSubmit(onSubmit)}>
          <DialogTrigger asChild>{children}</DialogTrigger>
          <DialogContent className="sm:max-w-[425px]">
            <DialogHeader>
              <DialogTitle>{t("add_server")}</DialogTitle>
              <DialogDescription>{t("add_server_desc")}</DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 py-4">
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t("server_name_label")}</FormLabel>
                    <FormControl>
                      <Input placeholder="My Awesome Server" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="protocol"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{t("protocol_label")}</FormLabel>
                    <Select
                      onValueChange={field.onChange}
                      defaultValue={field.value}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Select a protocol" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="stdio">stdio</SelectItem>
                        <SelectItem value="sse">sse</SelectItem>
                        <SelectItem value="http">http</SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              {protocol === "stdio" && (
                <FormField
                  control={form.control}
                  name="command"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>{t("command_label")}</FormLabel>
                      <FormControl>
                        <Input placeholder="npx -y my-mcp-server" {...field} />
                      </FormControl>
                      <FormDescription>{t("command_desc")}</FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}
              {(protocol === "sse" || protocol === "http") && (
                <FormField
                  control={form.control}
                  name="endpoint"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>{t("endpoint_label")}</FormLabel>
                      <FormControl>
                        <Input
                          placeholder="https://example.com/mcp"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  {t("cancel")}
                </Button>
              </DialogClose>
              <Button type="submit" disabled={isPending}>
                {isPending ? t("saving") : t("save_server")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </form>
      </Form>
    </Dialog>
  );
}
