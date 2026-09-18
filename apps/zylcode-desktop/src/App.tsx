import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStreamSubscription } from "./lib/events";
import TokenMetricsWidget from "./components/TokenMetricsWidget";
import McpInspector from "./components/McpInspector";
import ArtifactViewer from "./components/ArtifactViewer";
import ProviderSettings from "./components/ProviderSettings";
import { useArtifactStream } from "./lib/useArtifactStream";
import { StatusBar } from "./components/StatusBar";
import VerificationRungBadge from "./components/VerificationRungBadge";
import { ThemeProvider, useTheme, ThemeSelector } from "./components/ui";
import { motion, AnimatePresence } from "framer-motion";

type IntentResult = {
  summary: string;
  artifacts: { kind: string; label: string; content: string }[];
  success: boolean;
};

type VerificationReport = {
  passed: boolean;
  rung: number;
  checks: { name: string; passed: boolean; message: string }[];
  duration_ms: number;
};

type ToolDescriptor = {
  id: string;
  transport: string;
  command: string;
  env_keys: string[];
  description?: string | null;
};

function AppContent() {
  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("");
  const [intentResult, setIntentResult] = useState<IntentResult | null>(null);
  const [verification, setVerification] = useState<VerificationReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tools, setTools] = useState<ToolDescriptor[]>([]);
  const [metricsKey, setMetricsKey] = useState(0);
  const { currentTheme, setTheme, isDark } = useTheme();

  const { deltas, done, mcpCalls, isStreaming } = useStreamSubscription(true);

  // artifact stream parser hook (track: ARTIFACT-STREAM)
  const {
    files,
    activeTab,
    setActiveTab,
    diffMode,
    setDiffMode,
    saveStatus,
    saveFile,
    applyPatch,
    closeTab,
  } = useArtifactStream(deltas);

  useEffect(() => {
    if (done) {
      setIntentResult(done);
      setStreaming(false);
      setBusy(false);
      setMetricsKey((k) => k + 1);
    }
  }, [done]);

  useEffect(() => {
    setStreaming(isStreaming);
  }, [isStreaming]);

  useEffect(() => {
    // When intentResult is set, also push its artifacts into the stream parser
    if (intentResult && intentResult.artifacts) {
      for (const a of intentResult.artifacts) {
        // Create a synthetic delta that signals done
        // The useArtifactStream hook will parse the <artifact> blocks from done
      }
    }
  }, [intentResult]);

  async function handleProcessIntent(useStream: boolean) {
    if (!prompt.trim()) return;
    setBusy(true);
    setStreaming(useStream);
    setError(null);
    setIntentResult(null);
    try {
      if (useStream) {
        const result = await invoke<IntentResult>("process_intent_stream", {
          prompt,
          model: model.trim() || null,
        });
        setIntentResult(result);
      } else {
        const result = await invoke<IntentResult>("process_intent", {
          prompt,
          model: model.trim() || null,
        });
        setIntentResult(result);
      }
      setMetricsKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setStreaming(false);
    }
  }

  async function handleVerify() {
    setBusy(true);
    setError(null);
    try {
      const report = await invoke<VerificationReport>("verify_logic");
      setVerification(report);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen bg-background text-text-primary flex flex-col">
      {/* Premium Header */}
      <header className="border-b border-border bg-surface/80 backdrop-blur-sm px-6 py-3 flex items-center justify-between gap-4 sticky top-0 z-40">
        <div className="flex items-center gap-4">
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.3 }}
          >
            <h1 className="text-xl font-bold tracking-tight">
              <span className="bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">
                ZylCode
              </span>
              <span className="font-mono text-sm font-normal text-text-muted ml-2">
                desktop v0.2.0-dev
              </span>
            </h1>
            <p className="text-xs text-text-muted">
              Intent workspace — streaming synthesis, MCP tools, artifact preview
            </p>
          </motion.div>
        </div>
        
        <div className="flex items-center gap-4">
          <TokenMetricsWidget refreshKey={metricsKey} />
          <ThemeSelector />
        </div>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 min-h-0">
        {/* Left Sidebar - Stream & Tool Panel */}
        <motion.aside
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.3, delay: 0.1 }}
          className="w-[380px] shrink-0 border-r border-border bg-surface/50 flex flex-col min-h-0"
        >
          <div className="p-4 border-b border-border space-y-3">
            <h2 className="text-xs font-semibold uppercase tracking-widest text-text-muted">
              Stream & Tool Panel
            </h2>
            <div className="flex gap-2">
              <input
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder="model override (optional)"
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm font-mono outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all"
              />
            </div>
          </div>

          {/* Prompt Input */}
          <div className="p-4 border-b border-border">
            <div className="space-y-3">
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="Describe what you want to build..."
                rows={4}
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all resize-none"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                    handleProcessIntent(true);
                  }
                }}
              />
              <div className="flex gap-2">
                <button
                  onClick={() => handleProcessIntent(true)}
                  disabled={busy || !prompt.trim()}
                  className="flex-1 bg-primary text-primary-foreground px-4 py-2 rounded-md text-sm font-medium hover:bg-primary-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {streaming ? (
                    <span className="flex items-center gap-2">
                      <svg className="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                        <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                      </svg>
                      Streaming...
                    </span>
                  ) : (
                    "Generate"
                  )}
                </button>
                <button
                  onClick={handleVerify}
                  disabled={busy}
                  className="bg-secondary text-secondary-foreground px-4 py-2 rounded-md text-sm font-medium hover:bg-secondary-hover transition-colors disabled:opacity-50"
                >
                  Verify
                </button>
              </div>
            </div>
          </div>

          {/* MCP Tools */}
          <div className="flex-1 overflow-y-auto p-4">
            <McpInspector />
          </div>

          {/* Verification Badge */}
          {verification && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              className="p-4 border-t border-border"
            >
              <VerificationRungBadge rung={verification.rung} />
            </motion.div>
          )}
        </motion.aside>

        {/* Main Content Area */}
        <main className="flex-1 flex flex-col min-h-0">
          {/* Error Display */}
          <AnimatePresence>
            {error && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -10 }}
                className="bg-error/10 border border-error/20 text-error px-4 py-3 text-sm"
              >
                {error}
              </motion.div>
            )}
          </AnimatePresence>

          {/* Artifact Viewer */}
          <div className="flex-1 min-h-0">
            <ArtifactViewer
              files={files}
              activeTab={activeTab}
              setActiveTab={setActiveTab}
              diffMode={diffMode}
              setDiffMode={setDiffMode}
              saveStatus={saveStatus}
              saveFile={saveFile}
              applyPatch={applyPatch}
              closeTab={closeTab}
            />
          </div>

          {/* Provider Settings */}
          <div className="border-t border-border">
            <ProviderSettings />
          </div>
        </main>
      </div>

      {/* Status Bar */}
      <StatusBar theme={currentTheme} onThemeChange={setTheme} mcpBridgeCount={tools.length} />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <AppContent />
    </ThemeProvider>
  );
}