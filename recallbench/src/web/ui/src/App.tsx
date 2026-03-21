import { createSignal, createResource, For, Show, Component } from "solid-js";

interface RunSummary {
  id: string;
  filename: string;
  system: string | null;
  total_questions: number;
  accuracy: number;
}

interface Metrics {
  accuracy: {
    task_averaged: number;
    overall: number;
    per_type: Record<string, number>;
    abstention: number | null;
    total_questions: number;
    total_correct: number;
  };
  latency: { retrieval_p50: number; total_p50: number };
  cost: { tokens_in: number; tokens_out: number; estimated_usd: number };
}

interface Question {
  question_id: string;
  question_type: string;
  is_correct: boolean;
  ground_truth: string;
  hypothesis: string;
  is_abstention: boolean;
}

// Theme management
function setTheme(theme: string) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("recallbench-theme", theme);
}

const savedTheme = localStorage.getItem("recallbench-theme") || "midnight";
document.documentElement.setAttribute("data-theme", savedTheme);

const THEMES = ["midnight", "dark", "light", "cyber", "onyx", "slate", "frost"];

// API fetchers
const fetchRuns = async (): Promise<RunSummary[]> => {
  const resp = await fetch("/api/runs");
  return resp.json();
};

const fetchMetrics = async (id: string): Promise<Metrics> => {
  const resp = await fetch(`/api/runs/${id}/metrics`);
  return resp.json();
};

const fetchQuestions = async (id: string): Promise<Question[]> => {
  const resp = await fetch(`/api/runs/${id}/questions`);
  return resp.json();
};

// Components

const ThemeSwitcher: Component = () => {
  const [theme, setThemeSignal] = createSignal(savedTheme);

  const onChange = (e: Event) => {
    const val = (e.target as HTMLSelectElement).value;
    setThemeSignal(val);
    setTheme(val);
  };

  return (
    <select class="select select-sm select-bordered w-28" value={theme()} onChange={onChange}>
      <For each={THEMES}>
        {(t) => <option value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>}
      </For>
    </select>
  );
};

const AccuracyBadge: Component<{ value: number; size?: string }> = (props) => {
  const cls = () =>
    props.value >= 0.9 ? "badge-success" : props.value >= 0.7 ? "badge-warning" : "badge-error";
  return (
    <span class={`badge ${cls()} ${props.size || "badge-sm"}`}>
      {(props.value * 100).toFixed(1)}%
    </span>
  );
};

const RunCard: Component<{ run: RunSummary; onClick: () => void }> = (props) => {
  const accClass = () =>
    props.run.accuracy >= 0.9 ? "text-success" : props.run.accuracy >= 0.7 ? "text-warning" : "text-error";

  return (
    <div
      class="card bg-base-200 shadow-md hover:shadow-lg hover:border-primary border border-base-300 cursor-pointer transition-all"
      onClick={props.onClick}
    >
      <div class="card-body p-4">
        <h3 class="card-title text-base">{props.run.system || "Unknown"}</h3>
        <p class={`text-3xl font-bold ${accClass()}`}>{(props.run.accuracy * 100).toFixed(1)}%</p>
        <p class="text-xs text-base-content/50">
          {props.run.total_questions} questions &middot; {props.run.filename}
        </p>
      </div>
    </div>
  );
};

const StatCard: Component<{ label: string; value: string; class?: string }> = (props) => (
  <div class="stat bg-base-200 rounded-box p-3">
    <div class="stat-title text-xs">{props.label}</div>
    <div class={`stat-value text-lg ${props.class || ""}`}>{props.value}</div>
  </div>
);

const Dashboard: Component<{ onSelectRun: (id: string) => void }> = (props) => {
  const [runs] = createResource(fetchRuns);

  return (
    <div>
      <h2 class="text-2xl font-bold mb-4">Benchmark Runs</h2>
      <Show when={!runs.loading} fallback={<div class="skeleton h-32 w-full" />}>
        <Show
          when={runs()?.length}
          fallback={
            <div class="card bg-base-200">
              <div class="card-body items-center text-center py-12">
                <p class="text-base-content/50">No benchmark runs found.</p>
                <p class="text-sm text-base-content/30">
                  Run <kbd class="kbd kbd-sm">recallbench run</kbd> to generate results.
                </p>
              </div>
            </div>
          }
        >
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <For each={runs()}>
              {(run) => <RunCard run={run} onClick={() => props.onSelectRun(run.id)} />}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
};

const RunDetail: Component<{ runId: string; onBack: () => void }> = (props) => {
  const [metrics] = createResource(() => props.runId, fetchMetrics);
  const [questions] = createResource(() => props.runId, fetchQuestions);
  const [failOnly, setFailOnly] = createSignal(false);
  const [typeFilter, setTypeFilter] = createSignal("");

  const filteredQuestions = () => {
    let qs = questions() || [];
    if (failOnly()) qs = qs.filter((q) => !q.is_correct);
    if (typeFilter()) qs = qs.filter((q) => q.question_type === typeFilter());
    return qs;
  };

  const questionTypes = () => {
    const types = new Set((questions() || []).map((q) => q.question_type));
    return [...types].sort();
  };

  const accClass = () => {
    const overall = metrics()?.accuracy?.overall || 0;
    return overall >= 0.9 ? "text-success" : overall >= 0.7 ? "text-warning" : "text-error";
  };

  return (
    <div>
      {/* Breadcrumb */}
      <div class="breadcrumbs text-sm mb-4">
        <ul>
          <li>
            <a class="cursor-pointer" onClick={props.onBack}>Dashboard</a>
          </li>
          <li>{props.runId}</li>
        </ul>
      </div>

      <h2 class="text-2xl font-bold mb-4">{props.runId}</h2>

      <Show when={metrics()}>
        {(m) => (
          <>
            {/* Stats Cards */}
            <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 mb-6">
              <StatCard label="Task-Averaged" value={`${(m().accuracy.task_averaged * 100).toFixed(1)}%`} class={accClass()} />
              <StatCard label="Overall" value={`${(m().accuracy.overall * 100).toFixed(1)}%`} class={accClass()} />
              <StatCard label="Questions" value={`${m().accuracy.total_questions}`} />
              <StatCard label="Correct" value={`${m().accuracy.total_correct}`} class="text-success" />
              <StatCard label="Retrieval p50" value={`${m().latency.retrieval_p50.toFixed(0)}ms`} />
              <StatCard label="Est. Cost" value={`$${m().cost.estimated_usd.toFixed(2)}`} />
            </div>

            {/* Per-Type Table */}
            <Show when={Object.keys(m().accuracy.per_type).length > 0}>
              <div class="overflow-x-auto mb-6">
                <table class="table table-sm table-zebra">
                  <thead>
                    <tr><th>Question Type</th><th>Accuracy</th></tr>
                  </thead>
                  <tbody>
                    <For each={Object.entries(m().accuracy.per_type).sort((a, b) => a[0].localeCompare(b[0]))}>
                      {([type, acc]) => (
                        <tr>
                          <td>{type}</td>
                          <td><AccuracyBadge value={acc} /></td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </Show>
          </>
        )}
      </Show>

      {/* Questions */}
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-lg font-semibold">Questions</h3>
        <div class="flex gap-3 items-center">
          <label class="label cursor-pointer gap-2">
            <span class="label-text text-sm">Failures only</span>
            <input
              type="checkbox"
              class="toggle toggle-sm toggle-error"
              checked={failOnly()}
              onChange={(e) => setFailOnly(e.target.checked)}
            />
          </label>
          <select
            class="select select-sm select-bordered"
            value={typeFilter()}
            onChange={(e) => setTypeFilter(e.target.value)}
          >
            <option value="">All types</option>
            <For each={questionTypes()}>
              {(t) => <option value={t}>{t}</option>}
            </For>
          </select>
        </div>
      </div>

      <Show when={!questions.loading} fallback={<div class="skeleton h-48 w-full" />}>
        <div class="overflow-x-auto">
          <table class="table table-zebra table-sm">
            <thead>
              <tr>
                <th>ID</th>
                <th>Type</th>
                <th>Correct</th>
                <th>Ground Truth</th>
                <th>Hypothesis</th>
              </tr>
            </thead>
            <tbody>
              <For each={filteredQuestions()}>
                {(q) => (
                  <tr>
                    <td class="font-mono text-xs">{q.question_id}</td>
                    <td><span class="badge badge-ghost badge-sm">{q.question_type}</span></td>
                    <td>
                      {q.is_correct
                        ? <span class="badge badge-success badge-sm">Pass</span>
                        : <span class="badge badge-error badge-sm">Fail</span>}
                    </td>
                    <td class="max-w-xs truncate">{q.ground_truth}</td>
                    <td class="max-w-xs truncate">{q.hypothesis}</td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </div>
  );
};

// Main App
const App: Component = () => {
  const [view, setView] = createSignal<"dashboard" | "detail">("dashboard");
  const [selectedRun, setSelectedRun] = createSignal("");

  const showRun = (id: string) => {
    setSelectedRun(id);
    setView("detail");
  };

  const showDashboard = () => {
    setView("dashboard");
    setSelectedRun("");
  };

  return (
    <div class="bg-base-100 text-base-content min-h-screen flex flex-col">
      {/* Navbar */}
      <div class="navbar bg-base-200 border-b border-base-300 px-4">
        <div class="navbar-start">
          <span class="text-primary font-bold text-lg cursor-pointer" onClick={showDashboard}>
            RecallBench
          </span>
          <span class="text-base-content/50 text-sm ml-2">Universal AI Memory Benchmark</span>
        </div>
        <div class="navbar-end gap-2">
          <button class="btn btn-sm btn-ghost" onClick={showDashboard}>Dashboard</button>
          <ThemeSwitcher />
        </div>
      </div>

      {/* Content */}
      <main class="flex-1 p-6 max-w-7xl mx-auto w-full">
        <Show when={view() === "dashboard"}>
          <Dashboard onSelectRun={showRun} />
        </Show>
        <Show when={view() === "detail"}>
          <RunDetail runId={selectedRun()} onBack={showDashboard} />
        </Show>
      </main>

      {/* Footer */}
      <footer class="footer footer-center p-3 bg-base-200 text-base-content/50 text-xs border-t border-base-300">
        <p>RecallBench — Universal AI Memory System Benchmark</p>
      </footer>
    </div>
  );
};

export default App;
