import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { ServersPage } from "@/pages/Servers";
import { ProjectsPage } from "@/pages/Projects";
import { SettingsPage } from "@/pages/Settings";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";

function App() {
  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <TooltipProvider>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<Layout />}>
              <Route index element={<Navigate to="/mcp" replace />} />
              <Route path="mcp" element={<ServersPage />} />
              <Route path="project" element={<ProjectsPage />} />
              <Route path="settings" element={<SettingsPage />} />
            </Route>
          </Routes>
          <Toaster />
        </BrowserRouter>
      </TooltipProvider>
    </ThemeProvider>
  );
}

export default App;
