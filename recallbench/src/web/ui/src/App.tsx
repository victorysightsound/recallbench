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

// Comparison View
const CompareView: Component<{ onBack: () => void }> = (props) => {
  const [runs] = createResource(fetchRuns);
  const [selectedA, setSelectedA] = createSignal("");
  const [selectedB, setSelectedB] = createSignal("");
  const [metricsA] = createResource(() => selectedA() || undefined, (id) => fetchMetrics(id));
  const [metricsB] = createResource(() => selectedB() || undefined, (id) => fetchMetrics(id));

  return (
    <div>
      <div class="breadcrumbs text-sm mb-4">
        <ul><li><a class="cursor-pointer" onClick={props.onBack}>Dashboard</a></li><li>Compare</li></ul>
      </div>
      <h2 class="text-2xl font-bold mb-4">Compare Systems</h2>

      <div class="flex gap-4 mb-6">
        <select class="select select-bordered flex-1" onChange={(e) => setSelectedA(e.target.value)}>
          <option value="">Select System A</option>
          <Show when={runs()}><For each={runs()!}>{(r) => <option value={r.id}>{r.system} ({r.filename})</option>}</For></Show>
        </select>
        <select class="select select-bordered flex-1" onChange={(e) => setSelectedB(e.target.value)}>
          <option value="">Select System B</option>
          <Show when={runs()}><For each={runs()!}>{(r) => <option value={r.id}>{r.system} ({r.filename})</option>}</For></Show>
        </select>
      </div>

      <Show when={metricsA() && metricsB()}>
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead><tr><th>Metric</th><th>{selectedA()}</th><th>{selectedB()}</th></tr></thead>
            <tbody>
              <tr>
                <td class="font-semibold">Task-Averaged</td>
                <td><AccuracyBadge value={metricsA()!.accuracy.task_averaged} /></td>
                <td><AccuracyBadge value={metricsB()!.accuracy.task_averaged} /></td>
              </tr>
              <tr>
                <td class="font-semibold">Overall</td>
                <td><AccuracyBadge value={metricsA()!.accuracy.overall} /></td>
                <td><AccuracyBadge value={metricsB()!.accuracy.overall} /></td>
              </tr>
              <tr>
                <td class="font-semibold">Questions</td>
                <td>{metricsA()!.accuracy.total_questions}</td>
                <td>{metricsB()!.accuracy.total_questions}</td>
              </tr>
              <tr>
                <td class="font-semibold">Retrieval p50</td>
                <td>{metricsA()!.latency.retrieval_p50.toFixed(0)}ms</td>
                <td>{metricsB()!.latency.retrieval_p50.toFixed(0)}ms</td>
              </tr>
              <tr>
                <td class="font-semibold">Est. Cost</td>
                <td>${metricsA()!.cost.estimated_usd.toFixed(2)}</td>
                <td>${metricsB()!.cost.estimated_usd.toFixed(2)}</td>
              </tr>
            </tbody>
          </table>
        </div>

        {/* Per-type comparison */}
        <h3 class="text-lg font-semibold mt-6 mb-3">Per-Type Accuracy</h3>
        <div class="overflow-x-auto">
          <table class="table table-zebra table-sm">
            <thead><tr><th>Type</th><th>{selectedA()}</th><th>{selectedB()}</th><th>Diff</th></tr></thead>
            <tbody>
              <For each={Object.keys({...metricsA()!.accuracy.per_type, ...metricsB()!.accuracy.per_type}).sort()}>
                {(type) => {
                  const a = metricsA()!.accuracy.per_type[type] || 0;
                  const b = metricsB()!.accuracy.per_type[type] || 0;
                  const diff = a - b;
                  const diffCls = diff > 0 ? "text-success" : diff < 0 ? "text-error" : "";
                  return (
                    <tr>
                      <td>{type}</td>
                      <td><AccuracyBadge value={a} /></td>
                      <td><AccuracyBadge value={b} /></td>
                      <td class={diffCls}>{diff > 0 ? "+" : ""}{(diff * 100).toFixed(1)}%</td>
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </div>
  );
};

// Longevity View
const LongevityView: Component<{ onBack: () => void }> = (props) => {
  const [data, setData] = createSignal<any>(null);

  const loadFile = async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const text = await file.text();
    setData(JSON.parse(text));
  };

  const maxAcc = () => data() ? Math.max(...data().checkpoints.map((c: any) => c.accuracy)) : 1;
  const maxLat = () => data() ? Math.max(...data().checkpoints.map((c: any) => c.avg_retrieval_latency_ms)) : 1;

  return (
    <div>
      <div class="breadcrumbs text-sm mb-4">
        <ul><li><a class="cursor-pointer" onClick={props.onBack}>Dashboard</a></li><li>Longevity</li></ul>
      </div>
      <h2 class="text-2xl font-bold mb-4">Longevity Analysis</h2>

      <div class="mb-6">
        <input type="file" accept=".json" class="file-input file-input-bordered" onChange={loadFile} />
        <p class="text-xs text-base-content/50 mt-1">Load a longevity result JSON file from <code>recallbench longevity --output</code></p>
      </div>

      <Show when={data()}>
        <h3 class="text-lg font-semibold mb-2">{data().system_name} — Accuracy Over Time</h3>
        <svg viewBox="0 0 600 200" class="w-full max-w-2xl mb-6 bg-base-200 rounded-box p-2">
          <For each={data().checkpoints}>
            {(cp: any, i) => {
              const x = () => 50 + (i() / (data().checkpoints.length - 1 || 1)) * 500;
              const y = () => 180 - (cp.accuracy / (maxAcc() || 1)) * 160;
              const nextCp = () => data().checkpoints[i() + 1];
              return (
                <>
                  <circle cx={x()} cy={y()} r="4" fill="oklch(76% 0.177 163.223)" />
                  <text x={x()} y={y() - 10} text-anchor="middle" font-size="10" fill="currentColor">
                    {(cp.accuracy * 100).toFixed(0)}%
                  </text>
                  <Show when={nextCp()}>
                    {(next) => {
                      const nx = () => 50 + ((i() + 1) / (data().checkpoints.length - 1 || 1)) * 500;
                      const ny = () => 180 - (next().accuracy / (maxAcc() || 1)) * 160;
                      return <line x1={x()} y1={y()} x2={nx()} y2={ny()} stroke="oklch(76% 0.177 163.223)" stroke-width="2" />;
                    }}
                  </Show>
                  <text x={x()} y={195} text-anchor="middle" font-size="9" fill="currentColor" opacity="0.5">
                    {cp.sessions_ingested}
                  </text>
                </>
              );
            }}
          </For>
          <text x="300" y="12" text-anchor="middle" font-size="11" fill="currentColor">Accuracy vs Sessions Ingested</text>
        </svg>

        <h3 class="text-lg font-semibold mb-2">Retrieval Latency Over Time</h3>
        <svg viewBox="0 0 600 200" class="w-full max-w-2xl mb-6 bg-base-200 rounded-box p-2">
          <For each={data().checkpoints}>
            {(cp: any, i) => {
              const x = () => 50 + (i() / (data().checkpoints.length - 1 || 1)) * 500;
              const y = () => 180 - (cp.avg_retrieval_latency_ms / (maxLat() || 1)) * 160;
              const nextCp = () => data().checkpoints[i() + 1];
              return (
                <>
                  <circle cx={x()} cy={y()} r="4" fill="oklch(74% 0.16 232.661)" />
                  <text x={x()} y={y() - 10} text-anchor="middle" font-size="10" fill="currentColor">
                    {cp.avg_retrieval_latency_ms.toFixed(0)}ms
                  </text>
                  <Show when={nextCp()}>
                    {(next) => {
                      const nx = () => 50 + ((i() + 1) / (data().checkpoints.length - 1 || 1)) * 500;
                      const ny = () => 180 - (next().avg_retrieval_latency_ms / (maxLat() || 1)) * 160;
                      return <line x1={x()} y1={y()} x2={nx()} y2={ny()} stroke="oklch(74% 0.16 232.661)" stroke-width="2" />;
                    }}
                  </Show>
                  <text x={x()} y={195} text-anchor="middle" font-size="9" fill="currentColor" opacity="0.5">
                    {cp.sessions_ingested}
                  </text>
                </>
              );
            }}
          </For>
          <text x="300" y="12" text-anchor="middle" font-size="11" fill="currentColor">Latency (ms) vs Sessions Ingested</text>
        </svg>

        {/* Data table */}
        <div class="overflow-x-auto">
          <table class="table table-zebra table-sm">
            <thead><tr><th>Sessions</th><th>Memories</th><th>Accuracy</th><th>Retrieval (ms)</th><th>Ingest (ms)</th></tr></thead>
            <tbody>
              <For each={data().checkpoints}>
                {(cp: any) => (
                  <tr>
                    <td>{cp.sessions_ingested}</td>
                    <td>{cp.estimated_memories}</td>
                    <td><AccuracyBadge value={cp.accuracy} /></td>
                    <td>{cp.avg_retrieval_latency_ms.toFixed(1)}</td>
                    <td>{cp.avg_ingest_latency_ms.toFixed(1)}</td>
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
  const [view, setView] = createSignal<"dashboard" | "detail" | "compare" | "longevity">("dashboard");
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
          <button class="btn btn-sm btn-ghost" onClick={() => setView("compare")}>Compare</button>
          <button class="btn btn-sm btn-ghost" onClick={() => setView("longevity")}>Longevity</button>
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
        <Show when={view() === "compare"}>
          <CompareView onBack={showDashboard} />
        </Show>
        <Show when={view() === "longevity"}>
          <LongevityView onBack={showDashboard} />
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
