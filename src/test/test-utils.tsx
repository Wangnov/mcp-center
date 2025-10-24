import type { ReactElement } from "react";
import { render } from "@testing-library/react";
import {
  QueryClient,
  QueryClientProvider,
  type QueryClientConfig,
} from "@tanstack/react-query";

export function createQueryClient(config?: QueryClientConfig) {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, cacheTime: 0, refetchOnWindowFocus: false },
      ...config?.defaultOptions,
    },
  });
}

export function renderWithQueryClient(
  ui: ReactElement,
  client: QueryClient = createQueryClient(),
) {
  const Wrapper = ({ children }: { children: ReactElement }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );

  return {
    client,
    ...render(ui, { wrapper: Wrapper }),
  };
}
